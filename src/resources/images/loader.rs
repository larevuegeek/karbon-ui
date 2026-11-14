//! Chargeur d'images : cache borné, demandes notées pendant la vue, tâches de fond
//! ordonnancées par le runtime, sources d'image remplies à l'arrivée.
use super::job::{self, Loaded, Resolve};
use crate::resources::{ImageCache, ImageDecoder};
use crate::task::{Task, TaskCommand, TaskId, TaskResult};
use crate::ui::{scene::Image, template::ImageSlot};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Identifiants des tâches du chargeur : bit de poids fort posé, hors de ceux de
/// l'application.
const TASK_BIT: u64 = 1 << 63;

/// Source et plus grand côté décodé.
type Key = (String, u32);

/// Loads images in the background for the view, like a browser's `<img src>`: the view
/// asks for an image by its source and gets an [`ImageSlot`] showing a placeholder; the
/// file is decoded off the UI thread, kept in a memory-bounded cache, and the slot is
/// repainted when it arrives, without rebuilding the view.
///
/// Create it in [`Application::load`](crate::application::Application::load), keep it
/// with the application's resources and return it from
/// [`Application::image_loader`](crate::application::Application::image_loader): the
/// runtime then schedules its tasks and delivers their results itself. The handle is
/// cheap to clone; clones share the cache.
///
/// ```no_run
/// use karbon_ui::resources::ImageLoader;
/// # use karbon_ui::application::Resources;
/// # fn load(resources: &dyn Resources) {
/// let images = ImageLoader::new(resources.image_decoder())
///     .budget(128 << 20)
///     .thumbnail_edge(448);
/// // In a view: <img src={images.load(&album.cover)}>
/// let slot = images.load("covers/album.jpg");
/// # }
/// ```
///
/// A source is a file path, or a `data:image/…;base64,…` URL. Each request is decoded
/// once per size: failed sources are not retried until
/// [`forget_failures`](Self::forget_failures).
#[derive(Clone)]
pub struct ImageLoader(Rc<Inner>);

struct Inner {
    decoder: Option<Arc<dyn ImageDecoder>>,
    resolve: RefCell<Option<Resolve>>,
    placeholder: RefCell<Arc<Image>>,
    edge: Cell<u32>,
    max_in_flight: Cell<usize>,
    cache: RefCell<ImageCache<Key>>,
    /// Demandes de la vue absentes du cache, dans l'ordre.
    wanted: RefCell<VecDeque<Key>>,
    /// Chargements en vol, par tâche.
    in_flight: RefCell<HashMap<TaskId, Key>>,
    /// Demandes en vol ou en attente, pour ne pas les répéter.
    pending: RefCell<HashSet<Key>>,
    failed: RefCell<HashSet<Key>>,
    /// Sources montrées par la vue en attendant leur image.
    slots: RefCell<HashMap<Key, ImageSlot>>,
    /// Absences constatées par `get` : leur arrivée reconstruit la vue.
    rebuild_on: RefCell<HashSet<Key>>,
    next_task: Cell<u64>,
}

