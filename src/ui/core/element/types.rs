use super::*;
/// Size of an element along one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// Value computed at layout time from a compiled CSS expression (`calc()`, `em`, `vw`…).
    Computed(crate::ui::layout::ComputedLength),
    /// Natural size of the content.
    Auto,
    /// Fixed size in pixels.
    Px(f32),
    /// Percentage of the parent, from 0 to 100.
    Percent(f32),
    /// Share of the remaining space, like a CSS `fr` fraction.
    Fill(f32),
    /// Width of the content laid out without wrapping or shrinking, like CSS
    /// `width: max-content`: the element keeps it whatever room its parent offers (a
    /// wider content overflows). As a height, the same as [`Auto`](Self::Auto).
    MaxContent,
}
/// Spacing on the four sides of a box (padding, margin or border widths).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Insets {
    /// Top inset.
    pub top: f32,
    /// Right inset.
    pub right: f32,
    /// Bottom inset.
    pub bottom: f32,
    /// Left inset.
    pub left: f32,
}
impl Insets {
    /// Same inset on all four sides.
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}
/// Alignment of children on the cross axis.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Align {
    /// Align to the start of the axis.
    Start,
    /// Centre on the axis.
    Center,
    /// Align to the end of the axis.
    End,
    /// Fill the whole cross axis (default).
    #[default]
    Stretch,
    /// Align the first text baselines of the items of a flex row (`align-items:
    /// baseline`); elsewhere it behaves like [`Align::Start`].
    Baseline,
}
/// Distribution of children on the main axis.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Justify {
    /// Pack children at the start (default).
    #[default]
    Start,
    /// Centre children.
    Center,
    /// Pack children at the end.
    End,
    /// Spread the free space between children.
    SpaceBetween,
}
/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    /// Left to right.
    Horizontal,
    /// Top to bottom.
    Vertical,
}
/// Options of a two-pane [`Layout::Split`].
#[derive(Debug, Clone, Copy)]
pub struct Split {
    /// Direction along which the two panes are placed.
    pub axis: Axis,
    /// Share of the space given to the first pane, from 0 to 1.
    pub fraction: f32,
    /// Minimum size of the first pane, in pixels.
    pub min_first: f32,
    /// Minimum size of the second pane, in pixels.
    pub min_second: f32,
    /// Whether the user can drag the divider to resize the panes.
    pub resizable: bool,
}
impl Split {
    /// Side-by-side panes; `fraction` is the share of the first one.
    pub fn horizontal(fraction: f32) -> Self {
        Self {
            axis: Axis::Horizontal,
            fraction,
            min_first: 0.,
            min_second: 0.,
            resizable: false,
        }
    }
    /// Stacked panes; `fraction` is the share of the first one.
    pub fn vertical(fraction: f32) -> Self {
        Self {
            axis: Axis::Vertical,
            ..Self::horizontal(fraction)
        }
    }
    /// Minimum sizes of the first and second panes, in pixels.
    pub fn min_sizes(mut self, first: f32, second: f32) -> Self {
        self.min_first = first;
        self.min_second = second;
        self
    }
    /// Allow or forbid resizing by dragging the divider.
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }
}
/// How an element arranges its children.
#[derive(Debug, Clone, Copy)]
pub enum Layout {
    /// No children are laid out; the element is sized by its widget.
    Leaf,
    /// Page, dialogs, then an optional anchored popover. Root only.
    Layers,
    /// Children placed in a horizontal flex line.
    Row,
    /// Children stacked vertically.
    Column,
    /// Grid with a fixed number of equal columns.
    Grid {
        /// Number of columns.
        columns: usize,
    },
    /// Two resizable panes, see [`Split`].
    Split(Split),
    /// One child, clipped to a viewport with a scrollbar drawn by the engine.
    Scroll(Axis),
    /// Vertical collection with a fixed row height, built on demand.
    VirtualList,
}

/// Box-model and appearance properties shared by every element.
#[derive(Clone, Copy)]
pub struct Style {
    /// Preferred width.
    pub width: Length,
    /// Preferred height.
    pub height: Length,
    /// Minimum width, in pixels.
    pub min_width: f32,
    /// Minimum height, in pixels.
    pub min_height: f32,
    /// Maximum width, in pixels (`f32::INFINITY` when unbounded).
    pub max_width: f32,
    /// Maximum height, in pixels (`f32::INFINITY` when unbounded).
    pub max_height: f32,
    /// Inner spacing between the border and the children.
    pub padding: Insets,
    /// Outer spacing around the element.
    pub margin: Insets,
    /// Spacing between children on both axes, unless overridden by `row_gap` / `column_gap`.
    pub gap: f32,
    /// Vertical spacing between rows, overriding `gap`.
    pub row_gap: Option<f32>,
    /// Horizontal spacing between columns, overriding `gap`.
    pub column_gap: Option<f32>,
    /// Cross-axis alignment of children.
    pub align: Align,
    /// Main-axis distribution of children.
    pub justify: Justify,
    /// Solid background colour, if any.
    pub background: Option<Color>,
    /// Corner radius, in pixels.
    pub radius: f32,
    /// Number of grid columns spanned (1 by default); [`Style::SPAN_FULL`] reaches the
    /// last column line (`grid-column: 1 / -1`).
    pub span: usize,
    /// Explicit grid column, counted from 1.
    pub column: Option<usize>,
    /// Explicit grid row, counted from 1.
    pub row: Option<usize>,
}
impl Style {
    /// [`span`](Self::span) value that reaches the last column line, whatever the
    /// number of columns (`grid-column: 1 / -1`, Tailwind `col-span-full`).
    pub const SPAN_FULL: usize = usize::MAX;
}
impl Default for Style {
    fn default() -> Self {
        Self {
            width: Length::Fill(1.),
            height: Length::Auto,
            min_width: 0.,
            min_height: 0.,
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            padding: Insets::all(0.),
            margin: Insets::all(0.),
            gap: 0.,
            row_gap: None,
            column_gap: None,
            align: Align::Stretch,
            justify: Justify::Start,
            background: None,
            radius: 0.,
            span: 1,
            column: None,
            row: None,
        }
    }
}
impl Style {
    /// Spacing between columns: `column_gap`, or `gap` when unset.
    pub fn horizontal_gap(&self) -> f32 {
        self.column_gap.unwrap_or(self.gap)
    }
    /// Spacing between rows: `row_gap`, or `gap` when unset.
    pub fn vertical_gap(&self) -> f32 {
        self.row_gap.unwrap_or(self.gap)
    }
}
