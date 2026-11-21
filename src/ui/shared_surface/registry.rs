//! Les gabarits des surfaces partagées vivantes : l'image qu'un élément peint à la place
//! d'une surface, par laquelle le backend reconnaît la surface dans une scène.
use super::surface::Shared;
use crate::ui::scene::Image;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicUsize, Ordering},
};

/// Adresse du gabarit de chaque surface vivante, et la surface. Le gabarit appartient à
/// sa surface : son adresse ne peut pas être reprise par une autre image tant qu'elle vit.
static SURFACES: Mutex<Vec<(usize, Weak<Shared>)>> = Mutex::new(Vec::new());
/// Nombre de surfaces vivantes : zéro dans presque toutes les applications, ce qui épargne
/// le verrou à chaque image dessinée.
static COUNT: AtomicUsize = AtomicUsize::new(0);

/// Inscrit le gabarit de `surface`.
pub(super) fn register(surface: &Arc<Shared>) {
    let key = Arc::as_ptr(surface.placeholder()) as usize;
    let mut surfaces = SURFACES.lock().unwrap_or_else(|e| e.into_inner());
    surfaces.push((key, Arc::downgrade(surface)));
    COUNT.store(surfaces.len(), Ordering::Release);
}

/// Retire le gabarit d'adresse `key` (la surface est détruite).
pub(super) fn unregister(key: usize) {
    let mut surfaces = SURFACES.lock().unwrap_or_else(|e| e.into_inner());
    surfaces.retain(|(other, _)| *other != key);
    COUNT.store(surfaces.len(), Ordering::Release);
}

/// La surface dont `image` est le gabarit, s'il y en a une.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn surface_of(image: &Arc<Image>) -> Option<Arc<Shared>> {
    if COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    let key = Arc::as_ptr(image) as usize;
    let surfaces = SURFACES.lock().unwrap_or_else(|e| e.into_inner());
    let (_, surface) = surfaces.iter().find(|(other, _)| *other == key)?;
    surface.upgrade()
}
