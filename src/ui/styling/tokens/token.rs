//! Jetons pré-découpés des valeurs CSS ([`Token`], [`Unit`]) et parcours de leurs
//! groupes de fonctions.
use crate::ui::theme::Color;

/// Unit of a [`Token::Dimension`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    /// Logical pixels.
    Px,
    /// Multiple of the root font size.
    Rem,
    /// Multiple of the element font size.
    Em,
    /// Percentage of the viewport width.
    Vw,
    /// Percentage of the viewport height.
    Vh,
    /// Smaller viewport dimension percentage.
    Vmin,
    /// Larger viewport dimension percentage.
    Vmax,
    /// Degrees.
    Deg,
    /// Turns.
    Turn,
    /// Radians.
    Rad,
    /// Milliseconds.
    Ms,
    /// Seconds.
    S,
    /// Grid fraction.
    Fr,
    /// Points (4/3 px).
    Pt,
    /// Width of `0`, approximated as half the font size.
    Ch,
}
/// Pre-tokenized CSS component value, produced by [`css_file!`](crate::css_file) in
/// framework mode.
///
/// Functions and `var()` open a group closed by [`Token::Close`]; the optional fallback
/// of `var(--name, fallback)` follows a [`Token::Comma`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    /// Unitless number.
    Number(f32),
    /// Percentage, as written (`50%` is `50.0`).
    Percentage(f32),
    /// Number with a unit.
    Dimension(f32, Unit),
    /// Identifier (`solid`, `auto`, `inset`…).
    Ident(&'static str),
    /// Colour resolved by the compiler (hex or named colour).
    Color(Color),
    /// Quoted string, unescaped.
    String(&'static str),
    /// `,`.
    Comma,
    /// `/`.
    Slash,
    /// Other delimiter (`+`, `-`, `*` in `calc()`).
    Delim(char),
    /// Start of a function call (`rgba(`, `calc(`…), closed by [`Token::Close`].
    Function(&'static str),
    /// Start of `var(--name`, closed by [`Token::Close`].
    Var(&'static str),
    /// End of a function or `var()` group.
    Close,
    /// `url("data:image/svg+xml,…")`, compiled by the stylesheet compiler.
    Image(&'static crate::ui::rendering::svg::SvgImage),
}
/// Fin du groupe ouvert juste avant `start` (index du `Close` correspondant).
pub(super) fn group_end(tokens: &[Token], start: usize) -> usize {
    let mut depth = 1;
    let mut i = start;
    while i < tokens.len() {
        match tokens[i] {
            Token::Function(_) | Token::Var(_) => depth += 1,
            Token::Close => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    tokens.len()
}
/// Sépare une liste aux virgules de niveau 0.
pub(super) fn layers(tokens: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Function(_) | Token::Var(_) => depth += 1,
            Token::Close => depth -= 1,
            Token::Comma if depth == 0 => {
                out.push(&tokens[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&tokens[start..]);
    out
}