impl ImageLoader {
    /// Loader decoding with `decoder` ([`Resources::image_decoder`](crate::application::Resources::image_decoder));
    /// without one, every request shows its placeholder. Defaults: 128 MiB cache,
    /// 448-pixel thumbnails, 12 decodes at a time, transparent placeholder.
    pub fn new(decoder: Option<Arc<dyn ImageDecoder>>) -> Self {
        let transparent = Image::new(1, 1, vec![0; 4]).expect("1×1 image");
        Self(Rc::new(Inner {
            decoder,
            resolve: RefCell::new(None),
            placeholder: RefCell::new(Arc::new(transparent)),
            edge: Cell::new(448),
            max_in_flight: Cell::new(12),
            cache: RefCell::new(ImageCache::new(128 << 20)),
            wanted: RefCell::new(VecDeque::new()),
            in_flight: RefCell::new(HashMap::new()),
            pending: RefCell::new(HashSet::new()),
            failed: RefCell::new(HashSet::new()),
            slots: RefCell::new(HashMap::new()),
            rebuild_on: RefCell::new(HashSet::new()),
            next_task: Cell::new(0),
        }))
    }
    /// Memory budget of the cache, in bytes of decoded pixels.
    pub fn budget(self, bytes: usize) -> Self {
        self.0.cache.borrow_mut().set_budget(bytes);
        self
    }
    /// Largest side of a decoded image for [`load`](Self::load) and [`get`](Self::get)
    /// (the other methods take it as a parameter), `1..=2048`.
    pub fn thumbnail_edge(self, pixels: u32) -> Self {
        self.0.edge.set(pixels.clamp(1, 2048));
        self
    }
    /// Decodes running at the same time (at least one).
    pub fn max_in_flight(self, count: usize) -> Self {
        self.0.max_in_flight.set(count.max(1));
        self
    }
    /// Image shown while a source loads, and for a source that cannot be read.
    pub fn placeholder(self, image: Arc<Image>) -> Self {
        *self.0.placeholder.borrow_mut() = image;
        self
    }
    /// Chooses the file read for a source and a size, on the worker thread: a smaller
    /// copy already on disk, a cache folder… Without it, the source is the file path.
    pub fn resolve(self, resolve: impl Fn(&str, u32) -> PathBuf + Send + Sync + 'static) -> Self {
        *self.0.resolve.borrow_mut() = Some(Arc::new(resolve));
        self
    }

