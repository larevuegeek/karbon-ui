//! Recettes de layout partielles : seules les propriétés renseignées sont appliquées.
use crate::ui::{
    element::{Align, Element, Insets, Justify, Length},
    theme::Color,
};

/// Partial layout recipe: only the properties that were set are applied by [`Element::style`].
#[derive(Clone, Copy, Default)]
pub struct ViewStyle {
    width: Option<Length>,
    height: Option<Length>,
    min_width: Option<f32>,
    min_height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    padding: Option<Insets>,
    margin: Option<Insets>,
    gap: Option<f32>,
    row_gap: Option<f32>,
    column_gap: Option<f32>,
    align: Option<Align>,
    justify: Option<Justify>,
    background: Option<Color>,
    radius: Option<f32>,
}
impl ViewStyle {
    /// Empty recipe.
    pub const fn new() -> Self {
        Self {
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            padding: None,
            margin: None,
            gap: None,
            row_gap: None,
            column_gap: None,
            align: None,
            justify: None,
            background: None,
            radius: None,
        }
    }
    /// Fixed width, in pixels.
    pub const fn width(mut self, value: f32) -> Self {
        self.width = Some(Length::Px(value));
        self
    }
    /// Fixed height, in pixels.
    pub const fn height(mut self, value: f32) -> Self {
        self.height = Some(Length::Px(value));
        self
    }
    /// Take all the available width.
    pub const fn fill_width(mut self) -> Self {
        self.width = Some(Length::Fill(1.));
        self
    }
    /// Take all the available height.
    pub const fn fill_height(mut self) -> Self {
        self.height = Some(Length::Fill(1.));
        self
    }
    /// Minimum width, in pixels.
    pub const fn min_width(mut self, value: f32) -> Self {
        self.min_width = Some(value);
        self
    }
    /// Minimum height, in pixels.
    pub const fn min_height(mut self, value: f32) -> Self {
        self.min_height = Some(value);
        self
    }
    /// Maximum width, in pixels.
    pub const fn max_width(mut self, value: f32) -> Self {
        self.max_width = Some(value);
        self
    }
    /// Maximum height, in pixels.
    pub const fn max_height(mut self, value: f32) -> Self {
        self.max_height = Some(value);
        self
    }
    /// Same padding on all four sides.
    pub const fn padding(mut self, value: f32) -> Self {
        self.padding = Some(Insets::all(value));
        self
    }
    /// Padding from explicit [`Insets`].
    pub const fn insets(mut self, value: Insets) -> Self {
        self.padding = Some(value);
        self
    }
    /// Spacing between children on both axes; clears `row_gap` and `column_gap`.
    pub const fn gap(mut self, value: f32) -> Self {
        self.gap = Some(value);
        self.row_gap = None;
        self.column_gap = None;
        self
    }
    /// Outer spacing.
    pub const fn margin(mut self, value: Insets) -> Self {
        self.margin = Some(value);
        self
    }
    /// Vertical spacing between rows.
    pub const fn row_gap(mut self, value: f32) -> Self {
        self.row_gap = Some(value);
        self
    }
    /// Horizontal spacing between columns.
    pub const fn column_gap(mut self, value: f32) -> Self {
        self.column_gap = Some(value);
        self
    }
    /// Cross-axis alignment of children.
    pub const fn align(mut self, value: Align) -> Self {
        self.align = Some(value);
        self
    }
    /// Main-axis distribution of children.
    pub const fn justify(mut self, value: Justify) -> Self {
        self.justify = Some(value);
        self
    }
    /// Solid background colour; replaces any gradient.
    pub const fn background(mut self, value: Color) -> Self {
        self.background = Some(value);
        self
    }
    /// Corner radius, in pixels.
    pub const fn radius(mut self, value: f32) -> Self {
        self.radius = Some(value);
        self
    }
}
impl Element {
    /// Calls compose from left to right; the last property set wins.
    pub fn style(mut self, recipe: ViewStyle) -> Self {
        macro_rules! set {
            ($($field:ident),* $(,)?) => {
                $(if let Some(value) = recipe.$field {
                    self.style.$field = value;
                })*
            };
        }
        set!(
            width, height, min_width, min_height, max_width, max_height, padding, margin, align,
            justify, radius
        );
        if let Some(gap) = recipe.gap {
            self = self.gap(gap);
        }
        if let Some(gap) = recipe.row_gap {
            self = self.row_gap(gap);
        }
        if let Some(gap) = recipe.column_gap {
            self = self.column_gap(gap);
        }
        if let Some(color) = recipe.background {
            self.gradient = None;
            self.style.background = Some(color);
        }
        self
    }
}
