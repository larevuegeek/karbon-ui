//! Ressources GPU gardées d'une image à l'autre : textures des images (par `Arc`, avec
//! mipmaps), rastérisations Core Graphics des icônes, SVG et polygones (par contenu et
//! position sous-pixel), textures des groupes et des filtres (par taille arrondie et
//! format), résultats des groupes filtrés (par signature de leur contenu, comme
//! `graphics/cache.rs`) et atlas de glyphes.
//!
//! Bornes (`end`) : une image ou une rastérisation inutilisée depuis `IDLE_FRAMES`
//! images est libérée, et toutes celles que l'image courante n'a pas employées dès que
//! leur total dépasse leur budget ; une cible en réserve inutilisée depuis
//! `POOL_FRAMES` images est libérée, la réserve est bornée en nombre et en octets ; un
//! groupe filtré absent de l'image est oublié ; l'atlas est vidé au-delà de huit pages.
//! `trim` libère davantage au repos (réserve, entrées que la dernière image n'a pas
//! employées) et tout quand la fenêtre est masquée.
use super::{
    atlas::Atlas,
    device::{Gpu, Texture, replace},
};
use crate::ui::{
    element::ElementId,
    scene::{DrawCommand, Image, signature::Signature},
};
use objc2_metal::{MTLPixelFormat, MTLResource};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Images (textures des images, rastérisations) inutilisées depuis ce nombre d'images :
/// libérées.
pub(super) const IDLE_FRAMES: u64 = 60;
/// Budget des textures d'images ; au-delà, celles que l'image courante n'emploie pas
/// sont libérées.
const IMAGE_BYTES: usize = 64 << 20;
/// Budget des rastérisations.
const RASTER_BYTES: usize = 32 << 20;
/// Cibles en réserve inutilisées depuis ce nombre d'images : libérées.
pub(super) const POOL_FRAMES: u64 = 30;
/// Réserve des cibles libres : au plus huit textures et 32 Mo.
const POOL_COUNT: usize = 8;
const POOL_BYTES: usize = 32 << 20;

