//! The [`Application`] trait implemented by every window's application, and the
//! services and context it receives from the runtime.
use crate::ui::{element::Element, scene::Image, theme::Theme};
use std::sync::Arc;
mod message;
pub use message::Action;
pub(crate) use message::{Handler, Message};

/// Native services available while loading, without exposing OS handles.
///
/// Passed to [`Application::load`]; implemented by the platform backend.
pub trait Resources {
    /// Decode PNG or JPEG bytes into a drawable image, on the UI thread.
    ///
    /// # Errors
    ///
    /// [`Error::Image`](crate::Error::Image) when the bytes are not a supported image or
    /// exceed the decoder's limits, or [`Error::Platform`](crate::Error::Platform) when the
    /// native decoder fails.
    fn decode_image(&self, bytes: &[u8]) -> crate::Result<Arc<Image>>;
    /// Decoder that can be moved to background tasks, without moving a native handle.
    /// Returns `None` when the backend has none (the default).
    fn image_decoder(&self) -> Option<Arc<dyn crate::resources::ImageDecoder>> {
        None
    }
}

/// Environment passed to [`Application::view`].
#[derive(Clone, Copy, PartialEq)]
pub struct ViewContext {
    /// Width available to the view, in logical pixels.
    pub width: f32,
    /// Height available to the view (below the title bar, if any), in logical pixels.
    pub height: f32,
    /// Active theme colours.
    pub theme: Theme,
    /// Whether the window has a transparent background.
    pub transparent: bool,
    /// Label of the active renderer, for diagnostics.
    pub backend: &'static str,
    /// Physical pixels per logical pixel (`1.5` at 150 %). Border widths snap to whole
    /// physical pixels, as in browsers; `1.0` when unknown.
    pub scale: f32,
}

/// A window's application: it owns its data, describes it with [`view`](Self::view)
/// and changes it in [`update`](Self::update).
///
/// Widgets receive the messages to emit when they are built:
///
/// ```
/// use karbon_ui::ui::prelude::*;
///
/// #[derive(Clone)]
/// enum Msg {
///     NameChanged(String),
///     Save,
/// }
/// struct App {
///     name: String,
/// }
/// impl Application for App {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         column("form", [
///             input("name", self.name.as_str(), Msg::NameChanged),
///             button("save", "Save", Msg::Save),
///         ])
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         match message {
///             Msg::NameChanged(name) => self.name = name,
///             Msg::Save => {}
///         }
///         true
///     }
/// }
/// ```
pub trait Application: 'static {
    /// Messages emitted by the elements of [`view`](Self::view). Every handler in the
    /// view must produce this type; another type is reported as a programming error.
    type Message: 'static;
    /// Fonts to register with the renderer before the first view. Defaults to none.
    fn fonts(&self) -> &[crate::ui::font::FontAsset] {
        &[]
    }
    /// Called once before the first view, with the backend's native services (for
    /// example to decode images). Defaults to doing nothing.
    ///
    /// # Errors
    ///
    /// Any error aborts opening the window and is returned by
    /// [`platform::run`](crate::platform::run) or
    /// [`platform::run_windows`](crate::platform::run_windows). Library and I/O errors
    /// propagate with `?`; wrap the application's own errors with
    /// [`Error::application`](crate::Error::application):
    ///
    /// ```
    /// use karbon_ui::{Error, application::Resources, ui::prelude::*};
    /// # use std::sync::Arc;
    ///
    /// struct Viewer {
    ///     zoom: f32,
    ///     cover: Option<Arc<karbon_ui::ui::scene::Image>>,
    /// }
    /// impl Application for Viewer {
    ///     type Message = ();
    ///     fn load(&mut self, resources: &dyn Resources) -> karbon_ui::Result<()> {
    ///         // Erreur applicative : enveloppée explicitement.
    ///         self.zoom = "1.5".parse::<f32>().map_err(Error::application)?;
    ///         // Erreurs de la bibliothèque et d'E/S : propagées telles quelles.
    ///         let bytes = std::fs::read("cover.png")?;
    ///         self.cover = Some(resources.decode_image(&bytes)?);
    ///         Ok(())
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    ///     # fn update(&mut self, _: ()) -> bool { false }
    /// }
    /// ```
    fn load(&mut self, _resources: &dyn Resources) -> crate::Result<()> {
        Ok(())
    }
    /// Describe the current state. Called again after `update` or `task_completed`
    /// return `true`.
    fn view(&self, context: &ViewContext) -> Element;
    /// Apply a message emitted by the view; return `true` to rebuild the view.
    fn update(&mut self, message: Self::Message) -> bool;
    /// Drain the pending background work requests
    /// ([`TaskCommand`](crate::task::TaskCommand)s); the runtime polls this after
    /// handling events. Defaults to none.
    fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand> {
        Vec::new()
    }
    /// Receive the result of a background task, on the UI thread; return `true` to
    /// rebuild the view.
    fn task_completed(&mut self, _result: crate::task::TaskResult) -> bool {
        false
    }
    /// Return and drain the pending window requests; they are executed outside the UI
    /// callback.
    fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand> {
        Vec::new()
    }
    /// Receive the window's geometry and state: once when it is shown, then after each
    /// change of position, size, scale, maximized, minimized or full-screen state, and
    /// at the start and end of a move or resize with the pointer. Return `true` to
    /// rebuild the view. Defaults to ignoring them.
    ///
    /// ```
    /// use karbon_ui::{ui::prelude::*, window::WindowState};
    ///
    /// #[derive(Default)]
    /// struct Player {
    ///     window: WindowState,
    /// }
    /// impl Application for Player {
    ///     type Message = ();
    ///     fn window_changed(&mut self, state: WindowState) -> bool {
    ///         // Une poignée de redimensionnement relâchée : la taille est retenue.
    ///         let released = self.window.dragging && !state.dragging;
    ///         self.window = state;
    ///         released
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    ///     # fn update(&mut self, _: ()) -> bool { false }
    /// }
    /// ```
    fn window_changed(&mut self, _state: crate::window::WindowState) -> bool {
        false
    }
}

