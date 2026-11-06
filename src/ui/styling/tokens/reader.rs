//! Contexte d'évaluation et lecteur des jetons substitués : calculs (`calc()`, `min()`…),
//! longueurs, nombres et angles.
use super::{
    substitution::Globals,
    token::{Token, Unit, group_end},
};
use crate::ui::{element::Length, styling::CssVariables, theme::Color};

/// Context of an evaluation.
#[derive(Clone, Copy)]
pub(crate) struct Evaluation<'a> {
    pub variables: Option<&'a CssVariables>,
    pub globals: Option<Globals>,
    /// Font size of the element (for `em`), and of its parent (for `font-size: 1.2em`).
    pub font: f32,
    pub parent_font: f32,
    pub root_font: f32,
    pub viewport: (f32, f32),
    /// Template scale applied to pixel lengths.
    pub scale: f32,
    /// Computed text colour (`currentColor`).
    pub current: Color,
    /// Corner radii of the parent (top-left, top-right, bottom-right, bottom-left), for
    /// `border-*-radius: inherit`.
    pub parent_corners: [f32; 4],
}
impl Evaluation<'_> {
    fn length_unit(&self, n: f32, unit: Unit, font: f32) -> Option<f32> {
        let (vw, vh) = self.viewport;
        Some(match unit {
            Unit::Px => n * self.scale,
            Unit::Rem => n * self.root_font,
            Unit::Em => n * font,
            Unit::Ch => n * font * 0.5,
            Unit::Pt => n * 4. / 3. * self.scale,
            Unit::Vw => n * vw / 100.,
            Unit::Vh => n * vh / 100.,
            Unit::Vmin => n * vw.min(vh) / 100.,
            Unit::Vmax => n * vw.max(vh) / 100.,
            _ => return None,
        })
    }
}
/// Valeur numérique typée d'un calcul.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Quantity {
    Number(f32),
    /// Longueur en px et part en pourcentage.
    Length(f32, f32),
    Angle(f32),
}
impl Quantity {
    fn add(self, other: Self, sign: f32) -> Option<Self> {
        Some(match (self, other) {
            (Self::Number(a), Self::Number(b)) => Self::Number(a + sign * b),
            (Self::Length(a, p), Self::Length(b, q)) => Self::Length(a + sign * b, p + sign * q),
            // `0` sans unité dans une somme de longueurs.
            (Self::Length(a, p), Self::Number(b)) | (Self::Number(b), Self::Length(a, p))
                if b == 0. =>
            {
                Self::Length(a, p)
            }
            (Self::Angle(a), Self::Angle(b)) => Self::Angle(a + sign * b),
            _ => return None,
        })
    }
    fn scale(self, k: f32) -> Self {
        match self {
            Self::Number(a) => Self::Number(a * k),
            Self::Length(a, p) => Self::Length(a * k, p * k),
            Self::Angle(a) => Self::Angle(a * k),
        }
    }
}
/// Lecteur séquentiel d'une liste de jetons substitués.
pub(super) struct Reader<'a, 'e> {
    pub(super) tokens: &'a [Token],
    pub(super) at: usize,
    pub(super) eval: &'a Evaluation<'e>,
    pub(super) font: f32,
}
impl Reader<'_, '_> {
    pub(super) fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }
    pub(super) fn next(&mut self) -> Option<Token> {
        let t = self.peek();
        self.at += 1;
        t
    }
    pub(super) fn done(&self) -> bool {
        self.at >= self.tokens.len()
    }
    /// Sous-lecteur des arguments d'une fonction ouverte (le `Function` est consommé).
    pub(super) fn arguments(&mut self) -> &[Token] {
        let start = self.at;
        let end = group_end(self.tokens, start);
        self.at = end + 1;
        &self.tokens[start..end.min(self.tokens.len())]
    }
    pub(super) fn quantity(&mut self) -> Option<Quantity> {
        match self.next()? {
            Token::Number(n) => Some(Quantity::Number(n)),
            Token::Percentage(p) => Some(Quantity::Length(0., p)),
            Token::Dimension(n, unit) => match unit {
                Unit::Deg => Some(Quantity::Angle(n)),
                Unit::Turn => Some(Quantity::Angle(n * 360.)),
                Unit::Rad => Some(Quantity::Angle(n.to_degrees())),
                _ => Some(Quantity::Length(
                    self.eval.length_unit(n, unit, self.font)?,
                    0.,
                )),
            },
            Token::Function(name) => {
                let args = self.arguments().to_vec();
                let mut inner = Reader {
                    tokens: &args,
                    at: 0,
                    eval: self.eval,
                    font: self.font,
                };
                match name {
                    "calc" | "" => {
                        let q = inner.sum()?;
                        inner.done().then_some(q)
                    }
                    "min" | "max" | "clamp" => {
                        let mut values = Vec::new();
                        loop {
                            values.push(inner.sum()?);
                            match inner.next() {
                                Some(Token::Comma) => continue,
                                None => break,
                                _ => return None,
                            }
                        }
                        let pick = |a: Quantity, b: Quantity, max: bool| match (a, b) {
                            (Quantity::Number(x), Quantity::Number(y)) => {
                                Some(Quantity::Number(if max { x.max(y) } else { x.min(y) }))
                            }
                            (Quantity::Length(x, 0.), Quantity::Length(y, 0.)) => {
                                Some(Quantity::Length(if max { x.max(y) } else { x.min(y) }, 0.))
                            }
                            _ => None,
                        };
                        match name {
                            "clamp" if values.len() == 3 => {
                                pick(pick(values[0], values[1], true)?, values[2], false)
                            }
                            "clamp" => None,
                            _ => values
                                .into_iter()
                                .try_fold(None, |acc: Option<Quantity>, v| {
                                    Some(Some(match acc {
                                        None => v,
                                        Some(a) => pick(a, v, name == "max")?,
                                    }))
                                })
                                .flatten(),
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    fn product(&mut self) -> Option<Quantity> {
        let mut value = self.quantity()?;
        loop {
            match self.peek() {
                Some(Token::Delim('*')) => {
                    self.at += 1;
                    let rhs = self.quantity()?;
                    value = match (value, rhs) {
                        (q, Quantity::Number(k)) => q.scale(k),
                        (Quantity::Number(k), q) => q.scale(k),
                        _ => return None,
                    };
                }
                Some(Token::Slash) | Some(Token::Delim('/')) => {
                    self.at += 1;
                    match self.quantity()? {
                        Quantity::Number(k) if k != 0. => value = value.scale(1. / k),
                        _ => return None,
                    }
                }
                _ => return Some(value),
            }
        }
    }
    fn sum(&mut self) -> Option<Quantity> {
        let mut value = self.product()?;
        loop {
            match self.peek() {
                Some(Token::Delim(c @ ('+' | '-'))) => {
                    self.at += 1;
                    let rhs = self.product()?;
                    value = value.add(rhs, if c == '+' { 1. } else { -1. })?;
                }
                // `calc(1px -2px)` est découpé en deux nombres : somme implicite refusée.
                _ => return Some(value),
            }
        }
    }
    /// Longueur en px (les pourcentages sont refusés), `0` sans unité accepté.
    pub(super) fn px(&mut self) -> Option<f32> {
        match self.quantity()? {
            Quantity::Length(px, 0.) => Some(px),
            Quantity::Number(0.) => Some(0.),
            _ => None,
        }
    }
    pub(super) fn length(&mut self) -> Option<Length> {
        if let Some(Token::Ident("auto")) = self.peek() {
            self.at += 1;
            return Some(Length::Auto);
        }
        match self.quantity()? {
            Quantity::Length(px, 0.) => Some(Length::Px(px)),
            Quantity::Length(0., p) => Some(Length::Percent(p)),
            Quantity::Number(0.) => Some(Length::Px(0.)),
            _ => None,
        }
    }
    pub(super) fn number(&mut self) -> Option<f32> {
        match self.quantity()? {
            Quantity::Number(n) => Some(n),
            _ => None,
        }
    }
    pub(super) fn angle(&mut self) -> Option<f32> {
        match self.quantity()? {
            Quantity::Angle(a) => Some(a),
            Quantity::Number(0.) => Some(0.),
            _ => None,
        }
    }
    pub(super) fn at_end_or_comma(&self) -> bool {
        matches!(self.peek(), None | Some(Token::Comma))
    }
}
