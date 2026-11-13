//! Textures gardées d'une image à l'autre : images du moteur (pixels préparés par le
//! renderer Cairo : réduites à leur taille d'affichage, ou d'origine avec mipmaps quand
//! elles sont étirées ou tournées) et rastérisations Cairo des icônes, SVG et polygones
//! (par contenu, échelle et position sous-pixel, comme `macos/metal/resources.rs`).
//!
//! Une texture nouvelle attend ses pixels dans le tampon intermédiaire ; `record` les
//! copie en tête de l'image, puis calcule ses mipmaps par réductions successives
//! (`vkCmdBlitImage`, filtre linéaire : BGRA 8 bits les permet sur tous les GPU de bureau).
//!
//! Bornes (`end`, après une image terminée) : une texture inutilisée depuis
//! `IDLE_FRAMES` images est libérée, et toutes celles que l'image courante n'a pas
//! employées dès que le total de leur sorte dépasse son budget.
use super::{
    atlas::{SAMPLED, TRANSFER, region},
    commands::transition_levels,
    device::Gpu,
    memory::Image,
    staging::Staging,
};
use crate::{
    platform::linux::{bitmap::Surface, ffi::vulkan::*},
    ui::scene::Image as Picture,
};
use std::{collections::HashMap, ffi::c_void, hash::Hash, rc::Rc, sync::Arc};

/// Textures inutilisées depuis ce nombre d'images : libérées.
const IDLE_FRAMES: u64 = 60;
/// Budgets des textures d'images et des rastérisations ; au-delà, celles que l'image
/// courante n'emploie pas sont libérées.
const IMAGE_BYTES: usize = 64 << 20;
const RASTER_BYTES: usize = 32 << 20;
/// Textures d'une sorte au plus : chacune est une allocation Vulkan, et certains pilotes
/// n'en admettent que 4096 (`maxMemoryAllocationCount`) ; les rastérisations sont clés
/// par position au quart de pixel (seize au plus par contenu).
const MAX_TEXTURES: usize = 1024;
/// Lecture d'un niveau par une réduction.
const BLIT_READ: (u64, u64) = (
    VK_PIPELINE_STAGE_2_ALL_TRANSFER_BIT,
    VK_ACCESS_2_TRANSFER_READ_BIT,
);

/// Clé d'une rastérisation : contenu, échelle et position sous-pixel.
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub(super) struct RasterKey {
    pub content: u64,
    pub scale: [u32; 2],
    pub offset: [u32; 2],
}
/// Clé d'une image : adresse de ses pixels, taille de la texture, mipmaps.
type ImageKey = (usize, [usize; 2], bool);