/// Façade objet d'une application : le runtime et les fenêtres ignorent son type de message.
pub(crate) trait DynApplication {
    fn fonts(&self) -> &[crate::ui::font::FontAsset];
    fn load(&mut self, resources: &dyn Resources) -> crate::Result<()>;
    fn view(&self, context: &ViewContext) -> Element;
    fn update(&mut self, message: Message) -> bool;
    fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand>;
    fn task_completed(&mut self, result: crate::task::TaskResult) -> bool;
    fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand>;
    fn window_changed(&mut self, state: crate::window::WindowState) -> bool;
    /// Accès typé pour le harnais headless, qui connaît le type concret.
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
impl<A: Application> DynApplication for A {
    fn fonts(&self) -> &[crate::ui::font::FontAsset] {
        Application::fonts(self)
    }
    fn load(&mut self, resources: &dyn Resources) -> crate::Result<()> {
        Application::load(self, resources)
    }
    fn view(&self, context: &ViewContext) -> Element {
        Application::view(self, context)
    }
    fn update(&mut self, message: Message) -> bool {
        // Comportements Bootstrap (`data-bs-toggle`…) : appliqués par le runtime, la vue
        // est reconstruite avec le nouvel état.
        if std::any::TypeId::of::<A::Message>()
            != std::any::TypeId::of::<crate::ui::bootstrap::BootstrapAction>()
            && message.is::<crate::ui::bootstrap::BootstrapAction>()
        {
            if let Ok(action) = message.downcast::<crate::ui::bootstrap::BootstrapAction>() {
                crate::ui::bootstrap::apply(&action);
            }
            return true;
        }
        match message.downcast::<A::Message>() {
            Ok(message) => Application::update(self, message),
            // Erreur de programmation : la perdre silencieusement masquerait un contrôle inerte.
            Err(emitted) => panic!(
                "karbon-ui: an element emitted a message of type `{emitted}`, but the \
                 application `{}` expects `{}`; every handler in its view must produce \
                 `Application::Message`",
                std::any::type_name::<A>(),
                std::any::type_name::<A::Message>(),
            ),
        }
    }
    fn take_task_commands(&mut self) -> Vec<crate::task::TaskCommand> {
        Application::take_task_commands(self)
    }
    fn task_completed(&mut self, result: crate::task::TaskResult) -> bool {
        Application::task_completed(self, result)
    }
    fn take_window_commands(&mut self) -> Vec<crate::window::WindowCommand> {
        Application::take_window_commands(self)
    }
    fn window_changed(&mut self, state: crate::window::WindowState) -> bool {
        Application::window_changed(self, state)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
