use super::*;
pub(super) use crate::ui::{element::Element, scene::Image, theme::Theme};
pub(super) use std::sync::Arc;

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
    /// Colour preferences matched by the `prefers-color-scheme` and `prefers-contrast`
    /// media features, read before each frame. When they change, the whole view is built
    /// again, memoized subtrees included. Defaults to the system scheme and normal
    /// contrast.
    fn color_preferences(&self) -> ColorPreferences {
        ColorPreferences::default()
    }
    /// `true` while the application has work spread over several frames, such as a batch
    /// of rows to add a few at a time: [`frame_work`](Self::frame_work) is then called
    /// about once per frame. Defaults to none.
    fn has_frame_work(&self) -> bool {
        false
    }
    /// Do one slice of the work announced by [`has_frame_work`](Self::has_frame_work);
    /// return `true` to rebuild the view. A view that is too long to build in one frame
    /// makes the screen miss a refresh: built in slices, it does not.
    ///
    /// ```
    /// use karbon_ui::ui::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct Rows {
    ///     shown: usize,
    ///     waiting: usize,
    /// }
    /// impl Application for Rows {
    ///     type Message = ();
    ///     fn has_frame_work(&self) -> bool {
    ///         self.waiting > 0
    ///     }
    ///     fn frame_work(&mut self) -> bool {
    ///         let slice = self.waiting.min(20);
    ///         self.waiting -= slice;
    ///         self.shown += slice;
    ///         true
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    ///     # fn update(&mut self, _: ()) -> bool { false }
    /// }
    /// ```
    fn frame_work(&mut self) -> bool {
        false
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
    ///         // A resize handle was released: the size is kept.
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
    /// Receive a click or a menu choice on the window's
    /// [`TrayIcon`](crate::window::TrayIcon), set with
    /// [`WindowCommand::Tray`](crate::window::WindowCommand::Tray); delivered even while
    /// the window is hidden. Return `true` to rebuild the view. Defaults to ignoring it.
    ///
    /// ```
    /// use karbon_ui::{
    ///     ui::prelude::*,
    ///     window::{TrayEvent, WindowChange, WindowCommand, WindowId},
    /// };
    ///
    /// #[derive(Default)]
    /// struct Player {
    ///     commands: Vec<WindowCommand>,
    /// }
    /// impl Application for Player {
    ///     type Message = ();
    ///     fn tray_event(&mut self, event: TrayEvent) -> bool {
    ///         let command = match event {
    ///             TrayEvent::Activated { .. } | TrayEvent::MenuItem(1) => {
    ///                 WindowCommand::Change(WindowId::MAIN, WindowChange::Focus)
    ///             }
    ///             _ => WindowCommand::Exit,
    ///         };
    ///         self.commands.push(command);
    ///         false
    ///     }
    ///     fn take_window_commands(&mut self) -> Vec<WindowCommand> {
    ///         std::mem::take(&mut self.commands)
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    ///     # fn update(&mut self, _: ()) -> bool { false }
    /// }
    /// ```
    fn tray_event(&mut self, _event: crate::window::TrayEvent) -> bool {
        false
    }
    /// The application's [`ImageLoader`](crate::resources::ImageLoader), if it has one:
    /// the runtime then schedules its background decodes and delivers their results
    /// itself, outside [`take_task_commands`](Self::take_task_commands) and
    /// [`task_completed`](Self::task_completed). Defaults to none.
    ///
    /// ```
    /// use karbon_ui::{application::Resources, resources::ImageLoader, ui::prelude::*};
    ///
    /// #[derive(Default)]
    /// struct Gallery {
    ///     images: Option<ImageLoader>,
    /// }
    /// impl Application for Gallery {
    ///     type Message = ();
    ///     fn load(&mut self, resources: &dyn Resources) -> karbon_ui::Result<()> {
    ///         self.images = Some(ImageLoader::new(resources.image_decoder()));
    ///         Ok(())
    ///     }
    ///     fn image_loader(&self) -> Option<ImageLoader> {
    ///         self.images.clone()
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    ///     # fn update(&mut self, _: ()) -> bool { false }
    /// }
    /// ```
    fn image_loader(&self) -> Option<crate::resources::ImageLoader> {
        None
    }
    /// The application's [`Inbox`](crate::task::Inbox), if it has one: the window then
    /// wakes up when another thread posts to it, and applies the posts on the UI thread
    /// (the view is rebuilt when one of them returns `true`). Defaults to none.
    ///
    /// ```
    /// use karbon_ui::{task::Inbox, ui::prelude::*};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Downloaded(u32),
    /// }
    /// struct Downloads {
    ///     inbox: Inbox<Downloads>,
    ///     done: u32,
    /// }
    /// impl Application for Downloads {
    ///     type Message = Msg;
    ///     fn inbox(&self) -> Option<Inbox<Self>> {
    ///         Some(self.inbox.clone())
    ///     }
    ///     fn update(&mut self, message: Msg) -> bool {
    ///         let Msg::Downloaded(n) = message;
    ///         self.done += n;
    ///         true
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    /// }
    ///
    /// let app = Downloads { inbox: Inbox::new(), done: 0 };
    /// let sender = app.inbox.sender();
    /// std::thread::spawn(move || sender.send(Msg::Downloaded(1)));
    /// ```
    fn inbox(&self) -> Option<crate::task::Inbox<Self>>
    where
        Self: Sized,
    {
        None
    }
    /// A key pressed or released while no text field has the keyboard
    /// ([`KeyPress`](crate::ui::runtime::KeyPress)): every key of the keyboard, named
    /// after what the layout prints on it and with its place, with Shift, Ctrl and Alt.
    /// Shortcuts of a tool or a game (Ctrl+Z, a letter per tool) and keys held to move.
    /// Return `true` to rebuild the view. Defaults to
    /// ignoring the keys.
    ///
    /// The key keeps its usual effect on the interface: Tab moves the focus, Enter
    /// activates the focused control, Escape closes a menu. Not delivered on macOS and
    /// Linux yet.
    fn key(&mut self, press: crate::ui::runtime::KeyPress) -> bool {
        let _ = press;
        false
    }
    /// The user asks to close the window (the close button of the title bar, Alt+F4,
    /// the taskbar): return `false` to keep it open, for example to ask first whether to
    /// save the changes. The view is then rebuilt, so the application can show its
    /// question; it closes the window later with
    /// [`WindowCommand::Close`](crate::window::WindowCommand::Close) or
    /// [`WindowCommand::Exit`](crate::window::WindowCommand::Exit), which do not ask
    /// again. Defaults to `true`: the window closes as before. Asked on Windows only for
    /// now; macOS and Linux close without asking.
    ///
    /// ```
    /// use karbon_ui::{ui::prelude::*, window::{WindowCommand, WindowId}};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Discard,
    /// }
    /// #[derive(Default)]
    /// struct Editor {
    ///     unsaved: bool,
    ///     asking: bool,
    ///     commands: Vec<WindowCommand>,
    /// }
    /// impl Application for Editor {
    ///     type Message = Msg;
    ///     fn close_requested(&mut self) -> bool {
    ///         self.asking = self.unsaved;
    ///         !self.unsaved
    ///     }
    ///     fn update(&mut self, message: Msg) -> bool {
    ///         let Msg::Discard = message;
    ///         self.commands.push(WindowCommand::Close(WindowId::MAIN));
    ///         false
    ///     }
    ///     fn take_window_commands(&mut self) -> Vec<WindowCommand> {
    ///         std::mem::take(&mut self.commands)
    ///     }
    ///     # fn view(&self, _: &ViewContext) -> Element { text("t", "") }
    /// }
    /// ```
    fn close_requested(&mut self) -> bool {
        true
    }
    /// A zoom of the whole interface chosen by the application (a setting « Taille de
    /// l'interface »), multiplied by the scale of the display: at `1.25` on a 150 %
    /// display, everything is drawn 1.875 times larger, and the view gets
    /// [`ViewContext::width`] and [`ViewContext::height`] smaller and
    /// [`ViewContext::scale`] larger in proportion. Read after each change of the
    /// application; kept between 0.5 and 3. Defaults to `1.0`: the display scale alone,
    /// as before. Applied on Windows and by the headless harness for now.
    fn interface_zoom(&self) -> f32 {
        1.
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
    fn color_preferences(&self) -> ColorPreferences;
    fn has_frame_work(&self) -> bool;
    fn frame_work(&mut self) -> bool;
    fn window_changed(&mut self, state: crate::window::WindowState) -> bool;
    fn tray_event(&mut self, event: crate::window::TrayEvent) -> bool;
    fn image_loader(&self) -> Option<crate::resources::ImageLoader>;
    fn inbox(&self) -> Option<Arc<dyn crate::task::InboxPort>>;
    fn key(&mut self, press: crate::ui::runtime::KeyPress) -> bool;
    fn close_requested(&mut self) -> bool;
    fn interface_zoom(&self) -> f32;
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
        if message.ignored() {
            return false;
        }
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
    fn color_preferences(&self) -> ColorPreferences {
        Application::color_preferences(self)
    }
    fn has_frame_work(&self) -> bool {
        Application::has_frame_work(self)
    }
    fn frame_work(&mut self) -> bool {
        Application::frame_work(self)
    }
    fn window_changed(&mut self, state: crate::window::WindowState) -> bool {
        Application::window_changed(self, state)
    }
    fn tray_event(&mut self, event: crate::window::TrayEvent) -> bool {
        Application::tray_event(self, event)
    }
    fn image_loader(&self) -> Option<crate::resources::ImageLoader> {
        Application::image_loader(self)
    }
    fn inbox(&self) -> Option<Arc<dyn crate::task::InboxPort>> {
        Application::inbox(self).map(|inbox| inbox.port())
    }
    fn key(&mut self, press: crate::ui::runtime::KeyPress) -> bool {
        Application::key(self, press)
    }
    fn close_requested(&mut self) -> bool {
        Application::close_requested(self)
    }
    fn interface_zoom(&self) -> f32 {
        Application::interface_zoom(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
