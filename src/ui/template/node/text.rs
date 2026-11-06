//! Typographie héritée par les descendants : police, couleur, alignement, décorations.
use super::Node;
use crate::ui::{element::*, font::Font, theme::Color};
impl<W: Widget + 'static> Node<W> {
    /// Sets the font family and weight.
    pub fn font(mut self, value: impl Into<Font>) -> Self {
        let value = value.into();
        let t = &mut self.properties().text;
        t.family = Some(value.family);
        t.weight = Some(value.weight);
        self
    }
    /// Sets the font family.
    pub fn font_family(mut self, value: &'static str) -> Self {
        self.properties().text.family = Some(value);
        self
    }
    /// Sets the font size in pixels.
    pub fn font_size(mut self, value: f32) -> Self {
        self.properties().font_size_expression = None;
        self.properties().text.size = Some(value);
        self
    }
    /// Sets the font size from an expression (`em`, `rem`, `vw`, `calc()`…).
    pub fn font_size_expression(mut self, value: crate::ui::layout::ComputedLength) -> Self {
        self.properties().text.size = None;
        self.properties().font_size_expression = Some(value);
        self
    }
    /// Sets the font weight (1 to 999).
    pub fn weight(mut self, value: u16) -> Self {
        self.properties().text.weight = Some(value);
        self
    }
    /// Sets the italic style (`font-style`), inherited by descendants.
    pub fn font_italic(mut self, value: bool) -> Self {
        self.properties().text.italic = Some(value);
        self
    }
    /// Sets the text color, inherited by descendants.
    pub fn color(mut self, value: Color) -> Self {
        self.properties().text.color = Some(value);
        self.properties().color_expressions[0] = None;
        self
    }
    /// Sets the horizontal text alignment.
    pub fn text_align(mut self, value: crate::ui::styling::TextAlign) -> Self {
        self.properties().text.align = Some(value);
        self
    }
    /// Sets the line height.
    pub fn line_height(mut self, value: crate::ui::styling::LineHeight) -> Self {
        self.properties().text.line_height = Some(value);
        self
    }
    /// Sets the letter spacing, inherited by descendants.
    pub fn letter_spacing(mut self, value: crate::ui::styling::LetterSpacing) -> Self {
        self.properties().text.letter_spacing = Some(value);
        self
    }
    /// Sets the case transformation, inherited by descendants.
    pub fn text_transform(mut self, value: crate::ui::styling::TextTransform) -> Self {
        self.properties().text.transform = Some(value);
        self
    }
    /// Sets line breaking (`white-space`), inherited by descendants.
    pub fn white_space(mut self, value: crate::ui::styling::WhiteSpace) -> Self {
        self.properties().text.white_space = Some(value);
        self
    }
    /// Sets the overflow marker of clipped lines (`text-overflow`).
    pub fn text_overflow(mut self, value: crate::ui::styling::TextOverflow) -> Self {
        self.properties().text.overflow = Some(value);
        self
    }
    /// Right-to-left base direction (`direction: rtl`): a truncated line ends at its
    /// start with `…`.
    pub fn direction_rtl(mut self, value: bool) -> Self {
        self.properties().text.rtl = Some(value);
        self
    }
    /// Limits text to `value` lines (`line-clamp`), `0` for no limit.
    pub fn line_clamp(mut self, value: u16) -> Self {
        self.properties().text.line_clamp = Some(value);
        self
    }
    /// Uses tabular digits (`font-variant-numeric: tabular-nums`), inherited by descendants.
    pub fn tabular_numbers(mut self, value: bool) -> Self {
        self.properties().text.tabular_numbers = Some(value);
        self
    }
    /// Sets the text decoration lines.
    pub fn text_decoration(mut self, value: crate::ui::styling::TextDecoration) -> Self {
        self.properties().text.decoration = Some(value);
        self
    }
    /// Sets the colour of the decoration lines (`text-decoration-color`).
    pub fn decoration_color(mut self, value: Color) -> Self {
        self.properties().text.decoration_color = Some(value);
        self
    }
    /// Sets the distance from the baseline to the underline in pixels
    /// (`text-underline-offset`).
    pub fn underline_offset(mut self, value: f32) -> Self {
        self.properties().text.underline_offset = Some(value);
        self
    }
    /// Sets the thickness of the decoration lines in pixels
    /// (`text-decoration-thickness`).
    pub fn decoration_thickness(mut self, value: f32) -> Self {
        self.properties().text.decoration_thickness = Some(value);
        self
    }
}
