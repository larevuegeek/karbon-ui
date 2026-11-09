//! UI runtime: element identities, focus, pointer capture, transient state and invalidation
//! of the UI tree. Platform backends drive it through [`UiRuntime`].
use crate::application::{
    Action, Application, DynApplication, Handler, Message, Resources, ViewContext,
};
use crate::ui::core::hash::FastSet;
use crate::ui::{
    element::*,
    layers::{self, LayerFocus},
    layout, modal,
    popover::{self, ContextMenuRequest, PopoverKind},
    scene::{DrawCommand, PointF, RectF, Scene},
    scroll,
    select::{SelectInfo, SelectResponse, Selects},
    slider::{RangeInfo, SliderResponse, Sliders},
    text_input::{
        AccessibleText, ClipboardRequest, EditCommand, EditMenu, InputSnapshot, MenuAnchor,
        MenuResponse, TextInputs,
    },
    theme::Color,
    title_bar::*,
    tooltip::{TooltipInfo, Tooltips},
    virtual_list,
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

mod accessibility;
mod activation;
mod anchoring;
mod application;
mod components;
pub mod drag_drop;
mod events;
mod focus;
mod hit_test;
mod hover;
mod images;
mod interaction;
mod invalidation;
mod overlays;
mod painting;
mod prepare;
mod rebuild;
mod responses;
mod restyle;
mod scroll_probe;
mod scrolling;
mod smooth_scroll;
mod split;
mod text_cache;
mod text_input;
mod tooltip;
mod tree;
mod virtualization;
mod wheel;
pub use accessibility::{AccessibilityAction, AccessibilityError, AccessibleElement};
use components::{ComponentStates, Drag, ScrollDrag, SyncContext};
pub use events::{Cursor, EventResult, Key, UiEvent};
use interaction::Interaction;
use invalidation::{Invalidation, StyleSignature};
use painting::{paint_chrome, paint_node};
use tree::visit_tree;
pub use wheel::WheelEvent;
pub(crate) use wheel::WheelHandler;
/// Counters of the work done by a [`UiRuntime`], for diagnostics and benchmarks.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct RuntimeStats {
    /// Number of times the application view was rebuilt.
    pub views: u64,
    /// Number of layout passes.
    pub layouts: u64,
    /// Number of scenes painted.
    pub paints: u64,
    /// Number of scroll offset changes.
    pub scrolls: u64,
    /// Rows currently materialized by virtual lists.
    pub virtual_rows: usize,
    /// Cumulative time spent building the view (application `view`, validation,
    /// indexing, state carried over from the previous tree).
    pub view_time: std::time::Duration,
    /// Cumulative time spent resolving styles.
    pub style_time: std::time::Duration,
    /// Style resolutions limited to the elements whose pointer state or animation changed.
    pub partial_styles: u64,
    /// Cumulative time spent in layout (measure and arrange).
    pub layout_time: std::time::Duration,
    /// Cumulative time spent painting the scene.
    pub paint_time: std::time::Duration,
}
/// State of the window chrome (custom title bar) supplied by the backend on each frame.
#[derive(Clone, Copy, PartialEq)]
pub struct ChromeState {
    /// The custom title bar is drawn and reserves space at the top of the window.
    pub visible: bool,
    /// Caption button under the pointer.
    pub hovered: Option<CaptionButton>,
    /// Caption button being pressed.
    pub pressed: Option<CaptionButton>,
    /// The window is maximized (changes the maximize/restore glyph).
    pub maximized: bool,
}
impl Default for ChromeState {
    fn default() -> Self {
        Self {
            visible: true,
            hovered: None,
            pressed: None,
            maximized: false,
        }
    }
}
#[derive(Clone, Copy)]
enum Hit {
    Button(ElementId),
    Divider(ElementId, Axis),
    ScrollThumb(ElementId),
    ScrollTrack(ElementId),
    ScrollArea(ElementId),
    Input(ElementId),
    Slider(ElementId),
    Select(ElementId),
}
impl Hit {
    fn id(self) -> ElementId {
        match self {
            Self::Button(id)
            | Self::Divider(id, _)
            | Self::ScrollThumb(id)
            | Self::ScrollTrack(id)
            | Self::ScrollArea(id) => id,
            Self::Input(id) | Self::Slider(id) | Self::Select(id) => id,
        }
    }
}
#[derive(Clone, Copy)]
struct PaintState {
    hovered: Option<ElementId>,
    pressed: Option<ElementId>,
    focused: Option<ElementId>,
    keyboard_pressed: Option<ElementId>,
    focus_visible: bool,
}
/// Runtime of one window: owns the application, its current view tree and all transient UI
/// state, and turns input events into messages and frames into [`Scene`]s.
///
/// A backend calls [`event`](Self::event) for each input, then
/// [`prepare`](Self::prepare) and paints [`scene`](Self::scene) when a repaint is needed.
pub struct UiRuntime {
    app: Box<dyn DynApplication>,
    root: Option<Element>,
    context: Option<ViewContext>,
    chrome: ChromeState,
    scene: Scene,
    motions: crate::ui::animation::timeline::Timeline,
    /// Pointeur, survol, appui, focus et ordre de tabulation.
    interaction: Interaction,
    /// États transitoires des composants, indexés par élément.
    components: ComponentStates,
    /// Passes à refaire, caches de style et compteurs.
    invalidation: Invalidation,
    /// Redimensionnement interactif de la fenêtre en cours.
    live_resize: bool,
    /// Mesures de texte gardées d'une image à l'autre.
    text_cache: text_cache::TextCache,
}
impl UiRuntime {
    /// Load `app` and prepare its first view; messages emitted by the view are
    /// delivered to its `update` method.
    ///
    /// # Errors
    ///
    /// Any error returned by [`Application::load`].
    pub fn new(app: impl Application, resources: &dyn Resources) -> crate::Result<Self> {
        Self::from_dyn(Box::new(app), resources)
    }

