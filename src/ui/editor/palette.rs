//! Couleurs et mesures communes aux composants d'éditeur : lignes serrées, texte de 12,
//! palette sombre ou claire, axes X, Y, Z en rouge, vert et bleu.
use crate::ui::{
    theme::{Color, Theme},
    tooltip::TooltipStyle,
};

/// Colours and sizes shared by the editor components ([`TreeView`](super::TreeView),
/// [`NumberField`](super::NumberField), [`PropertyGrid`](super::PropertyGrid),
/// [`MenuBar`](super::MenuBar)): dense rows, small text and the axis colours of 3D
/// tools. Start from [`DARK`](Self::DARK) or [`LIGHT`](Self::LIGHT) and override fields
/// with struct update syntax.
///
/// ```
/// use karbon_ui::ui::{editor::EditorPalette, theme::Color};
///
/// let palette = EditorPalette {
///     accent: Color::hex(0xE0A030),
///     ..EditorPalette::DARK
/// };
/// assert_eq!(palette.row_height, 22.);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorPalette {
    /// Background of a panel.
    pub panel: Color,
    /// Background of a field, a header and a menu.
    pub field: Color,
    /// Background of a field or a row under the pointer.
    pub hover: Color,
    /// Background of a selected row.
    pub selection: Color,
    /// Main text.
    pub text: Color,
    /// Labels and secondary text.
    pub muted: Color,
    /// Greyed-out text of a disabled entry.
    pub disabled: Color,
    /// Focus outline, drop indicator, checked marks.
    pub accent: Color,
    /// Separators and field outlines.
    pub border: Color,
    /// Colours of the X, Y and Z axes (red, green, blue).
    pub axes: [Color; 3],
    /// Height of a row, a field and a menu entry, in logical pixels.
    pub row_height: f32,
    /// Size of the text, in logical pixels.
    pub font_size: f32,
}

impl EditorPalette {
    /// Dark palette, the usual look of 3D editors.
    pub const DARK: Self = Self {
        panel: Color::hex(0x1E2024),
        field: Color::hex(0x2A2D33),
        hover: Color::hex(0x353A42),
        selection: Color::hex(0x22416B),
        text: Color::hex(0xE6E8EB),
        muted: Color::hex(0xA9AFB8),
        disabled: Color::hex(0x6A7079),
        accent: Color::hex(0x4C9BFF),
        border: Color::hex(0x3B3F46),
        axes: [
            Color::hex(0xE5534B),
            Color::hex(0x6BBF59),
            Color::hex(0x4C8DF6),
        ],
        row_height: 22.,
        font_size: 12.,
    };
    /// Light palette.
    pub const LIGHT: Self = Self {
        panel: Color::hex(0xF3F4F6),
        field: Color::hex(0xFFFFFF),
        hover: Color::hex(0xE3E7ED),
        selection: Color::hex(0xC5DAF7),
        text: Color::hex(0x1C1F24),
        muted: Color::hex(0x50565F),
        disabled: Color::hex(0x9AA0A8),
        accent: Color::hex(0x1F6FD6),
        border: Color::hex(0xC9CED6),
        axes: [
            Color::hex(0xC62828),
            Color::hex(0x2E7D32),
            Color::hex(0x1565C0),
        ],
        row_height: 22.,
        font_size: 12.,
    };

    /// The same colours as a window [`Theme`], for the parts of the library that paint
    /// with a theme (menus of a [`Popover`](crate::ui::popover::Popover), dialogs).
    pub fn theme(&self) -> Theme {
        Theme {
            background: self.panel,
            surface: self.field,
            title_bar: self.panel,
            text: self.text,
            muted: self.muted,
            accent: self.accent,
            on_accent: Color::hex(0xFFFFFF),
            border: self.border,
            hover: self.hover,
            pressed: self.selection,
            danger: Color::hex(0xE5534B),
        }
    }

    /// The look of a [`Tooltip`](crate::ui::tooltip::Tooltip) in this palette.
    pub fn tooltip(&self) -> TooltipStyle {
        TooltipStyle {
            background: Some(self.field),
            foreground: Some(self.text),
            border: Some(self.border),
            font_size: self.font_size,
        }
    }
}

impl Default for EditorPalette {
    fn default() -> Self {
        Self::DARK
    }
}

/// Contrast ratio between two opaque colours, from 1 to 21, as defined by WCAG 2: text
/// is legible from 4.5.
///
/// ```
/// use karbon_ui::ui::{editor::contrast, theme::Color};
///
/// assert!((contrast(Color::hex(0x000000), Color::hex(0xFFFFFF)) - 21.).abs() < 0.01);
/// ```
pub fn contrast(a: Color, b: Color) -> f32 {
    fn luminance(color: Color) -> f32 {
        let linear = |v: u8| {
            let c = f32::from(v) / 255.;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tout texte des composants (texte, libellés) se lit à 4,5:1 au moins sur chacun des
    /// fonds où il se pose, dans les deux palettes.
    #[test]
    fn every_text_reads_at_four_and_a_half_to_one() {
        for palette in [EditorPalette::DARK, EditorPalette::LIGHT] {
            for background in [
                palette.panel,
                palette.field,
                palette.hover,
                palette.selection,
            ] {
                for text in [palette.text, palette.muted] {
                    let ratio = contrast(text, background);
                    assert!(ratio >= 4.5, "{text:?} on {background:?}: {ratio:.2}");
                }
            }
        }
    }
}
