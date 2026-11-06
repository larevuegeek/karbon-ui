//! Ressources du GPU gardées d'une image à l'autre : atlas des glyphes, textures des
//! images et des rastérisations, le tampon intermédiaire qui leur envoie leurs pixels, et
//! les résultats des groupes filtrés (par signature de leur contenu et place, comme
//! `graphics/cache.rs` ; un groupe absent d'une image est oublié). Le rendu attend chaque
//! image (`Commands::submit_wait`) : entre deux images, le GPU ne lit plus rien et les
//! ressources peuvent être libérées.
use super::{atlas::Atlas, device::Gpu, memory::Image, staging::Staging, textures::Textures};
use crate::ui::{
    element::ElementId,
    scene::{DrawCommand, signature::Signature},
};
use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    rc::Rc,
};

/// Ce qui identifie le résultat d'un groupe filtré : contenu, place et zone utile.
pub(super) struct Kept {
    pub id: ElementId,
    pub key: Signature,
    /// Repère de la couche (quantifié au quart de pixel) et taille de la zone.
    pub placement: [u32; 8],
    pub size: [usize; 2],
    /// Retient les `Arc` (images, dégradés) dont la signature compare les adresses.
    pub _source: Vec<DrawCommand>,
}
/// Résultat gardé d'un groupe filtré.
pub(super) struct Filtered {
    pub image: Image,
    pub kept: Kept,
}
pub(super) struct Resources {
    pub atlas: Atlas,
    pub textures: Textures,
    pub staging: Staging,
    filtered: HashMap<ElementId, Filtered>,
    used_filtered: HashSet<ElementId>,
}
impl Resources {
    pub fn new(gpu: &Rc<Gpu>) -> Self {
        Self {
            atlas: Atlas::new(gpu),
            textures: Textures::new(gpu),
            staging: Staging::new(gpu),
            filtered: HashMap::new(),
            used_filtered: HashSet::new(),
        }
    }
    /// Résultat gardé du groupe filtré `id`, retiré du cache pour l'image en cours, si son
    /// contenu et sa place n'ont pas changé.
    pub fn take_filtered(
        &mut self,
        id: ElementId,
        key: &Signature,
        placement: [u32; 8],
    ) -> Option<Filtered> {
        self.used_filtered.insert(id);
        let found = self.filtered.get(&id)?;
        if found.kept.placement != placement || found.kept.key != *key {
            return None;
        }
        self.filtered.remove(&id)
    }
    /// Image du résultat précédent du groupe `id`, réutilisée si elle a la taille
    /// `size` ; le résultat précédent est oublié.
    pub fn filter_output(&mut self, id: ElementId, size: [usize; 2]) -> Option<Image> {
        self.filtered
            .remove(&id)
            .filter(|f| [f.image.width, f.image.height] == size)
            .map(|f| f.image)
    }
    /// Garde le résultat d'un groupe filtré après l'image qui l'a calculé ou employé.
    pub fn store_filtered(&mut self, image: Image, kept: Kept) {
        self.used_filtered.insert(kept.id);
        self.filtered.insert(kept.id, Filtered { image, kept });
    }
    /// Début d'une image (la précédente est terminée) : un atlas trop plein est vidé.
    pub fn begin(&mut self) {
        if self.atlas.full() {
            self.atlas.clear();
        }
        self.textures.begin();
    }
    /// Enregistre dans `command`, en tête de l'image, les envois en attente.
    pub fn upload(&mut self, command: *mut c_void) -> crate::Result<()> {
        if self.staging.is_empty() {
            return Ok(());
        }
        let buffer = self.staging.send()?;
        self.atlas.record(command, buffer);
        self.textures.record(command, buffer);
        Ok(())
    }
    /// Après une image terminée : textures inutilisées, groupes filtrés absents de
    /// l'image et grand tampon libérés.
    pub fn end(&mut self) {
        self.textures.end();
        self.staging.trim();
        self.filtered
            .retain(|id, _| self.used_filtered.contains(id));
        self.used_filtered.clear();
    }
    /// Allège les ressources entre deux images (GPU au repos) : `hidden`, tout est oublié
    /// (fenêtre cachée), sinon seules les textures de la dernière image restent ; le
    /// tampon intermédiaire est rendu.
    pub fn trim(&mut self, hidden: bool) {
        if hidden {
            self.reset();
        } else {
            self.textures.trim();
        }
        self.staging.release();
    }
    /// Oublie tout (enregistrement abandonné : les copies prévues n'ont pas eu lieu).
    pub fn reset(&mut self) {
        self.atlas.clear();
        self.textures.clear();
        self.filtered.clear();
        self.staging.clear();
    }
}
