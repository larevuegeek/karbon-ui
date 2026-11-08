//! Elements, constraints and extension points, independent of the operating system.
use crate::application::{Action, Handler};
use crate::ui::{
    scene::{RectF, Scene},
    theme::{Color, Theme},
};
mod aria;
mod builders;
mod id;
pub(crate) mod memo;
mod rules;
mod types;
mod units;
mod widget;
pub use crate::ui::core::geometry::SizeF;
pub use aria::CurrentItem;
pub use id::ElementId;
pub use rules::*;
pub use types::*;
pub use widget::*;
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
    /// `onwheel` : gestionnaire de la molette ([`Element::on_wheel`]).
    pub(crate) on_wheel: Option<Box<crate::ui::runtime::WheelHandler>>,
    /// Sous-arbre mémorisé ([`Element::memo`]).
    pub(crate) memo: Option<Box<memo::MemoState>>,
    /// Mesures et placement de l'image précédente (layout incrémental).
    pub(crate) layout_cache: Option<Box<memo::LayoutCache>>,
    /// Indicateurs de la passe de layout en cours (`layout::memo`).
    pub(crate) layout_bits: u8,
    /// `content-visibility` : contenu sauté au dernier layout (ni peint ni touché, son
    /// placement n'est plus tenu à jour).
    pub(crate) content_skipped: bool,
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
}
impl Element {
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
