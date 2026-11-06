use super::*;
/// Which margins are `auto` and absorb free space, as in CSS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoMargins {
    /// Top margin is `auto`.
    pub top: bool,
    /// Right margin is `auto`.
    pub right: bool,
    /// Bottom margin is `auto`.
    pub bottom: bool,
    /// Left margin is `auto`.
    pub left: bool,
}
impl AutoMargins {
    /// Number of `auto` margins on the main axis (`row` = horizontal).
    pub fn count(self, row: bool) -> usize {
        if row {
            self.left as usize + self.right as usize
        } else {
            self.top as usize + self.bottom as usize
        }
    }
    /// Add `value` to each `auto` margin of the given axis (`row` = horizontal).
    pub fn add(self, mut margin: Insets, row: bool, value: f32) -> Insets {
        if row {
            if self.left {
                margin.left += value;
            }
            if self.right {
                margin.right += value;
            }
        } else {
            if self.top {
                margin.top += value;
            }
            if self.bottom {
                margin.bottom += value;
            }
        }
        margin
    }
}
/// How `width` and `height` are interpreted, as CSS `box-sizing`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxSizing {
    /// Sizes include padding and border (default).
    #[default]
    BorderBox,
    /// Sizes cover the content only; padding and border are added.
    ContentBox,
}
/// Positioning scheme, as CSS `position`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    /// Normal flow (default).
    #[default]
    Static,
    /// Normal flow, then shifted by the offsets.
    Relative,
    /// Out of flow, placed relative to the containing block.
    Absolute,
    /// Out of flow, placed relative to the viewport.
    Fixed,
}
/// Overflow behaviour on one axis, as CSS `overflow`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    /// Content may paint outside the element.
    Visible,
    /// Content is clipped to the element (default).
    #[default]
    Clip,
    /// Content is clipped, like `Clip`.
    Hidden,
    /// In templates, the container becomes a scroll viewport when its content
    /// overflows (CSS `overflow: auto`); elsewhere it clips like `Clip`.
    Auto,
    /// In templates, the container becomes a scroll viewport (CSS `overflow: scroll`);
    /// elsewhere it clips like `Clip`.
    Scroll,
}
/// Line wrapping of flex children, as CSS `flex-wrap`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexWrap {
    /// Single line (default).
    #[default]
    NoWrap,
    /// Wrap onto additional lines.
    Wrap,
    /// Wrap onto additional lines in reverse cross-axis order.
    WrapReverse,
}
/// Thickness of a scroll bar, as CSS `scrollbar-width`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarWidth {
    /// Engine default: a 12 px track always reserved, theme colours.
    #[default]
    Auto,
    /// Thin bar: a 10 px gutter reserved only while the content overflows (unless
    /// `scrollbar-gutter: stable`), a 4 px rounded thumb and a transparent track.
    Thin,
    /// No bar and no reserved space; the content still scrolls (wheel, keyboard).
    None,
}
/// Appearance of the scroll bar of a scroll container (`scrollbar-width`,
/// `scrollbar-color`, `scrollbar-gutter`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scrollbar {
    /// Thickness and behaviour of the bar.
    pub width: ScrollbarWidth,
    /// Thumb colour; `None` uses the theme.
    pub thumb: Option<Color>,
    /// Track colour; `None` uses the theme (`auto`) or nothing (`thin`).
    pub track: Option<Color>,
    /// Reserve the gutter even when the content fits (`scrollbar-gutter: stable`).
    pub gutter_stable: bool,
    /// Gutter thickness replacing the default of `auto` and `thin` bars
    /// (`::-webkit-scrollbar { width }` in templates); `None` keeps the default.
    pub size: Option<f32>,
}
impl Scrollbar {
    /// Thickness of the gutter reserved next to the viewport.
    pub fn thickness(self) -> f32 {
        match self.width {
            ScrollbarWidth::Auto => self.size.unwrap_or(crate::ui::scroll::BAR_SIZE),
            ScrollbarWidth::Thin => self.size.unwrap_or(crate::ui::scroll::THIN_BAR_SIZE),
            ScrollbarWidth::None => 0.,
        }
    }
}
/// Less common layout rules (CSS-like flex, positioning, overflow), allocated on demand.
///
/// Obtain them with [`Element::layout_rules`].
#[derive(Clone, Debug, Default)]
pub struct LayoutRules {
    pub(crate) paint_order: Vec<usize>,
    /// Font sizes and viewport used to resolve relative units.
    pub unit_context: crate::ui::layout::UnitContext,
    /// Explicit grid column tracks, replacing equal columns.
    pub columns: Option<std::sync::Arc<[crate::ui::layout::GridTrack]>>,
    /// Which margins are `auto`.
    pub margin_auto: AutoMargins,
    /// Border widths, part of the box model.
    pub border: Insets,
    /// How sizes include padding and border.
    pub box_sizing: BoxSizing,
    /// Use CSS flex defaults: `flex-grow: 0` and `flex-shrink: 1` instead of growing `Fill` children.
    pub css_flex: bool,
    /// Wrapping of flex children.
    pub flex_wrap: FlexWrap,
    /// Reverse the main-axis order of children.
    pub flex_reverse: bool,
    /// Explicit `flex-grow` factor.
    pub flex_grow: Option<f32>,
    /// Explicit `flex-shrink` factor.
    pub flex_shrink: Option<f32>,
    /// Explicit `flex-basis`.
    pub flex_basis: Option<Length>,
    /// Per-child override of the parent's [`Align`].
    pub align_self: Option<Align>,
    /// Positioning scheme.
    pub position: Position,
    /// `top`, `right`, `bottom`, `left` offsets used by positioned elements.
    pub offsets: [Option<Length>; 4],
    /// Paint order among siblings; higher values paint later (on top).
    pub z_index: i32,
    /// Horizontal overflow behaviour.
    pub overflow_x: Overflow,
    /// Vertical overflow behaviour.
    pub overflow_y: Overflow,
    /// Clip to the rounded padding box instead of a plain rectangle (set by `overflow`).
    pub rounded_clip: bool,
    /// Explicit grid row tracks (`grid-template-rows`); rows beyond them size to content.
    pub rows: Option<std::sync::Arc<[crate::ui::layout::GridTrack]>>,
    /// Layout order among siblings (CSS `order`); painting and focus keep the tree order.
    pub order: i32,
    /// Removed from layout, painting, hit testing, focus and accessibility
    /// (`display: none`).
    pub hidden: bool,
    /// Preferred width / height ratio (`aspect-ratio`), applied to an `auto` height.
    pub aspect_ratio: Option<f32>,
    /// Horizontal alignment inside a grid cell (`justify-self`); `None` stretches.
    pub justify_self: Option<Align>,
    /// Default `justify-self` of the children of a grid container (`justify-items`).
    pub justify_items: Option<Align>,
    /// Scroll bar appearance of a scroll container.
    pub scrollbar: Scrollbar,
    /// Inline-level box (`display: inline-block`, a `<button>` of a framework
    /// stylesheet): a block (non-flex) parent gives it its natural width instead of
    /// stretching it.
    pub inline_level: bool,
    /// Size limits given as a percentage or an expression (`max-width: 40%`,
    /// `max-height: calc(100vh - 330px)`), in the order min-width, min-height,
    /// max-width, max-height. Layout resolves them against the space offered to the
    /// element and writes the result into [`Style`](crate::ui::element::Style)'s pixel
    /// limits.
    pub relative_limits: [Option<Length>; 4],
    /// `repeat(auto-fill, track)` (`Some(false)`) or `repeat(auto-fit, track)`
    /// (`Some(true)`): the first column track is repeated as many times as the grid's
    /// width allows; with `auto-fit`, at most once per child.
    pub grid_auto_repeat: Option<bool>,
    /// Space kept around the element when it is scrolled into view (`scroll-margin`).
    pub scroll_margin: Insets,
    /// Inset of a scroll container's view used when scrolling a descendant into view
    /// (`scroll-padding`).
    pub scroll_padding: Insets,
    /// A scroll container keeps the rest of a wheel scroll instead of passing it to
    /// the enclosing scroll containers (`overscroll-behavior: contain | none`).
    pub overscroll_contain: bool,
    /// Révélation en cours ([`Element::reveal`]) : l'élément reste affiché (même
    /// `hidden`) à une fraction de sa hauteur naturelle.
    pub(crate) reveal: Option<Reveal>,
    /// Plus grand `z-index` positif d'un descendant de feuille de framework que ses
    /// ancêtres sans contexte d'empilement (z-index nul) laissent remonter jusqu'ici :
    /// l'élément est peint parmi ses frères à ce rang, comme en CSS.
    pub(crate) lifted_z: i32,
}
/// Hauteur d'un élément pendant sa révélation ([`Element::reveal`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reveal {
    /// Fraction (`0..1`) de la hauteur naturelle occupée par la boîte de bordure.
    pub progress: f32,
    /// Hauteur déclarée, mesurée sans les tailles imposées par le parent pendant le
    /// layout (`measure_main`, flex).
    pub height: Length,
    /// Hauteur naturelle (boîte de bordure) du dernier `measure` : le contenu y est
    /// disposé, puis rogné à la boîte animée.
    pub natural: f32,
}
impl Element {
    /// Advanced layout rules of this element, created with defaults on first access.
    pub fn layout_rules(&mut self) -> &mut LayoutRules {
        self.rules.get_or_insert_with(Default::default)
    }
    pub(crate) fn border(&self) -> Insets {
        self.rules.as_ref().map_or(Insets::default(), |r| r.border)
    }
    pub(crate) fn auto_margins(&self) -> AutoMargins {
        self.rules
            .as_ref()
            .map_or(AutoMargins::default(), |r| r.margin_auto)
    }
    pub(crate) fn content_box(&self) -> bool {
        self.rules
            .as_ref()
            .is_some_and(|r| r.box_sizing == BoxSizing::ContentBox)
    }
}
