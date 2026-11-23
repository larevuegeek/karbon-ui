use super::*;
pub(super) use crate::application::{Action, Handler};
pub(super) use crate::ui::{
    scene::{RectF, Scene},
    theme::{Color, Theme},
};
/// A node of the UI tree: an identity, layout behaviour, style, children and an optional widget.
///
/// Elements are usually created through the free functions of [`crate::ui::view`] or the
/// [`ui!`](crate::ui!) macro, then configured with builder methods. Layout results
/// (bounds, clipping, scroll offsets) are filled in by the runtime.
pub struct Element {
    pub(crate) backgrounds: Option<std::sync::Arc<[crate::ui::rendering::brush::Background]>>,
    pub(crate) rules: Option<Box<LayoutRules>>,
    pub(crate) styles: Option<Box<crate::ui::styling::Styles>>,
    pub(crate) gradient: Option<std::sync::Arc<crate::ui::rendering::brush::LinearGradient>>,
    pub(crate) motion: Option<Box<crate::ui::animation::Motion>>,
    /// Stable identity used to keep interaction state (focus, scroll, animations) across rebuilds.
    pub id: ElementId,
    /// How children are arranged.
    pub layout: Layout,
    /// Box-model and appearance properties.
    pub style: Style,
    /// Child elements, in document order.
    pub children: Vec<Element>,
    /// Optional widget that measures, paints and reacts to input for this element.
    pub widget: Option<Box<dyn Widget>>,
    pub(crate) rect: RectF,
    pub(crate) clip: RectF,
    pub(crate) intrinsic: SizeF,
    pub(crate) divider: Option<RectF>,
    pub(crate) scroll: Option<crate::ui::scroll::ScrollMetrics>,
    pub(crate) scroll_offset: f32,
    // État rare sur le tas : ne pas grossir chaque Text/Button avec celui d'une liste.
    pub(crate) virtual_list: Option<Box<crate::ui::virtual_list::VirtualList>>,
    pub(crate) context_menu: Option<Box<Handler<crate::ui::popover::ContextMenuRequest>>>,
    pub(crate) on_scroll: Option<Box<Handler<crate::ui::scroll::ScrollMetrics>>>,
    /// `onscrollrange` : plage défilable changée ([`Element::on_scroll_range`]).
    pub(crate) on_scroll_range: Option<Box<Handler<crate::ui::scroll::ScrollMetrics>>>,
    /// `onwheel` : gestionnaire de la molette ([`Element::on_wheel`]).
    pub(crate) on_wheel: Option<Box<crate::ui::runtime::WheelHandler>>,
    /// Survol, appui, focus et double clic ([`Element::on_hover`]…).
    pub(crate) listeners: Option<Box<Listeners>>,
    /// Sous-arbre mémorisé ([`Element::memo`]).
    pub(crate) memo: Option<Box<memo::MemoState>>,
    /// Mesures et placement de l'image précédente (layout incrémental).
    pub(crate) layout_cache: Option<Box<memo::LayoutCache>>,
    /// Indicateurs de la passe de layout en cours (`layout::memo`).
    pub(crate) layout_bits: u8,
    /// `content-visibility` : contenu sauté au dernier layout (ni peint ni touché, son
    /// placement n'est plus tenu à jour).
    pub(crate) content_skipped: bool,
    /// Défilement reporté sur le contenu sauté (`layout::scroll::translate`) : position
    /// réelle des descendants = position retenue + ce décalage, jusqu'à ce que le layout
    /// redispose le contenu. Un long défilement ne déplace ainsi que les lignes proches
    /// de la vue, pas toute la liste à chaque image.
    pub(crate) skipped_shift: (f32, f32),
    pub(crate) tooltip: Option<Box<crate::ui::tooltip::Tooltip>>,
    pub(crate) surface_style: Option<std::sync::Arc<crate::ui::style::SurfaceStyle>>,
    pub(crate) drag_source: Option<Box<crate::ui::drag_drop::DragSource>>,
    pub(crate) drop_target: Option<Box<crate::ui::drag_drop::DropTarget>>,
    pub(crate) skip_tab: bool,
    /// `aria-hidden` : sous-arbre absent de l'arbre d'accessibilité.
    pub(crate) semantics_hidden: bool,
    /// `aria-expanded` : état du panneau ou du menu que l'élément ouvre.
    pub(crate) expanded: Option<bool>,
    /// États ARIA déclarés (`aria-pressed`, `aria-checked`, `aria-selected`, `aria-current`).
    pub(crate) aria: aria::AriaStates,
    /// `data-drag-region` (`Some(true)`) : zone qui déplace la fenêtre (hors cibles
    /// interactives) ; `data-no-drag` (`Some(false)`) : sous-arbre exclu de la zone.
    pub(crate) drag_region: Option<bool>,
    /// `data-resize-region` : poignée qui redimensionne la fenêtre par ce bord ou ce coin.
    pub(crate) resize_region: Option<crate::window::ResizeEdge>,
    /// `data-pan-frame` : bloc déplacé par le moteur pendant un `onpan` à l'intérieur.
    pub(crate) pan_frame: bool,
}
/// Gestionnaires des événements d'élément (`runtime/listeners.rs`).
#[derive(Default)]
pub(crate) struct Listeners {
    pub hover: Option<Handler<bool>>,
    pub press: Option<Handler<bool>>,
    pub focus: Option<Handler<bool>>,
    pub double_click: Option<Action>,
    pub pan: Option<Handler<crate::ui::drag_drop::PanEvent>>,
    pub pointer: Option<crate::ui::runtime::raw_input::PointerHandler>,
    pub pointer_wheel: Option<crate::ui::runtime::raw_input::PointerHandler>,
    /// `on_key` : touches données à ce panneau quand il a le focus.
    pub key: Option<crate::ui::runtime::raw_input::KeyHandler>,
    /// `pointer_lock` : le geste de cet élément verrouille le pointeur.
    pub pointer_lock: bool,
    /// `on_file_drop` : fichiers tirés depuis un autre programme.
    pub file_drop: Option<crate::ui::runtime::raw_input::FileDropHandler>,
}
thread_local! {
    /// Un gestionnaire a été posé depuis la dernière lecture.
    pub(super) static LISTENERS_BUILT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// `true` si un gestionnaire a été posé depuis le dernier appel : une application qui n'en
/// pose aucun ne paie aucun parcours de l'arbre.
pub(crate) fn take_listeners_built() -> bool {
    LISTENERS_BUILT.with(|built| built.replace(false))
}
impl Element {
    pub(super) fn listeners_mut(&mut self) -> &mut Listeners {
        LISTENERS_BUILT.with(|built| built.set(true));
        self.listeners.get_or_insert_with(Default::default)
    }
    /// Reports the pointer entering (`true`) and leaving (`false`) this element or its
    /// descendants.
    pub fn on_hover<M: 'static>(mut self, handler: impl Fn(bool) -> M + 'static) -> Self {
        self.listeners_mut().hover = Some(Handler::new(handler));
        self
    }
    /// Reports the press starting (`true`) and ending (`false`) on this element or its
    /// descendants.
    pub fn on_press<M: 'static>(mut self, handler: impl Fn(bool) -> M + 'static) -> Self {
        self.listeners_mut().press = Some(Handler::new(handler));
        self
    }
    /// Follows a press-and-drag on this button: past 4 logical units the press becomes a
    /// pan ([`PanEvent`](crate::ui::drag_drop::PanEvent): start, moves, end) and the click
    /// is cancelled. The application moves the element itself; rebuilding the view keeps
    /// the gesture.
    pub fn on_pan<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::drag_drop::PanEvent) -> M + 'static,
    ) -> Self {
        self.listeners_mut().pan = Some(Handler::new(handler));
        self
    }
    /// React to the three buttons of the mouse over this element: a button down, the
    /// pointer moving, a button up
    /// ([`PointerEvent`](crate::ui::runtime::PointerEvent)), with the position in the
    /// element. `handler` returns the message to send, or `None` to let the event go to
    /// the ancestors of the element.
    ///
    /// When the handler takes a button down, this element receives the rest of the
    /// gesture, moves and releases, even outside it and outside the window, until the
    /// last button is released: the way to turn a 3D view while the right button is held.
    /// Without a gesture, moves go to the elements under the pointer. The usual effects
    /// of the primary button (click, drag, selection) and the context menu of the
    /// secondary one still happen. Not delivered on macOS and Linux yet.
    pub fn on_pointer<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::runtime::PointerEvent) -> Option<M> + 'static,
    ) -> Self {
        let handler = crate::ui::runtime::raw_input::PointerHandler::new(handler);
        self.listeners_mut().pointer = Some(handler);
        self
    }
    /// React to the mouse wheel over this element with the state of Shift, Ctrl and Alt
    /// ([`PointerEvent`](crate::ui::runtime::PointerEvent) with the phase
    /// [`PointerPhase::Wheel`](crate::ui::runtime::PointerPhase::Wheel) and the amount in
    /// `wheel`): Ctrl+wheel to zoom a timeline, Shift+wheel to scroll it sideways.
    /// `handler` returns the message to send, or `None` to let the wheel go on.
    ///
    /// The handlers of the elements under the pointer run from the deepest to the root,
    /// before the [`on_wheel`](Self::on_wheel) handlers. The first one that returns a
    /// message consumes the wheel: no `on_wheel` handler runs and nothing scrolls. When
    /// every handler returns `None`, the wheel behaves as before. Separate from
    /// [`on_pointer`](Self::on_pointer), whose handlers never see the wheel. Not delivered
    /// on macOS and Linux yet.
    ///
    /// ```
    /// use karbon_ui::ui::{prelude::*, runtime::PointerEvent};
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     Zoom(f32),
    /// }
    /// let timeline = column("timeline", []).on_pointer_wheel(|event: PointerEvent| {
    ///     event.control.then_some(Msg::Zoom(event.wheel.1))
    /// });
    /// # let _ = timeline;
    /// ```
    pub fn on_pointer_wheel<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::runtime::PointerEvent) -> Option<M> + 'static,
    ) -> Self {
        let handler = crate::ui::runtime::raw_input::PointerHandler::new(handler);
        self.listeners_mut().pointer_wheel = Some(handler);
        self
    }
    /// Reports the focus entering (`true`) and leaving (`false`) this element or its
    /// descendants.
    pub fn on_focus<M: 'static>(mut self, handler: impl Fn(bool) -> M + 'static) -> Self {
        self.listeners_mut().focus = Some(Handler::new(handler));
        self
    }
    /// Sends `message` when this element or a descendant is double-clicked.
    pub fn on_double_click<M: Clone + 'static>(mut self, message: M) -> Self {
        self.listeners_mut().double_click = Some(Action::new(message));
        self
    }

    /// Rectangle assigned by the last layout pass, in window coordinates.
    pub fn bounds(&self) -> RectF {
        self.rect
    }

    /// Stylesheet classes applied to this element, in template order (empty for
    /// elements built without classes). Meant for diagnostics and layout dumps.
    pub fn classes(&self) -> &[&'static str] {
        self.styles.as_ref().map_or(&[], |styles| &styles.classes)
    }

    /// Resting visual effect (CSS `transform` and `opacity` without hover, press or
    /// animation). Meant for diagnostics and layout dumps.
    pub fn base_effect(&self) -> crate::ui::animation::Effect {
        self.motion
            .as_ref()
            .map_or_else(Default::default, |m| m.base)
    }

    /// Style computed by the last style resolution, when the element carries stylesheet
    /// or inherited properties. Meant for diagnostics and layout dumps.
    pub fn computed_style(&self) -> Option<&crate::ui::styling::ComputedStyle> {
        self.styles
            .as_ref()
            .filter(|styles| styles.ready)
            .map(|styles| &styles.computed)
    }

    /// Depth-first search for the element with `id` in this subtree, including `self`.
    pub fn find(&self, id: ElementId) -> Option<&Self> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter().find_map(|child| child.find(id))
        }
    }

    pub(crate) fn find_mut(&mut self, id: ElementId) -> Option<&mut Self> {
        if self.id == id {
            Some(self)
        } else {
            self.children
                .iter_mut()
                .find_map(|child| child.find_mut(id))
        }
    }
}
