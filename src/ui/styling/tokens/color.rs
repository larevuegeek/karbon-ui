//! Couleurs lues dans les jetons : `rgb()`, `hsl()`, `color-mix()`, mots-clés, et couleur
//! d'une valeur de variable.
use super::{
    reader::{Evaluation, Quantity, Reader},
    substitution::{lookup, substitute},
    token::Token,
};
use crate::ui::{
    styling::{
        CssVariables, VariableValue,
        color::{MixSpace, hsl, mix_colors},
    },
    theme::Color,
};

impl Reader<'_, '_> {
    /// Canal de couleur : nombre, pourcentage de `reference`, ou `none`.
    fn channel(&mut self, reference: f32) -> Option<f32> {
        match self.peek()? {
            Token::Ident("none") => {
                self.at += 1;
                Some(0.)
            }
            Token::Percentage(p) => {
                self.at += 1;
                Some(p / 100. * reference)
            }
            _ => match self.quantity()? {
                Quantity::Number(n) => Some(n),
                Quantity::Length(0., p) => Some(p / 100. * reference),
                Quantity::Angle(a) => Some(a),
                _ => None,
            },
        }
    }
    /// Canaux d'une fonction de couleur, séparés par virgules ou espaces, alpha après
    /// `/` ou en quatrième position.
    fn channels(&mut self, references: [f32; 3]) -> Option<[f32; 4]> {
        let mut out = [0., 0., 0., 1.];
        for (i, reference) in references.iter().enumerate() {
            if i > 0 && self.peek() == Some(Token::Comma) {
                self.at += 1;
            }
            out[i] = self.channel(*reference)?;
        }
        match self.peek() {
            Some(Token::Comma) | Some(Token::Slash) => {
                self.at += 1;
                out[3] = self.channel(1.)?;
            }
            None => {}
            _ => return None,
        }
        self.done().then_some(out)
    }
    pub(super) fn color(&mut self) -> Option<Color> {
        match self.next()? {
            Token::Color(c) => Some(c),
            Token::Ident(name) => match name.to_ascii_lowercase().as_str() {
                "transparent" => Some(Color::rgba(0, 0, 0, 0)),
                "currentcolor" => Some(self.eval.current),
                "black" => Some(Color::hex(0)),
                "white" => Some(Color::hex(0xffffff)),
                _ => None,
            },
            Token::Function(name) => {
                let args = self.arguments().to_vec();
                let mut inner = Reader {
                    tokens: &args,
                    at: 0,
                    eval: self.eval,
                    font: self.font,
                };
                let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
                match name {
                    "rgb" | "rgba" => {
                        let c = inner.channels([255., 255., 255.])?;
                        Some(Color::rgba(
                            q(c[0] / 255.),
                            q(c[1] / 255.),
                            q(c[2] / 255.),
                            q(c[3]),
                        ))
                    }
                    "hsl" | "hsla" => {
                        let c = inner.channels([360., 1., 1.])?;
                        let s = if c[1] > 1. { c[1] / 100. } else { c[1] };
                        let l = if c[2] > 1. { c[2] / 100. } else { c[2] };
                        let [r, g, b] = hsl([c[0], s, l]);
                        Some(Color::rgba(q(r), q(g), q(b), q(c[3])))
                    }
                    "color-mix" => inner.color_mix(),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    /// `color-mix(in espace, c1 [p1], c2 [p2])`.
    fn color_mix(&mut self) -> Option<Color> {
        if self.next()? != Token::Ident("in") {
            return None;
        }
        let space = match self.next()? {
            Token::Ident("srgb") => MixSpace::Srgb,
            Token::Ident("oklab") => MixSpace::Oklab,
            Token::Ident("oklch") => MixSpace::Oklch,
            _ => return None,
        };
        // Direction de teinte éventuelle (`shorter hue`).
        while let Some(Token::Ident(_)) = self.peek() {
            self.at += 1;
        }
        if self.next()? != Token::Comma {
            return None;
        }
        let stop = |r: &mut Self| -> Option<(Color, Option<f32>)> {
            let mut percent = None;
            if let Some(Token::Percentage(p)) = r.peek() {
                r.at += 1;
                percent = Some(p / 100.);
            }
            let c = r.color()?;
            if let Some(Token::Percentage(p)) = r.peek() {
                r.at += 1;
                percent = Some(p / 100.);
            }
            Some((c, percent))
        };
        let (a, pa) = stop(self)?;
        if self.next()? != Token::Comma {
            return None;
        }
        let (b, pb) = stop(self)?;
        if !self.done() {
            return None;
        }
        let (pa, pb) = match (pa, pb) {
            (Some(x), Some(y)) => (x, y),
            (Some(x), None) => (x, 1. - x),
            (None, Some(y)) => (1. - y, y),
            (None, None) => (0.5, 0.5),
        };
        let total = pa + pb;
        if total <= 0. {
            return None;
        }
        Some(mix_colors(space, a, b, pa / total, total.min(1.)))
    }
}

/// Couleur d'une valeur de variable découpée en jetons (`ColorExpression::Variable`).
pub(crate) fn color_of(
    value: &VariableValue,
    current: Color,
    variables: Option<&CssVariables>,
) -> Option<Color> {
    let mut tokens = Vec::new();
    let mut look = |n, o: &mut Vec<Token>, d| lookup(variables, None, n, o, d);
    let source: &[Token] = match value {
        VariableValue::Tokens(t) => t,
        VariableValue::Static(t) => t,
        _ => return None,
    };
    if !substitute(source, &mut look, &mut tokens, 0) {
        return None;
    }
    let eval = Evaluation {
        variables,
        globals: None,
        font: 16.,
        parent_font: 16.,
        root_font: 16.,
        viewport: (0., 0.),
        scale: 1.,
        current,
        parent_corners: [0.; 4],
    };
    let mut r = Reader {
        tokens: &tokens,
        at: 0,
        eval: &eval,
        font: 16.,
    };
    r.color().filter(|_| r.done())
}
