use crate::ui::element::*;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Layout declarations of a style rule; `None` leaves the element's value unchanged.
///
/// Edge arrays are ordered top, right, bottom, left. Pixel values are
/// device-independent.
pub struct LayoutProperties {
    /// Preferred width.
    pub width: Option<Length>,
    /// Preferred height.
    pub height: Option<Length>,
    /// Minimum width in pixels.
    pub min_width: Option<f32>,
    /// Minimum height in pixels.
    pub min_height: Option<f32>,
    /// Maximum width in pixels.
    pub max_width: Option<f32>,
    /// Maximum height in pixels.
    pub max_height: Option<f32>,
    /// `margin` shorthand; an inner `None` means `auto` on that edge.
    pub margin: Option<[Option<f32>; 4]>,
    /// Individual padding edges (`padding-top`…).
    pub padding_edges: [Option<f32>; 4],
    /// Gap between rows and columns; overrides `row_gap` and `column_gap`.
    pub gap: Option<f32>,
    /// Gap between rows.
    pub row_gap: Option<f32>,
    /// Gap between columns.
    pub column_gap: Option<f32>,
    /// Individual margin edges (`margin-top`…); `Some(None)` means `auto`.
    pub margin_edges: [Option<Option<f32>>; 4],
    /// Main axis; setting it switches the element to a flex row or column.
    pub axis: Option<Axis>,
    /// Cross-axis alignment of children.
    pub align: Option<Align>,
    /// Main-axis distribution of children.
    pub justify: Option<Justify>,
    /// Whether sizes include padding and border.
    pub box_sizing: Option<BoxSizing>,
    /// Flex line wrapping.
    pub flex_wrap: Option<FlexWrap>,
    /// Lay children out in reverse order (`row-reverse`, `column-reverse`).
    pub flex_reverse: Option<bool>,
    /// Flex grow factor.
    pub flex_grow: Option<f32>,
    /// Flex shrink factor.
    pub flex_shrink: Option<f32>,
    /// Flex basis.
    pub flex_basis: Option<Length>,
    /// `display: none` (`Some(true)`) or shown again (`Some(false)`).
    pub hidden: Option<bool>,
    /// `display: block`: a vertical block without flex semantics.
    pub block: Option<bool>,
    /// `display: grid`, keeping the declared column tracks (one column without).
    pub display_grid: Option<bool>,
    /// Column tracks (`grid-template-columns`); switches the element to a grid.
    pub grid_columns: Option<&'static [crate::ui::layout::GridTrack]>,
    /// Row tracks (`grid-template-rows`).
    pub grid_rows: Option<&'static [crate::ui::layout::GridTrack]>,
    /// Layout order among siblings (`order`).
    pub order: Option<i32>,
    /// Preferred width / height ratio (`aspect-ratio`); `Some(None)` is `auto`.
    pub aspect_ratio: Option<Option<f32>>,
    /// Grid row (`grid-row`, `grid-row-start`), from 1.
    pub row: Option<usize>,
    /// Grid column (`grid-column`, `grid-column-start`), from 1.
    pub column: Option<usize>,
    /// Grid columns spanned (`grid-column: span N`).
    pub span: Option<usize>,
    /// Alignment in the grid cell (`justify-self`); `Some(None)` stretches.
    pub justify_self: Option<Option<Align>>,
    /// Size limits given as a percentage or an expression, in the order min-width,
    /// min-height, max-width, max-height (see
    /// [`LayoutRules::relative_limits`](crate::ui::element::LayoutRules::relative_limits)).
    pub relative_limits: [Option<Length>; 4],
    /// `repeat(auto-fill|auto-fit, track)` declared with
    /// [`grid_columns`](Self::grid_columns) (`Some(true)` for auto-fit); see
    /// [`LayoutRules::grid_auto_repeat`](crate::ui::element::LayoutRules::grid_auto_repeat).
    pub grid_auto_repeat: Option<bool>,
    /// Paint order among siblings (`z-index`), for instance raised while focused.
    pub z_index: Option<i32>,
    /// Padding, margins and gaps given as expressions (`em`, `rem`, `vw`, `vh`,
    /// `calc()`…); they win over the pixel values of the same edges and are written
    /// into them, in pixels, while styles resolve.
    pub spacing: super::SpacingExpressions,
}
impl LayoutProperties {
    /// Limite `i` (min-w, min-h, max-w, max-h) en px ou relative.
    fn limit(&self, i: usize) -> (Option<f32>, Option<Length>) {
        let px = [
            self.min_width,
            self.min_height,
            self.max_width,
            self.max_height,
        ][i];
        (px, self.relative_limits[i])
    }
    /// Layers `self` over `b`: each property set on `self` wins. The `gap` and
    /// `margin` shorthands set on `self` discard the matching longhands from `b`.
    pub fn over(self, b: Self) -> Self {
        // Une limite (en px ou relative) posée par `self` remplace celle de `b`.
        let limits: [(Option<f32>, Option<Length>); 4] = std::array::from_fn(|i| {
            let own = self.limit(i);
            if own.0.is_some() || own.1.is_some() {
                own
            } else {
                b.limit(i)
            }
        });
        let spacing = self.spacing_over(&b);
        Self {
            spacing,
            width: self.width.or(b.width),
            height: self.height.or(b.height),
            min_width: limits[0].0,
            min_height: limits[1].0,
            max_width: limits[2].0,
            max_height: limits[3].0,
            relative_limits: limits.map(|l| l.1),
            // Liée aux pistes : celles de `self` emportent leur répétition.
            grid_auto_repeat: if self.grid_columns.is_some() {
                self.grid_auto_repeat
            } else {
                b.grid_auto_repeat
            },
            margin: self.margin.or(b.margin),
            padding_edges: std::array::from_fn(|i| self.padding_edges[i].or(b.padding_edges[i])),
            gap: self.gap.or(b.gap),
            row_gap: self
                .row_gap
                .or(if self.gap.is_some() { None } else { b.row_gap }),
            column_gap: self.column_gap.or(if self.gap.is_some() {
                None
            } else {
                b.column_gap
            }),
            margin_edges: std::array::from_fn(|i| {
                self.margin_edges[i].or(if self.margin.is_some() {
                    None
                } else {
                    b.margin_edges[i]
                })
            }),
            axis: self.axis.or(b.axis),
            align: self.align.or(b.align),
            justify: self.justify.or(b.justify),
            box_sizing: self.box_sizing.or(b.box_sizing),
            flex_wrap: self.flex_wrap.or(b.flex_wrap),
            flex_reverse: self.flex_reverse.or(b.flex_reverse),
            flex_grow: self.flex_grow.or(b.flex_grow),
            flex_shrink: self.flex_shrink.or(b.flex_shrink),
            flex_basis: self.flex_basis.or(b.flex_basis),
            hidden: self.hidden.or(b.hidden),
            block: self.block.or(b.block),
            display_grid: self.display_grid.or(b.display_grid),
            grid_columns: self.grid_columns.or(b.grid_columns),
            grid_rows: self.grid_rows.or(b.grid_rows),
            order: self.order.or(b.order),
            aspect_ratio: self.aspect_ratio.or(b.aspect_ratio),
            row: self.row.or(b.row),
            column: self.column.or(b.column),
            span: self.span.or(b.span),
            justify_self: self.justify_self.or(b.justify_self),
            z_index: self.z_index.or(b.z_index),
        }
    }
    /// The rule changes the element's layout model (direction, block, grid).
    pub fn changes_model(self) -> bool {
        self.axis.is_some()
            || self.block.is_some()
            || self.display_grid.is_some()
            || self.grid_columns.is_some()
    }
    /// Writes the declared values into `node`'s style and layout rules.
    pub fn apply(self, node: &mut Element) {
        if let Some(v) = self.width {
            node.style.width = v;
        }
        if let Some(v) = self.height {
            node.style.height = v;
        }
        if let Some(v) = self.min_width {
            node.style.min_width = v;
        }
        if let Some(v) = self.min_height {
            node.style.min_height = v;
        }
        if let Some(v) = self.max_width {
            node.style.max_width = v;
        }
        if let Some(v) = self.max_height {
            node.style.max_height = v;
        }
        for i in 0..4 {
            match self.limit(i) {
                (_, Some(length)) => node.layout_rules().relative_limits[i] = Some(length),
                (Some(_), None) => {
                    if let Some(rules) = node.rules.as_mut() {
                        rules.relative_limits[i] = None;
                    }
                }
                (None, None) => {}
            }
        }
        if let Some(v) = self.margin {
            node.style.margin = Insets::edges(
                v[0].unwrap_or(0.),
                v[1].unwrap_or(0.),
                v[2].unwrap_or(0.),
                v[3].unwrap_or(0.),
            );
            node.layout_rules().margin_auto = AutoMargins {
                top: v[0].is_none(),
                right: v[1].is_none(),
                bottom: v[2].is_none(),
                left: v[3].is_none(),
            };
        }
        for (i, value) in self.margin_edges.into_iter().enumerate() {
            if let Some(value) = value {
                let (n, auto) = (value.unwrap_or(0.), value.is_none());
                match i {
                    0 => {
                        node.style.margin.top = n;
                        node.layout_rules().margin_auto.top = auto;
                    }
                    1 => {
                        node.style.margin.right = n;
                        node.layout_rules().margin_auto.right = auto;
                    }
                    2 => {
                        node.style.margin.bottom = n;
                        node.layout_rules().margin_auto.bottom = auto;
                    }
                    _ => {
                        node.style.margin.left = n;
                        node.layout_rules().margin_auto.left = auto;
                    }
                }
            }
        }
        if let Some(v) = self.padding_edges[0] {
            node.style.padding.top = v;
        }
        if let Some(v) = self.padding_edges[1] {
            node.style.padding.right = v;
        }
        if let Some(v) = self.padding_edges[2] {
            node.style.padding.bottom = v;
        }
        if let Some(v) = self.padding_edges[3] {
            node.style.padding.left = v;
        }
        if let Some(v) = self.gap {
            node.style.gap = v;
            node.style.row_gap = None;
            node.style.column_gap = None;
        }
        if let Some(v) = self.row_gap {
            node.style.row_gap = Some(v);
        }
        if let Some(v) = self.column_gap {
            node.style.column_gap = Some(v);
        }
        if let Some(v) = self.axis {
            node.layout = if v == Axis::Horizontal {
                Layout::Row
            } else {
                Layout::Column
            };
            node.layout_rules().css_flex = true;
        }
        if let Some(v) = self.align {
            node.style.align = v;
        }
        if let Some(v) = self.justify {
            node.style.justify = v;
        }
        if let Some(v) = self.box_sizing {
            node.layout_rules().box_sizing = v;
        }
        if let Some(v) = self.flex_wrap {
            node.layout_rules().flex_wrap = v;
        }
        if let Some(v) = self.flex_reverse {
            node.layout_rules().flex_reverse = v;
        }
        if let Some(v) = self.flex_grow {
            node.layout_rules().flex_grow = Some(v);
        }
        if let Some(v) = self.flex_shrink {
            node.layout_rules().flex_shrink = Some(v);
        }
        if let Some(v) = self.flex_basis {
            node.layout_rules().flex_basis = Some(v);
        }
        if let Some(v) = self.hidden {
            node.layout_rules().hidden = v;
        }
        if self.block == Some(true) {
            node.layout = Layout::Column;
            node.layout_rules().css_flex = false;
        }
        if let Some(tracks) = self.grid_columns {
            let rules = node.layout_rules();
            // Pas de réallocation tant que les pistes ne changent pas.
            if rules.columns.as_deref() != Some(tracks) {
                rules.columns = Some(tracks.into());
            }
            rules.grid_auto_repeat = self.grid_auto_repeat;
            node.layout = Layout::Grid {
                columns: tracks.len(),
            };
        } else if self.display_grid == Some(true) {
            let columns = node
                .rules
                .as_ref()
                .and_then(|r| r.columns.as_ref())
                .map_or(1, |c| c.len());
            node.layout = Layout::Grid { columns };
        }
        if let Some(tracks) = self.grid_rows {
            let rules = node.layout_rules();
            if rules.rows.as_deref() != Some(tracks) {
                rules.rows = Some(tracks.into());
            }
        }
        if let Some(v) = self.order {
            node.layout_rules().order = v;
        }
        if let Some(v) = self.aspect_ratio {
            node.layout_rules().aspect_ratio = v;
        }
        if let Some(v) = self.row {
            node.style.row = Some(v);
        }
        if let Some(v) = self.column {
            node.style.column = Some(v);
        }
        if let Some(v) = self.span {
            node.style.span = v;
        }
        if let Some(v) = self.z_index {
            node.layout_rules().z_index = v;
        }
        if let Some(v) = self.justify_self {
            node.layout_rules().justify_self = v;
        }
    }
}
