//! Image dont les pixels peuvent arriver plus tard (pochette décodée par une tâche de
//! fond), comme la source d'un `<img>` de navigateur : la vue la montre tout de suite,
//! vide ou avec une image d'attente, et le runtime la repeint quand elle arrive, sans
//! reconstruire la vue. La mise en page n'est refaite que si la taille de l'élément
//! dépend de l'image (largeur ou hauteur non fixée, voir `UiRuntime::images_arrived`).
use crate::ui::scene::Image;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};

/// Nombre total d'images arrivées dans des [`ImageSlot`] : le runtime le compare à sa
/// dernière lecture pour savoir s'il doit chercher les images à repeindre.
static ARRIVALS: AtomicU64 = AtomicU64::new(0);

/// Shared, fillable source of an image element (`<img src={…}>`): the view shows it
/// right away, empty or with a placeholder, and the runtime repaints it when
/// [`ImageSlot::set`] delivers the decoded pixels, without rebuilding the view. Layout
/// is redone only when the element's size depends on the image (width or height not
/// fixed by its style).
///
/// Clones share the same image. An `Arc<Image>` converts into a ready slot, so
/// `src={…}` accepts both.
///
/// ```
/// use karbon_ui::ui::{scene::Image, template::ImageSlot};
/// use std::sync::Arc;
///
/// let slot = ImageSlot::new();
/// assert!(slot.image().is_none());
/// // Later, typically in `Application::task_completed`, which can then return `false`:
/// slot.set(Arc::new(Image::new(1, 1, vec![0, 0, 0, 255]).unwrap()));
/// assert_eq!(slot.image().unwrap().width, 1);
/// ```
#[derive(Clone, Default)]
pub struct ImageSlot(Arc<Inner>);

#[derive(Default)]
struct Inner {
    image: RwLock<Option<Arc<Image>>>,
    /// Images livrées à cette source (le runtime repère ainsi celles de son arbre).
    revision: AtomicU64,
}

impl ImageSlot {
    /// Creates an empty slot: nothing is painted until [`ImageSlot::set`].
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates a slot showing `image` (a placeholder that [`ImageSlot::set`] may replace,
    /// or the final image).
    pub fn ready(image: Arc<Image>) -> Self {
        Self(Arc::new(Inner {
            image: RwLock::new(Some(image)),
            revision: AtomicU64::new(0),
        }))
    }
    /// Delivers the image: every element showing this slot is repainted at the next
    /// frame (the window is woken when this happens in
    /// [`Application::task_completed`](crate::application::Application::task_completed)).
    pub fn set(&self, image: Arc<Image>) {
        *self.0.image.write().unwrap_or_else(|e| e.into_inner()) = Some(image);
        self.0.revision.fetch_add(1, Ordering::Relaxed);
        ARRIVALS.fetch_add(1, Ordering::Relaxed);
    }
    /// The current image, if any.
    pub fn image(&self) -> Option<Arc<Image>> {
        self.0
            .image
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    /// Identity of the slot, shared by its clones (for example to key a memoized
    /// subtree that must not be rebuilt when the image arrives).
    pub fn id(&self) -> usize {
        Arc::as_ptr(&self.0) as *const u8 as usize
    }
    /// Images delivered so far by every slot of the process.
    pub(crate) fn arrivals() -> u64 {
        ARRIVALS.load(Ordering::Relaxed)
    }
    /// Images delivered so far to this slot.
    pub(crate) fn revision(&self) -> u64 {
        self.0.revision.load(Ordering::Relaxed)
    }
}

impl From<Arc<Image>> for ImageSlot {
    fn from(image: Arc<Image>) -> Self {
        Self::ready(image)
    }
}

impl From<&ImageSlot> for ImageSlot {
    fn from(slot: &ImageSlot) -> Self {
        slot.clone()
    }
}

impl std::fmt::Debug for ImageSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.image().map(|i| (i.width, i.height));
        f.debug_struct("ImageSlot").field("image", &size).finish()
    }
}
