pub(super) use super::super::{Headless, HeadlessRenderer};
pub(super) use crate::{
    application::{Application, ViewContext},
    platform::{Backend, Offscreen},
    task::TaskExecutor,
    ui::{
        runtime::{ChromeState, EventResult, UiEvent, UiRuntime},
        text_input::{ClipboardRequest, EditCommand},
        theme::Theme,
        title_bar::TITLE_BAR_HEIGHT,
    },
    window::{RenderMode, WindowCommand},
};
pub(super) use std::{marker::PhantomData, time::Instant};

/// Size, theme and chrome of the simulated window of a [`HeadlessApp`].
#[derive(Clone, Copy)]
pub struct HeadlessOptions {
    /// Window width in logical pixels.
    pub width: f32,
    /// Window height in logical pixels, title bar included.
    pub height: f32,
    /// Theme passed to [`Application::view`].
    pub theme: Theme,
    /// Reserve and draw the built-in title bar (`false` by default, so that element
    /// bounds start at the top of the window).
    pub title_bar: bool,
    /// Value of [`ViewContext::transparent`].
    pub transparent: bool,
    /// Disable animations and transitions.
    pub reduced_motion: bool,
    /// Physical pixels per logical pixel of the simulated display
    /// ([`ViewContext::scale`]), `1.0` by default. Border widths and the painted boxes
    /// of framework stylesheets snap to these pixels, as on a real display: pass the
    /// same factor to [`render`](HeadlessApp::render) to see them crisp.
    pub scale: f32,
}
impl Default for HeadlessOptions {
    /// An 800 × 600 window with the dark theme and no title bar.
    fn default() -> Self {
        Self {
            width: 800.,
            height: 600.,
            theme: Theme::DARK,
            title_bar: false,
            transparent: false,
            reduced_motion: false,
            scale: 1.,
        }
    }
}

/// An [`Application`] driven without any window, for tests that run on every target.
///
/// The harness owns the application and a [`UiRuntime`] measuring text with
/// [`HeadlessTextMeasurer`](super::super::HeadlessTextMeasurer). Every input method dispatches
/// the same [`UiEvent`]s a native window would, delivers the resulting messages to
/// [`Application::update`], then rebuilds, lays out and paints the view, so state can be
/// read back immediately.
///
/// - Element identities accept anything convertible to [`ElementId`](crate::ui::element::ElementId): the names given to
///   builders (`"save"`), or raw numbers.
/// - Time is virtual: it starts at the creation instant and only moves with
///   [`advance`](Self::advance), so animations are reproducible. Tooltip delays follow
///   the system clock and are not driven by `advance`.
/// - The clipboard is simulated ([`clipboard`](Self::clipboard),
///   [`set_clipboard`](Self::set_clipboard)).
/// - Background tasks only run when [`run_tasks`](Self::run_tasks) is called, and window
///   requests are collected for [`take_window_commands`](Self::take_window_commands).
///
/// ```
/// use karbon_ui::{platform::headless::HeadlessApp, ui::{prelude::*, runtime::Key}};
///
/// #[derive(Clone)]
/// enum Msg {
///     Name(String),
///     Save,
/// }
/// #[derive(Default)]
/// struct Form {
///     name: String,
///     saved: Vec<String>,
/// }
/// impl Application for Form {
///     type Message = Msg;
///     fn view(&self, _: &ViewContext) -> Element {
///         column("form", [
///             input("name", self.name.as_str(), Msg::Name),
///             button("save", "Save", Msg::Save),
///         ])
///     }
///     fn update(&mut self, message: Msg) -> bool {
///         match message {
///             Msg::Name(name) => self.name = name,
///             Msg::Save => self.saved.push(std::mem::take(&mut self.name)),
///         }
///         true
///     }
/// }
///
/// let mut app = HeadlessApp::new(Form::default())?;
/// app.click("name")?;
/// app.type_text("Ada")?;
/// app.press(Key::Tab)?;
/// assert_eq!(app.focused(), Some(ElementId::named("save")));
/// app.press(Key::Enter)?;
/// assert_eq!(app.app().saved, ["Ada"]);
/// assert_eq!(app.input_value("name").as_deref(), Some(""));
/// # Ok::<(), karbon_ui::Error>(())
/// ```
pub struct HeadlessApp<A: Application> {
    // Le runtime (et l'application) est détruit avant le moteur et les ouvriers.
    pub(super) ui: UiRuntime,
    pub(super) renderer: HeadlessRenderer,
    pub(super) executor: Option<TaskExecutor>,
    pub(super) options: HeadlessOptions,
    pub(super) now: Instant,
    pub(super) clipboard: Option<String>,
    pub(super) window_commands: Vec<WindowCommand>,
    pub(super) _application: PhantomData<fn() -> A>,
}

