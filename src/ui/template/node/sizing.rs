//! Boîte du nœud : tailles et limites, marges, padding, positionnement et ordre.
use super::Node;
use crate::ui::element::*;
impl<W: Widget + 'static> Node<W> {
    /// Sets the preferred width.
    pub fn width(mut self, value: impl Into<Length>) -> Self {
        self.element = self.element.width(value);
        self
    }
    /// Sets the preferred height.
    pub fn height(mut self, value: impl Into<Length>) -> Self {
        self.element = self.element.height(value);
        self
    }
    /// Sets the minimum width in pixels.
    pub fn min_width(mut self, value: f32) -> Self {
        self.element.style.min_width = value;
        self.fixed_limit(0)
    }
    /// Sets the minimum height in pixels.
    pub fn min_height(mut self, value: f32) -> Self {
        self.element.style.min_height = value;
        self.fixed_limit(1)
    }
    /// Sets the maximum width in pixels.
    pub fn max_width(mut self, value: f32) -> Self {
        self.element.style.max_width = value;
        self.fixed_limit(2)
    }
    /// Sets the maximum height in pixels.
    pub fn max_height(mut self, value: f32) -> Self {
        self.element.style.max_height = value;
        self.fixed_limit(3)
    }
    /// Une limite en px remplace la limite relative déclarée avant elle.
    pub(super) fn fixed_limit(mut self, index: usize) -> Self {
        if let Some(rules) = self.element.rules.as_mut() {
            rules.relative_limits[index] = None;
        }
        self
    }
    /// Sets a size limit given as a percentage or an expression (`max-width: 40%`):
    /// `index` is 0 for min-width, 1 min-height, 2 max-width, 3 max-height. Layout
    /// resolves it against the space offered by the parent.
    pub fn relative_limit(mut self, (index, value): (usize, Length)) -> Self {
        if let Some(slot) = self.element.layout_rules().relative_limits.get_mut(index) {
            *slot = Some(value);
        }
        self
    }
    /// Sets the preferred width / height ratio (`aspect-ratio`); `None` is `auto`.
    pub fn aspect_ratio(mut self, value: Option<f32>) -> Self {
        self.properties().layout.aspect_ratio = Some(value);
        self
    }
    /// Sets whether sizes include padding and border.
    pub fn box_sizing(mut self, value: BoxSizing) -> Self {
        self.element.layout_rules().box_sizing = value;
        self
    }
    /// Sets the padding on every edge.
    pub fn padding(mut self, value: Insets) -> Self {
        self.clear_spacing(0..4);
        self.element.layout_rules();
        self.element = self.element.padding(value);
        self.properties().padding = Some(value);
        self
    }
    /// Sets the margin on every edge.
    pub fn margin(mut self, value: Insets) -> Self {
        self.clear_spacing(4..8);
        self.element = self.element.margin(value);
        self
    }
    /// Sets the margin from CSS values (top, right, bottom, left); `None` means `auto`.
    pub fn css_margin(mut self, values: [Option<f32>; 4]) -> Self {
        self.clear_spacing(4..8);
        self.element.style.margin = Insets::edges(
            values[0].unwrap_or(0.),
            values[1].unwrap_or(0.),
            values[2].unwrap_or(0.),
            values[3].unwrap_or(0.),
        );
        self.element.layout_rules().margin_auto = AutoMargins {
            top: values[0].is_none(),
            right: values[1].is_none(),
            bottom: values[2].is_none(),
            left: values[3].is_none(),
        };
        self
    }
    /// Sets one margin edge (`0` top, `1` right, `2` bottom, `3` left); `None` means `auto`.
    ///
    /// # Panics
    ///
    /// Panics if `edge` is greater than 3.
    pub fn css_margin_edge(mut self, edge: usize, value: Option<f32>) -> Self {
        self.clear_spacing(4 + edge..5 + edge);
        let n = value.unwrap_or(0.);
        let auto = value.is_none();
        match edge {
            0 => {
                self.element.style.margin.top = n;
                self.element.layout_rules().margin_auto.top = auto;
            }
            1 => {
                self.element.style.margin.right = n;
                self.element.layout_rules().margin_auto.right = auto;
            }
            2 => {
                self.element.style.margin.bottom = n;
                self.element.layout_rules().margin_auto.bottom = auto;
            }
            3 => {
                self.element.style.margin.left = n;
                self.element.layout_rules().margin_auto.left = auto;
            }
            _ => unreachable!(),
        }
        self
    }
    /// Sets one margin (`margin == true`) or padding edge; `edge` is `"top"`,
    /// `"right"`, `"bottom"` or `"left"`.
    ///
    /// # Panics
    ///
    /// Panics on any other edge name.
    pub fn inset_edge(mut self, margin: bool, edge: &str, value: f32) -> Self {
        let target = if margin {
            &mut self.element.style.margin
        } else {
            &mut self.element.style.padding
        };
        let index = match edge {
            "top" => {
                target.top = value;
                0
            }
            "right" => {
                target.right = value;
                1
            }
            "bottom" => {
                target.bottom = value;
                2
            }
            "left" => {
                target.left = value;
                3
            }
            _ => unreachable!("compile-time edge"),
        };
        let index = index + if margin { 4 } else { 0 };
        self.clear_spacing(index..index + 1);
        if !margin {
            self.element.layout_rules();
            let padding = self.element.style.padding;
            self.properties().padding = Some(padding);
        }
        self
    }
    /// Sets one padding, margin or gap slot from an expression (`padding-top: 1em`,
    /// `gap: calc(1rem + 2px)`), evaluated while styles resolve against the element's
    /// font size, the root font size and the viewport; see
    /// [`SpacingExpressions`](crate::ui::styling::SpacingExpressions) for the slot
    /// indices. The expression replaces the pixel value declared before it.
    pub fn spacing_expression(
        mut self,
        (index, value): (usize, crate::ui::layout::ComputedLength),
    ) -> Self {
        self.properties().layout.spacing.set(index, value);
        self
    }
    /// Un espacement en px écarte les expressions déclarées avant lui sur ses côtés.
    pub(super) fn clear_spacing(&mut self, slots: std::ops::Range<usize>) {
        if let Some(styles) = self.element.styles.as_deref_mut() {
            for index in slots {
                styles.normal.layout.spacing.clear(index);
            }
        }
    }
    /// Sets the positioning scheme (static, relative, absolute…).
    pub fn position(mut self, value: Position) -> Self {
        self.element.layout_rules().position = value;
        self
    }
    /// Sets one positioning offset (`0` top, `1` right, `2` bottom, `3` left).
    pub fn offset(mut self, edge: usize, value: Length) -> Self {
        self.element.layout_rules().offsets[edge] = Some(value);
        self
    }
    /// Sets the stacking order among siblings.
    pub fn z_index(mut self, value: i32) -> Self {
        self.element.layout_rules().z_index = value;
        self
    }
    /// Marks the node as an inline-level box (`display: inline-block`): a block parent
    /// gives it its natural width instead of stretching it.
    pub fn inline_level(mut self, value: bool) -> Self {
        self.element.layout_rules().inline_level = value;
        self
    }
    /// Removes the node from layout, painting, hit testing, focus and accessibility
    /// (`display: none`) while `value` is `true`.
    pub fn hidden(mut self, value: bool) -> Self {
        self.properties().layout.hidden = Some(value);
        self
    }
    /// Sets the layout order among siblings (`order`).
    pub fn order(mut self, value: i32) -> Self {
        self.element.layout_rules().order = value;
        self
    }
}
