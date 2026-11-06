use crate::ui::{font::Font, theme::Color, widgets::Text};
/// Implement only the style capabilities a custom widget supports.
/// Widget that accepts a text size (template `font_size`).
pub trait TextSize: Sized {
    /// Returns the widget with its text size set to `size` pixels.
    fn text_size(self, size: f32) -> Self;
}
/// Widget that accepts a text color (template `color`).
pub trait TextColor: Sized {
    /// Returns the widget with its text color set to `color`.
    fn text_color(self, color: Color) -> Self;
}
/// Widget that accepts a complete font (template `font`).
pub trait TextFont: Sized {
    /// Returns the widget with its font set to `font`.
    fn text_font(self, font: Font) -> Self;
}
/// Widget that accepts a font family (template `font_family`).
pub trait TextFamily: Sized {
    /// Returns the widget with its font family set to `family`, keeping the weight.
    fn text_family(self, family: &'static str) -> Self;
}
/// Widget that accepts a font weight (template `weight`).
pub trait TextWeight: Sized {
    /// Returns the widget with its font weight set to `weight` (1 to 999).
    fn text_weight(self, weight: u16) -> Self;
}
impl TextSize for Text {
    fn text_size(self, size: f32) -> Self {
        self.size(size)
    }
}
impl TextColor for Text {
    fn text_color(self, color: Color) -> Self {
        self.color(color)
    }
}
impl TextFont for Text {
    fn text_font(self, font: Font) -> Self {
        self.font(font)
    }
}
impl TextFamily for Text {
    fn text_family(self, family: &'static str) -> Self {
        let weight = self
            .font
            .map_or(if self.bold { 700 } else { 400 }, |f| f.weight);
        self.font(Font::new(family, weight))
    }
}
impl TextWeight for Text {
    fn text_weight(self, weight: u16) -> Self {
        self.weight(weight)
    }
}