/// Clé d'une rastérisation : contenu, échelle et position sous-pixel.
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub(super) struct RasterKey {
    pub content: u64,
    pub scale: [u32; 2],
    pub offset: [u32; 2],
}
pub(super) struct Raster {
    pub texture: Texture,
    pub width: usize,
    pub height: usize,
    used: u64,
}
/// Texture d'une image du moteur ; l'`Arc` retenu empêche la réutilisation de l'adresse
/// qui sert de clé.
struct Uploaded {
    _image: Arc<Image>,
    texture: Texture,
    used: u64,
}
/// Groupe filtré déjà calculé : sortie des filtres (zone utile `size`).
pub(super) struct Filtered {
    key: Signature,
    placement: [u32; 8],
    pub texture: Texture,
    pub size: [usize; 2],
    /// Retient les `Arc` (images, dégradés) dont la signature compare les adresses.
    _source: Vec<DrawCommand>,
}
/// Texture de rendu en réserve : taille et format, texture, dernière image qui l'a prise.
struct Pooled {
    key: ([usize; 2], MTLPixelFormat),
    texture: Texture,
    used: u64,
}
/// Ce que `trim` libère.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::platform::macos) enum Trim {
    /// Repos : réserve des cibles, images et rastérisations que la dernière image n'a pas
    /// employées. Le prochain dessin retrouve atlas, textures visibles et groupes filtrés.
    Idle,
    /// Fenêtre masquée ou réduite : tout.
    Hidden,
}
pub(super) struct Resources {
    pub atlas: Atlas,
    images: HashMap<usize, Uploaded>,
    image_bytes: usize,
    rasters: HashMap<RasterKey, Raster>,
    raster_bytes: usize,
    /// Textures des couches et des filtres libres et prises par l'image en cours.
    layers: Vec<Pooled>,
    taken: Vec<Pooled>,
    filtered: HashMap<ElementId, Filtered>,
    used_filtered: HashSet<ElementId>,
    /// Texture 1 × 1 liée aux lots sans texture.
    pub blank: Texture,
    frame: u64,
}
fn size(texture: &Texture) -> usize {
    texture.allocatedSize()
}
impl Resources {
    pub fn new(gpu: &Gpu) -> crate::Result<Self> {
        let blank = gpu.upload_texture(MTLPixelFormat::BGRA8Unorm, 1, 1, false)?;
        replace(&blank, (0, 0, 1, 1), &[0; 4], 4);
        Ok(Self {
            atlas: Atlas::default(),
            images: HashMap::new(),
            image_bytes: 0,
            rasters: HashMap::new(),
            raster_bytes: 0,
            layers: Vec::new(),
            taken: Vec::new(),
            filtered: HashMap::new(),
            used_filtered: HashSet::new(),
            blank,
            frame: 0,
        })
    }
    /// Octets alloués : atlas, images, rastérisations, réserve des cibles, groupes filtrés.
    pub fn usage(&self) -> [usize; 5] {
        [
            self.atlas.bytes(),
            self.images.values().map(|u| size(&u.texture)).sum(),
            self.rasters.values().map(|r| size(&r.texture)).sum(),
            self.layers
                .iter()
                .chain(&self.taken)
                .map(|p| size(&p.texture))
                .sum(),
            self.filtered.values().map(|f| size(&f.texture)).sum(),
        ]
    }
    pub fn begin(&mut self) {
        self.frame += 1;
        self.used_filtered.clear();
        self.layers.append(&mut self.taken);
    }
    /// Libère ce que les images récentes n'ont plus utilisé (voir le module).
    pub fn end(&mut self) {
        let frame = self.frame;
        let keep = |used: u64, over: bool| used == frame || (!over && frame - used < IDLE_FRAMES);
        let over = self.image_bytes > IMAGE_BYTES;
        self.images.retain(|_, u| keep(u.used, over));
        self.image_bytes = self.images.values().map(|u| size(&u.texture)).sum();
        let over = self.raster_bytes > RASTER_BYTES;
        self.rasters.retain(|_, r| keep(r.used, over));
        self.raster_bytes = self.rasters.values().map(|r| size(&r.texture)).sum();
        self.filtered
            .retain(|id, _| self.used_filtered.contains(id));
        // Réserve : les plus récemment prises d'abord, dans les bornes.
        self.layers
            .retain(|p| frame - p.used < POOL_FRAMES || p.used == frame);
        self.layers.sort_by_key(|p| std::cmp::Reverse(p.used));
        let mut bytes = 0;
        let mut count = 0;
        self.layers.retain(|p| {
            bytes += size(&p.texture);
            count += 1;
            count <= POOL_COUNT && bytes <= POOL_BYTES
        });
    }
    /// Libère ce qui peut l'être entre deux dessins (voir `Trim`) ; le GPU retient les
    /// textures encore lues par une image en vol.
    pub fn trim(&mut self, what: Trim) {
        self.layers.clear();
        let frame = self.frame;
        match what {
            Trim::Idle => {
                self.images.retain(|_, u| u.used == frame);
                self.rasters.retain(|_, r| r.used == frame);
            }
            Trim::Hidden => {
                self.images.clear();
                self.rasters.clear();
                self.filtered.clear();
                self.atlas.clear();
            }
        }
        self.image_bytes = self.images.values().map(|u| size(&u.texture)).sum();
        self.raster_bytes = self.rasters.values().map(|r| size(&r.texture)).sum();
    }
    /// Texture de `image` ; la seconde valeur est vraie si elle vient d'être créée (ses
    /// mipmaps restent à calculer).
    pub fn image(&mut self, gpu: &Gpu, image: &Arc<Image>) -> crate::Result<(Texture, bool)> {
        let key = Arc::as_ptr(image) as usize;
        if let Some(found) = self.images.get_mut(&key) {
            found.used = self.frame;
            return Ok((found.texture.clone(), false));
        }
        let (w, h) = (image.width as usize, image.height as usize);
        if w == 0 || h == 0 || w > 16384 || h > 16384 || image.pixels.len() != w * h * 4 {
            return Err(crate::Error::invalid_input("invalid image buffer"));
        }
        let texture = gpu.upload_texture(MTLPixelFormat::BGRA8Unorm, w, h, true)?;
        replace(&texture, (0, 0, w, h), &image.pixels, w * 4);
        self.image_bytes += size(&texture);
        self.images.insert(
            key,
            Uploaded {
                _image: Arc::clone(image),
                texture: texture.clone(),
                used: self.frame,
            },
        );
        Ok((texture, true))
    }
    /// Rastérisation `key`, créée par `draw` (pixels BGRA de `width` × `height`) si besoin.
    pub fn raster(
        &mut self,
        gpu: &Gpu,
        key: RasterKey,
        (width, height): (usize, usize),
        draw: impl FnOnce() -> crate::Result<Vec<u8>>,
    ) -> crate::Result<Texture> {
        let frame = self.frame;
        if let Some(found) = self.rasters.get_mut(&key)
            && (found.width, found.height) == (width, height)
        {
            found.used = frame;
            return Ok(found.texture.clone());
        }
        let pixels = draw()?;
        let texture = gpu.upload_texture(MTLPixelFormat::BGRA8Unorm, width, height, false)?;
        replace(&texture, (0, 0, width, height), &pixels, width * 4);
        self.raster_bytes += size(&texture);
        self.rasters.insert(
            key,
            Raster {
                texture: texture.clone(),
                width,
                height,
                used: frame,
            },
        );
        Ok(texture)
    }
    /// Texture de rendu de `size` pixels (arrondis) au format `format`, prise pour
    /// l'image en cours (couches, passages de filtre).
    pub fn target(
        &mut self,
        gpu: &Gpu,
        size: [usize; 2],
        format: MTLPixelFormat,
    ) -> crate::Result<Texture> {
        let key = (size, format);
        let texture = match self.layers.iter().position(|p| p.key == key) {
            Some(index) => self.layers.swap_remove(index).texture,
            None => gpu.render_texture(format, size[0], size[1])?,
        };
        self.taken.push(Pooled {
            key,
            texture: texture.clone(),
            used: self.frame,
        });
        Ok(texture)
    }
    /// Résultat gardé du groupe filtré `id`, si son contenu et sa place n'ont pas changé.
    pub fn filtered(
        &mut self,
        id: ElementId,
        key: &Signature,
        placement: [u32; 8],
    ) -> Option<&Filtered> {
        self.used_filtered.insert(id);
        self.filtered
            .get(&id)
            .filter(|f| f.placement == placement && f.key == *key)
    }
    /// Texture de sortie du groupe filtré `id` (`size` pixels) : celle de son résultat
    /// précédent s'il a la même taille (le GPU ordonne la réécriture après les lectures
    /// des images en vol), sinon une nouvelle.
    pub fn filter_output(
        &mut self,
        gpu: &Gpu,
        id: ElementId,
        size: [usize; 2],
    ) -> crate::Result<Texture> {
        match self.filtered.remove(&id) {
            Some(f) if f.size == size => Ok(f.texture),
            _ => gpu.render_texture(super::filter::FORMAT, size[0], size[1]),
        }
    }
    pub fn store_filtered(
        &mut self,
        id: ElementId,
        key: Signature,
        placement: [u32; 8],
        texture: Texture,
        size: [usize; 2],
        commands: &[DrawCommand],
    ) {
        self.used_filtered.insert(id);
        self.filtered.insert(
            id,
            Filtered {
                key,
                placement,
                texture,
                size,
                _source: commands.to_vec(),
            },
        );
    }
    /// Nombre d'images, de rastérisations et de cibles en réserve (tests).
    #[cfg(test)]
    pub fn counts(&self) -> [usize; 3] {
        [self.images.len(), self.rasters.len(), self.layers.len()]
    }
}
