//! Caches du renderer d'une image à l'autre : surfaces des images et images réduites à
//! leur taille d'affichage, groupes filtrés (signature de leur contenu, comme sous
//! Windows et macOS) et `backdrop-filter` (empreinte des pixels relus). Chaque peinture ne
//! garde que les entrées qu'elle a utilisées.
use super::super::{
    bitmap::{Bitmap, ImageSurface, Surface},
    ffi::cairo::FILTER_GOOD,
    imaging,
};
use crate::ui::{
    element::ElementId,
    scene::{DrawCommand, Image, signature::Signature},
};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

pub(super) use crate::platform::shared::{hash, placement};

/// Groupe filtré déjà calculé.
struct Filtered {
    key: Signature,
    /// Repère de la couche (quantifié au quart de pixel) et taille de la zone.
    placement: [u32; 8],
    image: Rc<Bitmap>,
    /// Retient les `Arc` (images, dégradés) dont la signature compare les adresses.
    _source: Vec<DrawCommand>,
}
/// `backdrop-filter` déjà calculé pour des pixels d'empreinte `hash`.
struct Backdrop {
    hash: u64,
    image: Rc<Bitmap>,
}
pub(super) type BackdropKey = [u64; 8];
#[derive(Default)]
pub(super) struct Caches {
    pub images: HashMap<usize, Rc<ImageSurface>>,
    scaled: HashMap<(usize, usize, usize), Rc<Bitmap>>,
    filters: HashMap<ElementId, Filtered>,
    backdrops: HashMap<BackdropKey, Backdrop>,
    used_scaled: HashSet<(usize, usize, usize)>,
    /// Tailles affichées de chaque image lors de sa dernière peinture (une image peut
    /// être affichée à plusieurs tailles).
    requested: HashMap<usize, HashSet<(usize, usize)>>,
    requesting: HashMap<usize, HashSet<(usize, usize)>>,
    /// Une image a été dessinée approchée (animation) : une image exacte doit suivre.
    pub refine: bool,
    used_filters: HashSet<ElementId>,
    used_backdrops: HashSet<BackdropKey>,
}
impl Caches {
    /// Début d'une peinture : rien n'est encore marqué utilisé.
    pub fn begin(&mut self) {
        self.used_scaled.clear();
        self.used_filters.clear();
        self.used_backdrops.clear();
    }
    /// Fin d'une peinture : les entrées inutilisées sont libérées.
    pub fn end(&mut self) {
        self.scaled.retain(|k, _| self.used_scaled.contains(k));
        self.filters.retain(|k, _| self.used_filters.contains(k));
        self.backdrops
            .retain(|k, _| self.used_backdrops.contains(k));
        for (id, sizes) in self.requesting.drain() {
            self.requested.insert(id, sizes);
        }
        self.requested.retain(|id, _| self.images.contains_key(id));
    }
    pub fn filtered(
        &mut self,
        id: ElementId,
        key: &Signature,
        placement: [u32; 8],
    ) -> Option<Rc<Bitmap>> {
        self.used_filters.insert(id);
        self.filters
            .get(&id)
            .filter(|f| f.placement == placement && f.key == *key)
            .map(|f| Rc::clone(&f.image))
    }
    pub fn store_filtered(
        &mut self,
        id: ElementId,
        key: Signature,
        placement: [u32; 8],
        image: Rc<Bitmap>,
        commands: &[DrawCommand],
    ) {
        self.filters.insert(
            id,
            Filtered {
                key,
                placement,
                image,
                _source: commands.to_vec(),
            },
        );
    }
    pub fn backdrop(&mut self, key: BackdropKey, hash: u64) -> Option<Rc<Bitmap>> {
        self.used_backdrops.insert(key);
        self.backdrops
            .get(&key)
            .filter(|b| b.hash == hash)
            .map(|b| Rc::clone(&b.image))
    }
    pub fn store_backdrop(&mut self, key: BackdropKey, hash: u64, image: Rc<Bitmap>) {
        self.backdrops.insert(key, Backdrop { hash, image });
    }
    /// Surface de `image`, redimensionnée une fois à sa taille d'affichage pour une copie
    /// rapide à chaque image.
    pub fn image(
        &mut self,
        image: &Arc<Image>,
        width: usize,
        height: usize,
    ) -> crate::Result<Option<Rc<dyn Surface>>> {
        let id = Arc::as_ptr(image) as usize;
        let Some(native) = self.images.get(&id) else {
            return Ok(None);
        };
        let resized = (width, height) != (image.width as usize, image.height as usize);
        if !resized || width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Ok(Some(native.clone()));
        }
        let key = (id, width, height);
        self.used_scaled.insert(key);
        self.requesting
            .entry(id)
            .or_default()
            .insert((width, height));
        if let Some(scaled) = self.scaled.get(&key) {
            return Ok(Some(scaled.clone()));
        }
        // Image déjà affichée à la peinture précédente, mais pas à cette taille : elle est
        // animée, et étirée à l'affichage plutôt que redimensionnée à chaque image.
        let animated = self
            .requested
            .get(&id)
            .is_some_and(|sizes| !sizes.contains(&(width, height)));
        if animated {
            self.refine = true;
            return Ok(Some(native.clone()));
        }
        let surface = Bitmap::new(width, height)?;
        imaging::draw_scaled(&surface, &**native, FILTER_GOOD)?;
        let scaled = Rc::new(surface);
        self.scaled.insert(key, Rc::clone(&scaled));
        Ok(Some(scaled))
    }
}
