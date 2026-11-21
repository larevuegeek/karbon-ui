//! Image vivante : une source d'image renouvelée sans cesse (la vue 3D d'un moteur en mode
//! copie), dont le backend garde la texture et la met à jour en place quand la taille ne
//! change pas, au lieu d'en créer une neuve à chaque image.
use super::Image;
use crate::ui::template::ImageSlot;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

/// Dernière image livrée par chaque source vivante, par numéro de source. Un `Weak`
/// garde l'adresse réservée : une autre image ne peut pas la reprendre et passer pour
/// celle de la source.
static LIVE: Mutex<Option<HashMap<u64, Weak<Image>>>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);

impl Image {
    /// Wrap opaque BGRA pixels without checking each one: every alpha byte is set to 255,
    /// so the buffer is valid premultiplied BGRA whatever it held. Cheaper than
    /// [`Image::new`] for a large image made every frame (the 3D view of an engine).
    ///
    /// # Errors
    ///
    /// [`ImageError::InvalidPixels`](crate::ImageError::InvalidPixels) if a dimension is
    /// zero or the buffer length is not `width * height * 4`.
    ///
    /// ```
    /// use karbon_ui::ui::scene::Image;
    ///
    /// let image = Image::opaque_bgra(2, 1, vec![10, 20, 30, 0, 40, 50, 60, 7]).unwrap();
    /// assert_eq!(image.pixels, [10, 20, 30, 255, 40, 50, 60, 255]);
    /// ```
    pub fn opaque_bgra(width: u32, height: u32, mut pixels: Vec<u8>) -> crate::Result<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || expected != Some(pixels.len()) {
            return Err(crate::ImageError::InvalidPixels.into());
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

/// A source of images renewed all the time, like the 3D view of an engine copied into
/// the window: an [`ImageSlot`] whose successive images the backend recognises as the
/// same picture. When the new image has the size of the previous one, its texture is
/// updated in place (Direct2D `CopyFromMemory`) instead of created anew. Elsewhere, and
/// for an image of another size, it behaves exactly like an `ImageSlot`.
///
/// Clones share the same source; [`set`](Self::set) may be called from any thread.
///
/// ```
/// use karbon_ui::ui::scene::{Image, LiveImage};
/// use std::sync::Arc;
///
/// let view = LiveImage::new();
/// let frame = Image::opaque_bgra(4, 4, vec![0; 64]).unwrap();
/// view.set(Arc::new(frame));
/// assert_eq!(view.image().unwrap().width, 4);
/// // In a widget: paint `view.image()` and return `Some(view.slot())` from
/// // `Widget::deferred_image`, so that each new image repaints without rebuilding.
/// ```
#[derive(Clone)]
pub struct LiveImage(Arc<Source>);

struct Source {
    id: u64,
    slot: ImageSlot,
}

impl Drop for Source {
    fn drop(&mut self) {
        if let Some(live) = LIVE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            live.remove(&self.id);
        }
    }
}

impl Default for LiveImage {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveImage {
    /// An empty source: nothing is painted until [`set`](Self::set).
    pub fn new() -> Self {
        Self(Arc::new(Source {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            slot: ImageSlot::new(),
        }))
    }

    /// Deliver the next image: the elements showing this source are repainted at the
    /// next frame, without rebuilding the view.
    pub fn set(&self, image: Arc<Image>) {
        LIVE.lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_or_insert_with(HashMap::new)
            .insert(self.0.id, Arc::downgrade(&image));
        self.0.slot.set(image);
    }

    /// The current image, if any.
    pub fn image(&self) -> Option<Arc<Image>> {
        self.0.slot.image()
    }

    /// The slot behind the source, to return from
    /// [`Widget::deferred_image`](crate::ui::element::Widget::deferred_image).
    pub fn slot(&self) -> &ImageSlot {
        &self.0.slot
    }
}

/// Numéro de la source vivante dont `image` est la dernière image livrée, s'il y en a
/// une : le backend reprend alors la texture de l'image précédente de cette source.
/// Seul Direct2D s'en sert pour l'instant.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn live_source(image: &Arc<Image>) -> Option<u64> {
    let live = LIVE.lock().unwrap_or_else(|e| e.into_inner());
    live.as_ref()?
        .iter()
        .find(|(_, weak)| std::ptr::eq(weak.as_ptr(), Arc::as_ptr(image)))
        .map(|(id, _)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_source_recognises_only_its_latest_image() {
        let view = LiveImage::new();
        let first = Arc::new(Image::opaque_bgra(1, 1, vec![0; 4]).unwrap());
        view.set(Arc::clone(&first));
        assert!(live_source(&first).is_some());
        let second = Arc::new(Image::opaque_bgra(1, 1, vec![9; 4]).unwrap());
        view.set(Arc::clone(&second));
        assert_eq!(live_source(&first), None);
        assert!(live_source(&second).is_some());
        // Une image ordinaire n'est d'aucune source ; une source détruite s'oublie.
        let plain = Arc::new(Image::new(1, 1, vec![0, 0, 0, 255]).unwrap());
        assert_eq!(live_source(&plain), None);
        drop(view);
        assert_eq!(live_source(&second), None);
        assert!(Image::opaque_bgra(2, 2, vec![0; 4]).is_err());
        assert!(Image::opaque_bgra(0, 1, vec![]).is_err());
    }
}