struct Texture {
    image: Image,
    bytes: usize,
    used: u64,
    /// Début des pixels du niveau 0 dans le tampon intermédiaire, tant qu'ils n'ont pas
    /// été copiés.
    pending: Option<usize>,
    /// Retient l'image source, dont l'adresse sert de clé.
    _source: Option<Arc<Picture>>,
    /// Retient l'icône vectorielle d'une rastérisation, dont l'adresse entre dans la clé
    /// (M11 de la revue) : libérée, elle pourrait céder son adresse à une autre icône.
    _icon: Option<Arc<crate::ui::rendering::vector::VectorIcon>>,
}
impl Texture {
    /// Texture de `width` × `height` pixels BGRA (`pixels`, `stride` octets par ligne),
    /// avec tous ses niveaux de mipmap si `mipmaps`.
    fn new(
        gpu: &Rc<Gpu>,
        staging: &mut Staging,
        (width, height): (usize, usize),
        (pixels, stride): (&[u8], usize),
        mipmaps: bool,
    ) -> crate::Result<Self> {
        if pixels.len() < stride * height.saturating_sub(1) + width * 4 {
            return Err(crate::Error::invalid_input("invalid image buffer"));
        }
        let levels = if mipmaps {
            usize::BITS - width.max(height).leading_zeros()
        } else {
            1
        };
        let mut usage = VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
        if levels > 1 {
            usage |= VK_IMAGE_USAGE_TRANSFER_SRC_BIT;
        }
        let image = Image::with_levels(
            gpu,
            (width, height),
            VK_FORMAT_B8G8R8A8_UNORM,
            usage,
            levels,
        )?;
        let offset = staging.push(pixels, stride, width * 4, height);
        let bytes = width * height * 4;
        Ok(Self {
            image,
            bytes: if levels > 1 { bytes / 3 * 4 } else { bytes },
            used: 0,
            pending: Some(offset),
            _source: None,
            _icon: None,
        })
    }
}
pub(super) struct Textures {
    images: HashMap<ImageKey, Texture>,
    rasters: HashMap<RasterKey, Texture>,
    frame: u64,
    gpu: Rc<Gpu>,
}
impl Textures {
    pub fn new(gpu: &Rc<Gpu>) -> Self {
        Self {
            images: HashMap::new(),
            rasters: HashMap::new(),
            frame: 0,
            gpu: Rc::clone(gpu),
        }
    }
    pub fn begin(&mut self) {
        self.frame += 1;
    }
    /// Vue de la texture de `source` faite des pixels de `surface` (préparée par le
    /// renderer Cairo), avec mipmaps si elle est étirée.
    pub fn image(
        &mut self,
        staging: &mut Staging,
        source: &Arc<Picture>,
        surface: &dyn Surface,
        mipmaps: bool,
    ) -> crate::Result<u64> {
        let size = surface.size();
        let key = (Arc::as_ptr(source) as usize, [size.0, size.1], mipmaps);
        let frame = self.frame;
        if let Some(found) = self.images.get_mut(&key) {
            found.used = frame;
            return Ok(found.image.view);
        }
        let mut texture = Texture::new(&self.gpu, staging, size, surface.bytes(), mipmaps)?;
        texture.used = frame;
        texture._source = Some(Arc::clone(source));
        let view = texture.image.view;
        self.images.insert(key, texture);
        Ok(view)
    }
    /// Vue de la rastérisation `key` de `width` × `height` pixels, dessinée par `draw`
    /// (pixels BGRA jointifs) si elle est nouvelle.
    pub fn raster(
        &mut self,
        staging: &mut Staging,
        key: RasterKey,
        (width, height): (usize, usize),
        icon: Option<&Arc<crate::ui::rendering::vector::VectorIcon>>,
        draw: impl FnOnce() -> crate::Result<Vec<u8>>,
    ) -> crate::Result<u64> {
        let frame = self.frame;
        if let Some(found) = self.rasters.get_mut(&key)
            && (found.image.width, found.image.height) == (width, height)
        {
            found.used = frame;
            return Ok(found.image.view);
        }
        let pixels = draw()?;
        let mut texture = Texture::new(
            &self.gpu,
            staging,
            (width, height),
            (&pixels, width * 4),
            false,
        )?;
        texture.used = frame;
        texture._icon = icon.cloned();
        let view = texture.image.view;
        self.rasters.insert(key, texture);
        Ok(view)
    }
    /// Enregistre dans `command` les copies en attente depuis le tampon intermédiaire
    /// `staging`, puis les mipmaps ; chaque texture finit en lecture par le shader.
    pub fn record(&mut self, command: *mut c_void, staging: u64) {
        let gpu = &*self.gpu;
        for texture in self.images.values_mut().chain(self.rasters.values_mut()) {
            let Some(offset) = texture.pending.take() else {
                continue;
            };
            let image = &mut texture.image;
            let (raw, levels) = (image.raw, image.levels);
            let (dst, src) = (
                VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            );
            let read = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
            let none = (VK_PIPELINE_STAGE_2_NONE, VK_ACCESS_2_NONE);
            transition_levels(
                gpu,
                command,
                raw,
                (0, levels),
                (VK_IMAGE_LAYOUT_UNDEFINED, dst),
                none,
                TRANSFER,
            );
            let copy = region([0, 0, image.width, image.height], offset);
            // SAFETY : région dans l'image et dans le tampon intermédiaire (écrit par
            // `Staging::send`).
            unsafe { (gpu.fns.vkCmdCopyBufferToImage)(command, staging, raw, dst, 1, &copy) };
            let (mut w, mut h) = (image.width as i32, image.height as i32);
            for level in 1..levels {
                transition_levels(
                    gpu,
                    command,
                    raw,
                    (level - 1, 1),
                    (dst, src),
                    TRANSFER,
                    BLIT_READ,
                );
                let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
                let layers = |level| VkImageSubresourceLayers {
                    aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                    mipLevel: level,
                    baseArrayLayer: 0,
                    layerCount: 1,
                };
                let corner = |x, y| VkOffset3D { x, y, z: 0 };
                let blit = VkImageBlit {
                    srcSubresource: layers(level - 1),
                    srcOffsets: [
                        corner(0, 0),
                        VkOffset3D {
                            z: 1,
                            ..corner(w, h)
                        },
                    ],
                    dstSubresource: layers(level),
                    dstOffsets: [
                        corner(0, 0),
                        VkOffset3D {
                            z: 1,
                            ..corner(nw, nh)
                        },
                    ],
                };
                // SAFETY : niveaux distincts de la même image, en source et destination.
                unsafe {
                    (gpu.fns.vkCmdBlitImage)(
                        command,
                        raw,
                        src,
                        raw,
                        dst,
                        1,
                        &blit,
                        VK_FILTER_LINEAR,
                    )
                };
                (w, h) = (nw, nh);
            }
            // Niveaux réduits en source, le dernier en destination : tous lus ensuite.
            if levels > 1 {
                transition_levels(
                    gpu,
                    command,
                    raw,
                    (0, levels - 1),
                    (src, read),
                    (BLIT_READ.0, VK_ACCESS_2_NONE),
                    SAMPLED,
                );
            }
            transition_levels(
                gpu,
                command,
                raw,
                (levels - 1, 1),
                (dst, read),
                TRANSFER,
                SAMPLED,
            );
            image.layout = read;
        }
    }
    /// Après une image terminée : libère ce que les images récentes n'ont plus employé.
    pub fn end(&mut self) {
        let frame = self.frame;
        retain(&mut self.images, frame, IMAGE_BYTES);
        retain(&mut self.rasters, frame, RASTER_BYTES);
    }
    /// Garde seulement les textures employées par la dernière image (GPU au repos).
    pub fn trim(&mut self) {
        let frame = self.frame;
        self.images.retain(|_, t| t.used == frame);
        self.rasters.retain(|_, t| t.used == frame);
    }
    /// Oublie toutes les textures (GPU au repos).
    pub fn clear(&mut self) {
        self.images.clear();
        self.rasters.clear();
    }
    /// Nombre de textures d'images et de rastérisations (tests).
    #[cfg(test)]
    pub fn counts(&self) -> [usize; 2] {
        [self.images.len(), self.rasters.len()]
    }
}
/// Garde les textures employées par l'image `frame`, et les autres tant qu'elles ont
/// servi depuis moins de `IDLE_FRAMES` images et que leur total reste sous `budget` et
/// leur nombre sous `MAX_TEXTURES`.
fn retain<K: Hash + Eq>(textures: &mut HashMap<K, Texture>, frame: u64, budget: usize) {
    let over =
        textures.values().map(|t| t.bytes).sum::<usize>() > budget || textures.len() > MAX_TEXTURES;
    textures.retain(|_, t| t.used == frame || (!over && frame - t.used < IDLE_FRAMES));
}
