pub(super) use crate::ui::{
    layout::{Expression, UnitContext},
    styling::{CssVariables, VariableValue},
    theme::Color,
};

/// Colour space used by `color-mix()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MixSpace {
    /// Gamma-encoded sRGB.
    Srgb,
    /// OKLCH, hues interpolated along the shorter arc.
    Oklch,
    /// OKLab.
    Oklab,
}

/// Colour evaluated while styles resolve, built at compile time by
/// [`css_file!`](crate::css_file) when a colour depends on `currentColor` or on a
/// custom property (`oklch(70% 0.1 var(--h))`).
///
/// Channel expressions are plain numbers: percentages are converted by the compiler
/// (`70%` lightness becomes `0.7`), hues are in degrees and alpha in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorExpression {
    /// A constant colour.
    Constant(Color),
    /// The element's computed text colour (`currentColor`).
    CurrentColor,
    /// A colour custom property, `var(--name, fallback)`.
    Variable {
        /// Property name, including the leading `--`.
        name: &'static str,
        /// Colour used when the variable is unset or not a colour.
        fallback: &'static ColorExpression,
    },
    /// `rgb(r g b / alpha)`, channels in `0..=255`.
    Rgb([&'static Expression; 4]),
    /// `hsl(h s l / alpha)`, saturation and lightness in `0..=1`.
    Hsl([&'static Expression; 4]),
    /// `oklch(L C h / alpha)`, lightness in `0..=1`.
    Oklch([&'static Expression; 4]),
    /// `oklab(L a b / alpha)`, lightness in `0..=1`.
    Oklab([&'static Expression; 4]),
    /// `color-mix(in space, first weight, second)`.
    Mix {
        /// Interpolation space.
        space: MixSpace,
        /// First colour.
        first: &'static ColorExpression,
        /// Second colour.
        second: &'static ColorExpression,
        /// Share of the first colour, `0..=1`.
        weight: &'static Expression,
        /// Alpha multiplier when the percentages add up to less than 100%.
        alpha: &'static Expression,
    },
}

impl ColorExpression {
    /// Evaluates the colour; `current` is the computed text colour and `variables`
    /// the custom properties in scope. Non-finite channels give transparent black.
    pub fn evaluate(self, current: Color, variables: Option<&CssVariables>) -> Color {
        let rgba = self.linear(current, variables, 0);
        if rgba.iter().any(|n| !n.is_finite()) {
            return Color::rgba(0, 0, 0, 0);
        }
        to_color(rgba)
    }
    /// Couleur sRGB (canaux 0..1, alpha 0..1) ; profondeur bornée contre les cycles.
    pub(super) fn linear(
        self,
        current: Color,
        vars: Option<&CssVariables>,
        depth: usize,
    ) -> [f32; 4] {
        if depth > 32 {
            return [f32::NAN; 4];
        }
        // Les pourcentages d'un canal se rapportent à sa valeur de référence (100 %).
        let r =
            |e: &Expression, reference: f32| e.evaluate(UnitContext::default(), reference, vars);
        let n = |e: &Expression| r(e, 1.);
        match self {
            Self::Constant(c) => from_color(c),
            Self::CurrentColor => from_color(current),
            Self::Variable { name, fallback } => match vars.and_then(|v| v.get(name)) {
                Some(VariableValue::Color(c)) => from_color(*c),
                Some(value @ (VariableValue::Tokens(_) | VariableValue::Static(_))) => {
                    match super::super::tokens::color_of(value, current, vars) {
                        Some(c) => from_color(c),
                        None => fallback.linear(current, vars, depth + 1),
                    }
                }
                _ => fallback.linear(current, vars, depth + 1),
            },
            Self::Rgb(c) => [
                r(c[0], 255.) / 255.,
                r(c[1], 255.) / 255.,
                r(c[2], 255.) / 255.,
                n(c[3]),
            ],
            Self::Hsl(c) => {
                let [x, y, z] = hsl([n(c[0]), n(c[1]), n(c[2])]);
                [x, y, z, n(c[3])]
            }
            Self::Oklch(c) => {
                let [x, y, z] = oklab_to_srgb(oklch_to_oklab([n(c[0]), r(c[1], 0.4), n(c[2])]));
                [x, y, z, n(c[3])]
            }
            Self::Oklab(c) => {
                let [x, y, z] = oklab_to_srgb([n(c[0]), r(c[1], 0.4), r(c[2], 0.4)]);
                [x, y, z, n(c[3])]
            }
            Self::Mix {
                space,
                first,
                second,
                weight,
                alpha,
            } => {
                let a = first.linear(current, vars, depth + 1);
                let b = second.linear(current, vars, depth + 1);
                let mut out = mix(space, a, b, n(weight).clamp(0., 1.));
                out[3] *= n(alpha).clamp(0., 1.);
                out
            }
        }
    }
    /// Returns `true` when every constant channel is finite and the tree stays within
    /// 32 levels.
    pub fn valid(self) -> bool {
        fn walk(e: ColorExpression, depth: usize) -> bool {
            if depth > 32 {
                return false;
            }
            match e {
                ColorExpression::Constant(_) | ColorExpression::CurrentColor => true,
                ColorExpression::Variable { name, fallback } => {
                    name.starts_with("--") && walk(*fallback, depth + 1)
                }
                ColorExpression::Rgb(c)
                | ColorExpression::Hsl(c)
                | ColorExpression::Oklch(c)
                | ColorExpression::Oklab(c) => c.iter().all(|e| e.valid()),
                ColorExpression::Mix {
                    first,
                    second,
                    weight,
                    alpha,
                    ..
                } => {
                    walk(*first, depth + 1)
                        && walk(*second, depth + 1)
                        && weight.valid()
                        && alpha.valid()
                }
            }
        }
        walk(self, 0)
    }
}

pub(super) fn from_color(c: Color) -> [f32; 4] {
    [
        c.red as f32 / 255.,
        c.green as f32 / 255.,
        c.blue as f32 / 255.,
        c.alpha as f32 / 255.,
    ]
}
pub(super) fn to_color(c: [f32; 4]) -> Color {
    let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    Color::rgba(q(c[0]), q(c[1]), q(c[2]), q(c[3]))
}
/// Mixes two colours like CSS `color-mix(in <space>, a <pa>, b <pb>)`, for colours known
/// only at run time (the compiler evaluates constant `color-mix()` itself).
///
/// Percentages are in `0..=100` and follow the CSS rules: a missing one completes the
/// other to 100 %, both missing mean 50 % each, and a sum below 100 % lowers the alpha.
/// Channels mix premultiplied by alpha; OKLCH hues take the shorter arc.
///
/// ```
/// use karbon_ui::ui::{styling::{color_mix, MixSpace}, theme::Color};
///
/// // `color-mix(in oklab, #0ea5e9 85%, white)`
/// let light = color_mix(MixSpace::Oklab, Color::hex(0x0ea5e9), Some(85.), Color::hex(0xffffff), None);
/// assert!(light.red > 0x0e && light.alpha == 255);
/// ```
pub fn color_mix(space: MixSpace, a: Color, pa: Option<f32>, b: Color, pb: Option<f32>) -> Color {
    let clamp = |p: Option<f32>| p.map(|p| p.clamp(0., 100.) / 100.);
    let (pa, pb) = match (clamp(pa), clamp(pb)) {
        (Some(x), Some(y)) => (x, y),
        (Some(x), None) => (x, 1. - x),
        (None, Some(y)) => (1. - y, y),
        (None, None) => (0.5, 0.5),
    };
    let total = pa + pb;
    if total <= 0. {
        return Color::rgba(0, 0, 0, 0);
    }
    mix_colors(space, a, b, pa / total, total.min(1.))
}
/// `color-mix()` de deux couleurs : part `w` de `a`, alpha multiplié par `alpha`.
pub(crate) fn mix_colors(space: MixSpace, a: Color, b: Color, w: f32, alpha: f32) -> Color {
    let mut out = mix(space, from_color(a), from_color(b), w.clamp(0., 1.));
    out[3] *= alpha.clamp(0., 1.);
    to_color(out)
}
/// `hsl()` vers sRGB ; teinte en degrés, saturation et luminosité dans 0..1.
pub(crate) fn hsl([h, s, l]: [f32; 3]) -> [f32; 3] {
    let h = h.rem_euclid(360.);
    let (s, l) = (s.clamp(0., 1.), l.clamp(0., 1.));
    let f = |n: f32| {
        let k = (n + h / 30.) % 12.;
        let a = s * l.min(1. - l);
        l - a * (k - 3.).min(9. - k).clamp(-1., 1.)
    };
    [f(0.), f(8.), f(4.)]
}
pub(super) fn oklch_to_oklab([l, c, h]: [f32; 3]) -> [f32; 3] {
    let h = h.to_radians();
    [l, c.max(0.) * h.cos(), c.max(0.) * h.sin()]
}
pub(super) fn encode(c: f32) -> f32 {
    if c.abs() <= 0.003_130_8 {
        12.92 * c
    } else {
        c.signum() * (1.055 * c.abs().powf(1. / 2.4) - 0.055)
    }
}
pub(super) fn decode(c: f32) -> f32 {
    if c.abs() <= 0.040_45 {
        c / 12.92
    } else {
        c.signum() * ((c.abs() + 0.055) / 1.055).powf(2.4)
    }
}
/// OKLab vers sRGB encodé (non borné ; la conversion en octets borne chaque canal).
pub(crate) fn oklab_to_srgb([l, a, b]: [f32; 3]) -> [f32; 3] {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        encode(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
        encode(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
        encode(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
    ]
}
pub(super) fn srgb_to_oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
    let (r, g, b) = (decode(r), decode(g), decode(b));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}
/// Mélange prémultiplié de `a` (part `w`) et `b`, comme `color-mix()`.
pub(super) fn mix(space: MixSpace, a: [f32; 4], b: [f32; 4], w: f32) -> [f32; 4] {
    let (wa, wb) = (w, 1. - w);
    let alpha = a[3] * wa + b[3] * wb;
    if alpha <= 0. {
        return [0.; 4];
    }
    let blend = |x: f32, y: f32| (x * a[3] * wa + y * b[3] * wb) / alpha;
    match space {
        MixSpace::Srgb => [
            blend(a[0], b[0]),
            blend(a[1], b[1]),
            blend(a[2], b[2]),
            alpha,
        ],
        MixSpace::Oklab => {
            let (p, q) = (
                srgb_to_oklab([a[0], a[1], a[2]]),
                srgb_to_oklab([b[0], b[1], b[2]]),
            );
            let [r, g, bl] =
                oklab_to_srgb([blend(p[0], q[0]), blend(p[1], q[1]), blend(p[2], q[2])]);
            [r, g, bl, alpha]
        }
        MixSpace::Oklch => {
            let lch = |c: [f32; 4]| {
                let [l, x, y] = srgb_to_oklab([c[0], c[1], c[2]]);
                [l, x.hypot(y), y.atan2(x).to_degrees()]
            };
            let (p, q) = (lch(a), lch(b));
            // Teinte absente d'une couleur achromatique : celle de l'autre.
            let (hp, hq) = match (p[1] < 1e-4, q[1] < 1e-4) {
                (true, false) => (q[2], q[2]),
                (false, true) => (p[2], p[2]),
                _ => (p[2], q[2]),
            };
            let mut delta = (hq - hp).rem_euclid(360.);
            if delta > 180. {
                delta -= 360.;
            }
            let hue = hp + delta * wb;
            let [r, g, bl] =
                oklab_to_srgb(oklch_to_oklab([blend(p[0], q[0]), blend(p[1], q[1]), hue]));
            [r, g, bl, alpha]
        }
    }
}