    /// Image of `source` for the view: the decoded image if cached, otherwise the
    /// placeholder, replaced (and repainted) when the image arrives.
    pub fn load(&self, source: &str) -> ImageSlot {
        self.load_sized(source, self.0.edge.get())
    }
    /// [`load`](Self::load) decoding to at most `edge` pixels.
    pub fn load_sized(&self, source: &str, edge: u32) -> ImageSlot {
        let placeholder = self.0.placeholder.borrow().clone();
        self.load_or(source, edge, &placeholder)
    }
    /// [`load_sized`](Self::load_sized) with `fallback` instead of the placeholder.
    pub fn load_or(&self, source: &str, edge: u32, fallback: &Arc<Image>) -> ImageSlot {
        let key = (source.to_string(), edge);
        if let Some(image) = self.request(&key) {
            return ImageSlot::ready(image);
        }
        if source.is_empty() || self.0.failed.borrow().contains(&key) {
            return ImageSlot::ready(fallback.clone());
        }
        self.0
            .slots
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| ImageSlot::ready(fallback.clone()))
            .clone()
    }
    /// Decoded image of `source` if cached; otherwise requests it, and its arrival
    /// rebuilds the view (which chose its layout from the absence).
    pub fn get(&self, source: &str) -> Option<Arc<Image>> {
        self.get_sized(source, self.0.edge.get())
    }
    /// [`get`](Self::get) decoding to at most `edge` pixels.
    pub fn get_sized(&self, source: &str, edge: u32) -> Option<Arc<Image>> {
        let key = (source.to_string(), edge);
        let image = self.request(&key);
        if image.is_none() && !source.is_empty() && !self.0.failed.borrow().contains(&key) {
            self.0.rebuild_on.borrow_mut().insert(key);
        }
        image
    }
    /// Decoded image of `source` if cached, without requesting it.
    pub fn peek(&self, source: &str) -> Option<Arc<Image>> {
        self.peek_sized(source, self.0.edge.get())
    }
    /// [`peek`](Self::peek) for images decoded to at most `edge` pixels.
    pub fn peek_sized(&self, source: &str, edge: u32) -> Option<Arc<Image>> {
        self.0.cache.borrow_mut().get(&(source.to_string(), edge))
    }
    /// Requests these sources ahead of the view (data just arrived, next page…).
    pub fn preload<'a>(&self, sources: impl IntoIterator<Item = &'a str>) {
        self.preload_sized(sources, self.0.edge.get());
    }
    /// [`preload`](Self::preload) decoding to at most `edge` pixels.
    pub fn preload_sized<'a>(&self, sources: impl IntoIterator<Item = &'a str>, edge: u32) {
        for source in sources {
            let _ = self.request(&(source.to_string(), edge));
        }
    }
    /// Retries sources that could not be read (their files may have changed).
    pub fn forget_failures(&self) {
        self.0.failed.borrow_mut().clear();
    }

    /// Image en cache, ou demande notée pour un chargement.
    fn request(&self, key: &Key) -> Option<Arc<Image>> {
        if key.0.is_empty() {
            return None;
        }
        if let Some(image) = self.0.cache.borrow_mut().get(key) {
            return Some(image);
        }
        if !self.0.failed.borrow().contains(key) && !self.0.pending.borrow().contains(key) {
            let mut wanted = self.0.wanted.borrow_mut();
            if !wanted.contains(key) {
                wanted.push_back(key.clone());
            }
        }
        None
    }

    /// Tâches des demandes notées depuis le dernier tour, dans la limite des chargements
    /// en vol ; le reste attend une place.
    pub(crate) fn take_commands(&self) -> Vec<TaskCommand> {
        let mut commands = Vec::new();
        let Some(decoder) = self.0.decoder.clone() else {
            self.0.wanted.borrow_mut().clear();
            return commands;
        };
        let resolve = self.0.resolve.borrow().clone();
        let max = self.0.max_in_flight.get();
        while self.0.in_flight.borrow().len() < max {
            let Some(key) = self.0.wanted.borrow_mut().pop_front() else {
                break;
            };
            if !self.0.pending.borrow_mut().insert(key.clone()) {
                continue;
            }
            let id = TaskId(TASK_BIT | self.0.next_task.get());
            self.0
                .next_task
                .set((self.0.next_task.get() + 1) & !TASK_BIT);
            self.0.in_flight.borrow_mut().insert(id, key.clone());
            let (decoder, resolve) = (decoder.clone(), resolve.clone());
            commands.push(TaskCommand::Spawn(Task::new(id, move |cancel| {
                job::load(&key.0, key.1, decoder.as_ref(), resolve.as_ref(), &cancel)
            })));
        }
        commands
    }
    /// `true` si cette tâche est un chargement du chargeur.
    pub(crate) fn owns(&self, id: TaskId) -> bool {
        id.0 & TASK_BIT != 0 && self.0.in_flight.borrow().contains_key(&id)
    }
    /// Résultat d'un chargement : remplit sa source, le met en cache ; `true` si la vue
    /// doit être reconstruite (une absence constatée par `get`).
    pub(crate) fn completed(&self, result: TaskResult) -> bool {
        let Some(key) = self.0.in_flight.borrow_mut().remove(&result.id) else {
            return false;
        };
        self.0.pending.borrow_mut().remove(&key);
        let slot = self.0.slots.borrow_mut().remove(&key);
        let rebuild = self.0.rebuild_on.borrow_mut().remove(&key);
        let image = match result.into_value::<Loaded>() {
            Ok(Loaded(Ok(image))) => image,
            // Fichier illisible, décodage refusé ou tâche perdue : la source garde son
            // image d'attente et n'est plus demandée, comme une image cassée.
            Ok(Loaded(Err(_))) | Err(_) => {
                self.0.failed.borrow_mut().insert(key);
                return false;
            }
        };
        if let Some(slot) = slot {
            slot.set(image.clone());
        }
        if self
            .0
            .cache
            .borrow_mut()
            .insert(key.clone(), image)
            .is_err()
        {
            self.0.failed.borrow_mut().insert(key);
            return false;
        }
        rebuild
    }
    /// L'application a annulé toutes les tâches de la fenêtre : les chargements en vol
    /// sont redemandés.
    pub(crate) fn cancelled_all(&self) {
        let in_flight = std::mem::take(&mut *self.0.in_flight.borrow_mut());
        let mut wanted = self.0.wanted.borrow_mut();
        let mut pending = self.0.pending.borrow_mut();
        for (_, key) in in_flight {
            pending.remove(&key);
            wanted.push_front(key);
        }
    }
}
