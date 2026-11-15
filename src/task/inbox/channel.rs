//! Boîte aux lettres de l'application : n'importe quel thread y dépose une fermeture (ou
//! un message), la fenêtre est réveillée et l'applique sur le thread de l'interface.
//!
//! La fenêtre s'abonne par une tâche spéciale (`Task::listen`) qui n'occupe aucun
//! ouvrier : le `TaskScope` remet un `Port`, qui dépose un faux résultat de tâche
//! (`INBOX_TASK`) et réveille la fenêtre comme un vrai. Un seul réveil est en route à la
//! fois, quel que soit le nombre de dépôts.
use super::super::{Port, TaskId};
use std::{
    any::Any,
    collections::VecDeque,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Identifiant réservé des réveils de la boîte aux lettres.
pub(crate) const INBOX_TASK: TaskId = TaskId(u64::MAX);

type Post<A> = Box<dyn FnOnce(&mut A) -> bool + Send>;

/// Mailbox of an application: any thread posts work to it with an [`InboxSender`], and
/// the window wakes up and applies it to the application on the UI thread, in the order
/// posted. Return it from
/// [`Application::inbox`](crate::application::Application::inbox).
///
/// It connects work that karbon-ui does not run (an async runtime, a thread of the
/// application, a device or network callback) without polling: no worker thread waits
/// for it, and a burst of posts wakes the window once.
///
/// ```
/// use karbon_ui::{task::Inbox, ui::prelude::*};
///
/// struct Player {
///     inbox: Inbox<Player>,
///     position: f64,
/// }
/// impl Application for Player {
///     type Message = ();
///     fn inbox(&self) -> Option<Inbox<Self>> {
///         Some(self.inbox.clone())
///     }
///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
///     # fn update(&mut self, _: ()) -> bool { false }
/// }
///
/// let player = Player { inbox: Inbox::new(), position: 0. };
/// let sender = player.inbox.sender();
/// std::thread::spawn(move || {
///     // Decoder thread: the position reaches the application on the UI thread.
///     sender.post(|player| {
///         player.position = 12.5;
///         true // rebuild the view
///     });
/// });
/// ```
pub struct Inbox<A> {
    shared: Arc<Shared<A>>,
}

/// Handle to post to an [`Inbox`] from any thread; cheap to clone.
pub struct InboxSender<A> {
    shared: Arc<Shared<A>>,
}

struct Shared<A> {
    state: Mutex<State<A>>,
    /// Signalée à chaque dépôt, pour [`Inbox::wait`].
    ready: Condvar,
    /// Poignées `Inbox` vivantes : sans elles, plus personne ne lit.
    readers: AtomicUsize,
}

struct State<A> {
    posts: VecDeque<Post<A>>,
    /// La fenêtre qui écoute, s'il y en a une.
    port: Option<Port>,
    /// Un réveil est en route et pas encore livré.
    signalled: bool,
    /// Un abonnement est demandé et pas encore remis par la fenêtre.
    attaching: bool,
}

impl<A> Shared<A> {
    fn state(&self) -> MutexGuard<'_, State<A>> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl<A: 'static> Inbox<A> {
    /// Empty mailbox, not yet listened to by a window.
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    posts: VecDeque::new(),
                    port: None,
                    signalled: false,
                    attaching: false,
                }),
                ready: Condvar::new(),
                readers: AtomicUsize::new(1),
            }),
        }
    }
    /// A handle to post to this mailbox from any thread.
    pub fn sender(&self) -> InboxSender<A> {
        InboxSender {
            shared: self.shared.clone(),
        }
    }
    /// Applies the pending posts to `app` now, in order; returns `true` if one of them
    /// asked for a rebuild. The window does this by itself: call it where no window runs
    /// (tests, a headless preview).
    pub fn deliver(&self, app: &mut A) -> bool {
        let posts = {
            let mut state = self.shared.state();
            state.signalled = false;
            std::mem::take(&mut state.posts)
        };
        let mut changed = false;
        for post in posts {
            changed |= post(app);
        }
        changed
    }
    /// Waits until something is posted, `timeout` at most; returns `true` if posts are
    /// pending. For code that runs without a window (a headless preview waiting for
    /// its data); a window never needs it.
    pub fn wait(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut state = self.shared.state();
        while state.posts.is_empty() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            state = self
                .shared
                .ready
                .wait_timeout(state, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        true
    }
    /// Number of posts waiting to be applied.
    pub fn len(&self) -> usize {
        self.shared.state().posts.len()
    }
    /// `true` when nothing waits to be applied.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Accès du runtime, sans le type de l'application.
    pub(crate) fn port(&self) -> Arc<dyn InboxPort> {
        self.shared.clone()
    }
}

