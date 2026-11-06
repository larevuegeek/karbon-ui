//! Disposition des enfants : flexbox, grille, alignements, débordement et barres de
//! défilement.
use super::{ContainerStyle, Node};
use crate::ui::{element::*, theme::Color};
impl<W: Widget + 'static> Node<W> {
    /// Enables CSS flexbox semantics for this container.
    pub fn css_flex(mut self, value: bool) -> Self {
        self.element.layout_rules().css_flex = value;
        self
    }
    /// Lays children out in a row (horizontal) or a column (vertical).
    pub fn flex_direction(mut self, value: Axis) -> Self
    where
        W: ContainerStyle,
    {
        self.element.layout = match value {
            Axis::Horizontal => Layout::Row,
            Axis::Vertical => Layout::Column,
        };
        self
    }
    /// Sets flex line wrapping.
    pub fn flex_wrap(mut self, value: FlexWrap) -> Self {
        self.element.layout_rules().flex_wrap = value;
        self
    }
    /// Lays out children in reverse order.
    pub fn flex_reverse(mut self, value: bool) -> Self {
        self.element.layout_rules().flex_reverse = value;
        self
    }
    /// Sets the flex grow factor.
    pub fn flex_grow(mut self, value: f32) -> Self {
        self.element.layout_rules().flex_grow = Some(value);
        self
    }
    /// Sets the flex shrink factor.
    pub fn flex_shrink(mut self, value: f32) -> Self {
        self.element.layout_rules().flex_shrink = Some(value);
        self
    }
    /// Sets the flex basis.
    pub fn flex_basis(mut self, value: Length) -> Self {
        self.element.layout_rules().flex_basis = Some(value);
        self
    }
    /// Sets the cross-axis alignment of children.
    pub fn align(mut self, value: Align) -> Self {
        self.element = self.element.align(value);
        self
    }
    /// Overrides the parent's cross-axis alignment for this node; `None` inherits it.
    pub fn align_self(mut self, value: Option<Align>) -> Self {
        self.element.layout_rules().align_self = value;
        self
    }
    /// Sets the main-axis distribution of children.
    pub fn justify(mut self, value: Justify) -> Self {
        self.element = self.element.justify(value);
        self
    }
    /// Sets the gap between children (rows and columns).
    pub fn gap(mut self, value: f32) -> Self {
        self.clear_spacing(8..10);
        self.element = self.element.gap(value);
        self
    }
    /// Sets the gap between rows.
    pub fn row_gap(mut self, value: f32) -> Self {
        self.clear_spacing(8..9);
        self.element = self.element.row_gap(value);
        self
    }
    /// Sets the gap between columns.
    pub fn column_gap(mut self, value: f32) -> Self {
        self.clear_spacing(9..10);
        self.element = self.element.column_gap(value);
        self
    }
    /// Turns the container into a grid of `value` equal columns.
    pub fn columns(mut self, value: usize) -> Self
    where
        W: ContainerStyle,
    {
        self.element.layout = Layout::Grid { columns: value };
        self
    }
    /// Turns the container into a grid with explicit column tracks.
    pub fn grid_tracks(mut self, value: std::sync::Arc<[crate::ui::layout::GridTrack]>) -> Self
    where
        W: ContainerStyle,
    {
        self.element.layout = Layout::Grid {
            columns: value.len(),
        };
        let rules = self.element.layout_rules();
        rules.columns = Some(value);
        rules.grid_auto_repeat = None;
        self
    }
    /// Sets explicit grid row tracks (`grid-template-rows`).
    pub fn grid_rows(mut self, value: std::sync::Arc<[crate::ui::layout::GridTrack]>) -> Self {
        self.element.layout_rules().rows = Some(value);
        self
    }
    /// Repeats the first column track as many times as the grid's width allows
    /// (`repeat(auto-fill, …)`; `fit` for `auto-fit`, at most once per child).
    pub fn grid_auto_repeat(mut self, fit: bool) -> Self {
        self.element.layout_rules().grid_auto_repeat = Some(fit);
        self
    }
    /// Sets the number of grid columns spanned.
    pub fn span(mut self, value: usize) -> Self {
        self.element = self.element.span(value);
        self
    }
    /// Places the node on a given grid row.
    pub fn row(mut self, value: usize) -> Self {
        self.element.style.row = Some(value);
        self
    }
    /// Places the node in a given grid column.
    pub fn column(mut self, value: usize) -> Self {
        self.element.style.column = Some(value);
        self
    }
    /// Aligns the node horizontally inside its grid cell (`justify-self`); `None`
    /// stretches.
    pub fn justify_self(mut self, value: Option<Align>) -> Self {
        self.element.layout_rules().justify_self = value;
        self
    }
    /// Aligns the children of a grid container horizontally in their cells
    /// (`justify-items`), unless they set [`justify_self`](Self::justify_self).
    pub fn justify_items(mut self, value: Option<Align>) -> Self {
        self.element.layout_rules().justify_items = value;
        self
    }
    /// Sets horizontal and vertical overflow; clipping follows rounded corners.
    pub fn overflow(mut self, x: Overflow, y: Overflow) -> Self {
        let rules = self.element.layout_rules();
        rules.overflow_x = x;
        rules.overflow_y = y;
        rules.rounded_clip = true;
        self
    }
    /// Sets the overflow of one axis (`horizontal` selects x, otherwise y).
    pub fn overflow_axis(mut self, horizontal: bool, value: Overflow) -> Self {
        let rules = self.element.layout_rules();
        rules.rounded_clip = true;
        if horizontal {
            rules.overflow_x = value;
        } else {
            rules.overflow_y = value;
        }
        self
    }
    /// Sets one edge (0 top, 1 right, 2 bottom, 3 left) of the space kept around the
    /// element when it is scrolled into view (`scroll-margin`).
    pub fn scroll_margin_edge(mut self, (index, value): (usize, f32)) -> Self {
        set_edge(&mut self.element.layout_rules().scroll_margin, index, value);
        self
    }
    /// Sets one edge of a scroll container's view inset used when a descendant is
    /// scrolled into view (`scroll-padding`).
    pub fn scroll_padding_edge(mut self, (index, value): (usize, f32)) -> Self {
        set_edge(
            &mut self.element.layout_rules().scroll_padding,
            index,
            value,
        );
        self
    }
    /// Keeps the rest of a wheel scroll in this scroll container
    /// (`overscroll-behavior: contain`).
    pub fn overscroll_contain(mut self, value: bool) -> Self {
        self.element.layout_rules().overscroll_contain = value;
        self
    }
    /// Scroll bar thickness of a scroll container (`scrollbar-width`).
    pub fn scrollbar_width(mut self, value: ScrollbarWidth) -> Self {
        self.element.layout_rules().scrollbar.width = value;
        self
    }
    /// Thumb and track colours of the scroll bar (`scrollbar-color`); `None` uses the theme.
    pub fn scrollbar_color(mut self, (thumb, track): (Option<Color>, Option<Color>)) -> Self {
        let bar = &mut self.element.layout_rules().scrollbar;
        bar.thumb = thumb;
        bar.track = track;
        self
    }
    /// Gutter thickness of the scroll bar (`::-webkit-scrollbar { width }`), in pixels.
    pub fn scrollbar_size(mut self, value: f32) -> Self {
        self.element.layout_rules().scrollbar.size = Some(value.max(0.));
        self
    }
    /// Reserve the scroll bar gutter even when the content fits (`scrollbar-gutter: stable`).
    pub fn scrollbar_gutter(mut self, stable: bool) -> Self {
        self.element.layout_rules().scrollbar.gutter_stable = stable;
        self
    }
}
/// Côté `index` (haut, droite, bas, gauche) de `insets`.
fn set_edge(insets: &mut Insets, index: usize, value: f32) {
    match index {
        0 => insets.top = value,
        1 => insets.right = value,
        2 => insets.bottom = value,
        _ => insets.left = value,
    }
}