impl<A: Application> HeadlessApp<A> {
    /// Load `application` in an 800 × 600 window with the dark theme and build its first
    /// frame.
    ///
    /// # Errors
    ///
    /// Any error returned by [`Application::load`], and the errors of
    /// [`prepare`](Self::prepare).
    pub fn new(application: A) -> crate::Result<Self> {
        Self::with_options(application, HeadlessOptions::default())
    }
    /// Load `application` with explicit window options and build its first frame.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for a negative or non-finite
    /// size or invalid embedded fonts, any error returned by [`Application::load`], and
    /// the errors of [`prepare`](Self::prepare).
    pub fn with_options(application: A, options: HeadlessOptions) -> crate::Result<Self> {
        let mut renderer = Headless::offscreen(RenderMode::Software)?;
        let mut ui = UiRuntime::new(application, renderer.resources())?;
        renderer.load_fonts(ui.font_assets())?;
        ui.set_reduced_motion(options.reduced_motion);
        let mut app = Self {
            ui,
            renderer,
            executor: None,
            options,
            now: Instant::now(),
            clipboard: None,
            window_commands: Vec::new(),
            _application: PhantomData,
        };
        app.prepare()?;
        Ok(app)
    }

    pub(super) fn context(&self) -> crate::Result<ViewContext> {
        let o = self.options;
        let title = if o.title_bar {
            TITLE_BAR_HEIGHT as f32
        } else {
            0.
        };
        if !(o.width.is_finite() && o.height.is_finite() && o.width >= 0. && o.height >= title) {
            return Err(crate::Error::invalid_input("invalid headless window size"));
        }
        if !(o.scale.is_finite() && o.scale > 0.) {
            return Err(crate::Error::invalid_input(
                "invalid headless display scale",
            ));
        }
        Ok(ViewContext {
            width: o.width,
            height: o.height - title,
            theme: o.theme,
            transparent: o.transparent,
            backend: super::super::LABEL,
            scale: o.scale,
        })
    }

    /// Bring the frame up to date: rebuild, lay out and paint whatever was invalidated,
    /// at the current virtual time. Input methods call it themselves; call it after
    /// changing the runtime through [`runtime_mut`](Self::runtime_mut).
    ///
    /// # Errors
    ///
    /// The errors of [`UiRuntime::prepare`], e.g.
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for an invalid view tree.
    pub fn prepare(&mut self) -> crate::Result<()> {
        let context = self.context()?;
        let chrome = ChromeState {
            visible: self.options.title_bar,
            ..Default::default()
        };
        self.ui
            .prepare_at(context, chrome, &mut self.renderer, self.now)
    }

    /// Dispatch a raw input event, answer its clipboard request with the simulated
    /// clipboard, then [`prepare`](Self::prepare) the next frame.
    ///
    /// Positions are logical window coordinates, as returned by [`bounds`](Self::bounds).
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn event(&mut self, event: UiEvent) -> crate::Result<EventResult> {
        // Un événement s'applique toujours à une image à jour, comme dans une fenêtre.
        self.prepare()?;
        let result = self.ui.event(event);
        match &result.clipboard {
            Some(ClipboardRequest::Write { text, cut }) => {
                self.clipboard = Some(text.clone());
                if *cut {
                    self.ui.event(UiEvent::Edit(EditCommand::DeleteSelection));
                }
            }
            Some(ClipboardRequest::Read) => {
                if let Some(text) = self.clipboard.clone() {
                    self.ui.event(UiEvent::Text(text.into()));
                }
            }
            None => {}
        }
        self.prepare()?;
        Ok(result)
    }
}