impl<A: 'static> Default for Inbox<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> Clone for Inbox<A> {
    fn clone(&self) -> Self {
        self.shared.readers.fetch_add(1, Ordering::Relaxed);
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<A> Drop for Inbox<A> {
    fn drop(&mut self) {
        self.shared.readers.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<A> InboxSender<A> {
    /// Posts `apply`, run on the UI thread with the application; it returns `true` to
    /// rebuild the view. Returns `false` (and drops `apply`) once every [`Inbox`]
    /// handle is gone: the application has ended, a loop that feeds it can stop.
    pub fn post(&self, apply: impl FnOnce(&mut A) -> bool + Send + 'static) -> bool {
        if self.shared.readers.load(Ordering::Relaxed) == 0 {
            return false;
        }
        let port = {
            let mut state = self.shared.state();
            state.posts.push_back(Box::new(apply));
            self.shared.ready.notify_all();
            let port = state.port.clone().filter(|_| !state.signalled);
            if port.is_some() {
                state.signalled = true;
            }
            port
        };
        // Réveil hors du verrou : la fenêtre peut livrer aussitôt.
        if let Some(port) = port
            && !port.signal()
        {
            let mut state = self.shared.state();
            state.signalled = false;
            state.port = None;
        }
        true
    }
    /// `true` once every [`Inbox`] handle is gone.
    pub fn is_closed(&self) -> bool {
        self.shared.readers.load(Ordering::Relaxed) == 0
    }
}

impl<A: crate::application::Application> InboxSender<A>
where
    A::Message: Send,
{
    /// Posts a message, handled by
    /// [`Application::update`](crate::application::Application::update) as if an element
    /// of the view had emitted it. Returns `false` once the application has ended.
    pub fn send(&self, message: A::Message) -> bool {
        self.post(move |app| app.update(message))
    }
}

impl<A> Clone for InboxSender<A> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

/// Abonnement d'une boîte aux lettres à la fenêtre (voir `Task::listen`).
pub(crate) trait Listener: Send + Sync {
    /// La fenêtre écoute désormais par `port`.
    fn attach(&self, port: Port);
}

/// Ce que le runtime voit d'une boîte aux lettres, sans le type de l'application.
pub(crate) trait InboxPort: Listener {
    /// `true` s'il faut (ré)abonner la fenêtre : jamais abonnée, abonnement perdu
    /// (fenêtre ou portée de tâches remplacée, tâches annulées), ou `force`.
    fn wants_listener(&self, force: bool) -> bool;
    /// Applique les dépôts à l'application (de type `A`) ; `true` pour reconstruire.
    fn deliver_any(&self, app: &mut dyn Any) -> bool;
    /// La même boîte, comme abonné d'une tâche.
    fn listener(self: Arc<Self>) -> Arc<dyn Listener>;
}

impl<A: 'static> Listener for Shared<A> {
    fn attach(&self, port: Port) {
        let signal = {
            let mut state = self.state();
            state.port = Some(port.clone());
            state.attaching = false;
            // Un réveil d'avant (fenêtre précédente, tâches annulées) ne viendra plus.
            state.signalled = !state.posts.is_empty();
            state.signalled
        };
        if signal && !port.signal() {
            let mut state = self.state();
            state.signalled = false;
            state.port = None;
        }
    }
}

impl<A: 'static> InboxPort for Shared<A> {
    fn wants_listener(&self, force: bool) -> bool {
        let mut state = self.state();
        if !force && (state.attaching || state.port.as_ref().is_some_and(Port::is_live)) {
            return false;
        }
        state.attaching = true;
        true
    }
    fn deliver_any(&self, app: &mut dyn Any) -> bool {
        let Some(app) = app.downcast_mut::<A>() else {
            return false;
        };
        let posts = {
            let mut state = self.state();
            state.signalled = false;
            std::mem::take(&mut state.posts)
        };
        let mut changed = false;
        for post in posts {
            changed |= post(app);
        }
        changed
    }
    fn listener(self: Arc<Self>) -> Arc<dyn Listener> {
        self
    }
}