    pub(crate) fn from_dyn(
        mut app: Box<dyn DynApplication>,
        resources: &dyn Resources,
    ) -> crate::Result<Self> {
        app.load(resources)?;
        Ok(Self {
            app,
            root: None,
            context: None,
            chrome: ChromeState::default(),
            scene: Scene::new(Color::rgba(0, 0, 0, 0)),
            motions: Default::default(),
            interaction: Interaction::default(),
            components: ComponentStates::default(),
            invalidation: Invalidation::default(),
            live_resize: false,
            text_cache: Default::default(),
        })
    }

    /// Delays every running animation, transition and exit by `delta`, as if the last
    /// [`prepare`](Self::prepare) had happened `delta` later. A backend that prepares the
    /// first frame before its window exists calls it when the frame is first presented.
    pub fn defer_animations(&mut self, delta: std::time::Duration) {
        self.motions.shift(delta);
    }
    /// Work counters since creation.
    pub fn stats(&self) -> RuntimeStats {
        self.invalidation.stats
    }
    /// `true` if the tree uses state styles (`:hover`, `:focus-visible`…), so the backend must
    /// repaint on pointer movement.
    pub fn has_state_styles(&self) -> bool {
        self.invalidation.dynamic_styles
    }

    /// Scene produced by the last [`prepare`](Self::prepare).
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Current view tree, or `None` before the first [`prepare`](Self::prepare).
    pub fn root(&self) -> Option<&Element> {
        self.root.as_ref()
    }

    /// Element with keyboard focus, if any.
    pub fn focused(&self) -> Option<ElementId> {
        self.interaction.focused
    }

    /// `true` while a pointer gesture (press, drag, scroll thumb…) holds the mouse.
    pub fn has_capture(&self) -> bool {
        self.components.captured() || self.interaction.captured()
    }

    /// Geste pointeur ou appui clavier qu'Échap annule avant toute fermeture.
    fn gesture_active(&self) -> bool {
        self.components.splits.drag.is_some()
            || self.components.scrolls.drag.is_some()
            || self.interaction.pressed.is_some()
            || self.interaction.keyboard_pressed.is_some()
    }

    /// Livrer un message à l'application ; une reconstruction demandée invalide la vue.
    fn send(&mut self, message: Message) -> bool {
        let rebuild = self.app.update(message);
        if rebuild {
            self.invalidate();
        }
        rebuild
    }

    /// Force a view rebuild, relayout and repaint on the next [`prepare`](Self::prepare).
    pub fn invalidate(&mut self) {
        self.invalidation.all();
    }
    /// Des couleurs ou des longueurs s'animent : la cascade est relancée à chaque image.
    #[cfg(test)]
    pub(crate) fn animating_styles(&self) -> bool {
        self.motions.animating_styles()
    }
    /// Tells the runtime whether the system uses a dark colour scheme, for
    /// [`ColorScheme::System`](crate::application::ColorScheme::System).
    pub fn set_system_dark(&mut self, dark: bool) {
        self.invalidation.system_dark = dark;
    }

    /// When `reduced` is `true`, disable animation durations and apply final states immediately.
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        if self.motions.reduced != reduced {
            self.motions.reduced = reduced;
            self.invalidation.repaint = true;
            // `prefers-reduced-motion` des feuilles de styles.
            self.invalidation.style_signature = None;
            self.invalidation.relayout = true;
        }
    }
    /// When `false`, every [`prepare`](Self::prepare) recomputes styles, measures and
    /// layout from scratch, without the engine's incremental caches (`true` by default).
    /// The result must be identical: meant for tests and diagnostics that compare both.
    pub fn set_incremental(&mut self, incremental: bool) {
        self.invalidation.incremental = incremental;
        self.invalidation.relayout = true;
    }
    /// Declare that the user is resizing the window interactively. While it lasts, blurred
    /// shadows of elements that only change size are stretched instead of re-rasterized
    /// at every step; ending it repaints them exactly.
    pub fn set_live_resize(&mut self, live: bool) {
        if self.live_resize != live {
            self.live_resize = live;
            self.invalidation.repaint = true;
            // Fin du geste : requêtes de conteneur et ombres exactes.
            self.invalidation.relayout |= !live;
        }
    }
}
