//! Couleurs CSS (`#hex`, `rgb()`, `hsl()`, `oklch()`, `color-mix()`, `currentColor`,
//! variables) : constantes repliées à la compilation ou expressions évaluées à l'exécution.
use super::*;
/// Couleur analysée : constante (sRGB 0..1 et alpha), ou arbre évalué à l'exécution
/// quand elle dépend de `currentColor` ou d'une variable.
#[derive(Clone, Debug)]
pub(crate) enum Ast {
    Const([f32; 4]),
    Current,
    Var {
        name: String,
        fallback: Box<Ast>,
    },
    Func {
        kind: &'static str,
        channels: Vec<Chan>,
    },
    Mix {
        space: &'static str,
        a: Box<Ast>,
        b: Box<Ast>,
        weight: f32,
        alpha: f32,
    },
}
/// Canal d'une fonction de couleur : nombre final, ou code d'`Expression`.
#[derive(Clone, Debug)]
pub(crate) enum Chan {
    N(f32),
    E(String),
}
const COLOR_EXPR: &str = "__ui_root::ui::styling::ColorExpression";
/// Référence des pourcentages de chaque canal (100 % = valeur donnée).
fn references(kind: &str) -> [f32; 4] {
    match kind {
        "Rgb" => [255., 255., 255., 1.],
        "Oklch" => [1., 0.4, 1., 1.],
        "Oklab" => [1., 0.4, 0.4, 1.],
        _ => [1., 1., 1., 1.],
    }
}
fn named(v: &str) -> Option<u32> {
    Some(match v {
        "black" => 0x000000,
        "white" => 0xffffff,
        "red" => 0xff0000,
        "green" => 0x008000,
        "blue" => 0x0000ff,
        "yellow" => 0xffff00,
        "cyan" | "aqua" => 0x00ffff,
        "magenta" | "fuchsia" => 0xff00ff,
        "gray" | "grey" => 0x808080,
        "silver" => 0xc0c0c0,
        "maroon" => 0x800000,
        "olive" => 0x808000,
        "lime" => 0x00ff00,
        "teal" => 0x008080,
        "navy" => 0x000080,
        "purple" => 0x800080,
        "orange" => 0xffa500,
        "rebeccapurple" => 0x663399,
        _ => return None,
    })
}
/// Analyse une couleur CSS. Avec `vars`, `var()` reste une référence évaluée à
/// l'exécution (repli : valeur de `:root` ou repli explicite) ; sans, elle est refusée.
pub(crate) fn parse_color(
    value: &str,
    vars: Option<&BTreeMap<String, String>>,
    depth: usize,
) -> Result<Ast> {
    if depth > 16 {
        return Err("Color nesting exceeds 16".into());
    }
    let raw = value.trim();
    let v = raw.to_ascii_lowercase();
    if let Some(hex) = v.strip_prefix('#') {
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid CSS hexadecimal color".into());
        }
        let full = match hex.len() {
            3 | 4 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
            6 | 8 => hex.into(),
            _ => return Err("Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA".into()),
        };
        let channel = |at| u8::from_str_radix(&full[at..at + 2], 16).unwrap() as f32 / 255.;
        return Ok(Ast::Const([
            channel(0),
            channel(2),
            channel(4),
            if full.len() == 8 { channel(6) } else { 1. },
        ]));
    }
    if v == "transparent" {
        return Ok(Ast::Const([0.; 4]));
    }
    if v == "currentcolor" {
        return Ok(Ast::Current);
    }
    if let Some(rgb) = named(&v) {
        let c = |shift: u32| ((rgb >> shift) & 0xff) as f32 / 255.;
        return Ok(Ast::Const([c(16), c(8), c(0), 1.]));
    }
    let Some((function, rest)) = raw.split_once('(') else {
        return Err(format!(
            "Unsupported color '{value}'; use hex, a named color, rgb(), hsl(), oklch(), oklab(), color-mix() or currentColor"
        ));
    };
    let inner = rest.strip_suffix(')').ok_or("Unclosed color function")?;
    let function = function.trim().to_ascii_lowercase();
    if function == "var" {
        let Some(vars) = vars else {
            return Err("Unresolved var() in color".into());
        };
        let args = split(inner, ',')?;
        let name = args[0].trim();
        if !name.starts_with("--") || args.len() > 2 {
            return Err("Use var(--name[, fallback])".into());
        }
        let selected = args
            .get(1)
            .copied()
            .or_else(|| vars.get(name).map(String::as_str))
            .ok_or_else(|| {
                format!("Unknown CSS variable '{name}': declare it in :root or give a fallback")
            })?;
        let fallback = parse_color(selected, Some(vars), depth + 1)?;
        return Ok(Ast::Var {
            name: name.into(),
            fallback: Box::new(fallback),
        });
    }
    if function == "color-mix" {
        let parts = split(inner, ',')?;
        if parts.len() != 3 {
            return Err("Use color-mix(in srgb|oklch|oklab, color [p%], color [p%])".into());
        }
        let space = match words(parts[0])?.as_slice() {
            [i, s] if i == "in" && s == "srgb" => "Srgb",
            [i, s] if i == "in" && s == "oklch" => "Oklch",
            [i, s] if i == "in" && s == "oklab" => "Oklab",
            [i, s, h, hue] if i == "in" && s == "oklch" && h == "shorter" && hue == "hue" => {
                "Oklch"
            }
            _ => return Err("color-mix supports in srgb, in oklch and in oklab".into()),
        };
        let item = |part: &str| -> Result<(Ast, Option<f32>)> {
            let tokens = words(part)?;
            let (color, percent) = match tokens.as_slice() {
                [c] => (c.as_str(), None),
                [c, p] if p.ends_with('%') => (c.as_str(), Some(p.as_str())),
                [p, c] if p.ends_with('%') => (c.as_str(), Some(p.as_str())),
                _ => return Err("Use a color and an optional percentage in color-mix".into()),
            };
            let percent = percent
                .map(|p| {
                    let n = number(&p[..p.len() - 1])?;
                    if (0. ..=100.).contains(&n) {
                        Ok(n)
                    } else {
                        Err("color-mix percentage outside 0..100%".to_string())
                    }
                })
                .transpose()?;
            Ok((parse_color(color, vars, depth + 1)?, percent))
        };
        let (a, pa) = item(parts[1])?;
        let (b, pb) = item(parts[2])?;
        let (pa, pb) = match (pa, pb) {
            (None, None) => (50., 50.),
            (Some(p), None) => (p, 100. - p),
            (None, Some(p)) => (100. - p, p),
            (Some(a), Some(b)) => (a, b),
        };
        let sum = pa + pb;
        if sum <= 0. {
            return Err("color-mix percentages add up to zero".into());
        }
        return Ok(Ast::Mix {
            space,
            a: Box::new(a),
            b: Box::new(b),
            weight: pa / sum,
            alpha: (sum / 100.).min(1.),
        });
    }
    let kind = match function.as_str() {
        "rgb" | "rgba" => "Rgb",
        "hsl" | "hsla" => "Hsl",
        "oklch" => "Oklch",
        "oklab" => "Oklab",
        _ => {
            return Err(format!(
                "Color function {function}() is not implemented; use rgb(), hsl(), oklch(), oklab() or color-mix()"
            ));
        }
    };
    let comma = split(inner, ',')?;
    let mut items: Vec<String> = if comma.len() > 1 {
        if kind == "Oklch" || kind == "Oklab" {
            return Err(format!("{function}() uses space-separated channels"));
        }
        comma.iter().map(|s| s.trim().to_owned()).collect()
    } else {
        let slash = split(inner, '/')?;
        if slash.len() > 2 {
            return Err("Use a single / before alpha".into());
        }
        let mut items = words(slash[0])?;
        if let Some(alpha) = slash.get(1) {
            if items.len() != 3 {
                return Err(format!("{function}() needs three channels before /"));
            }
            items.push(alpha.trim().to_owned());
        }
        items
    };
    if items.len() == 3 {
        items.push("1".into());
    }
    if items.len() != 4 {
        return Err(format!(
            "{function}() needs three channels and an optional alpha"
        ));
    }
    let reference = references(kind);
    let mut channels = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let hue = i == 0 && kind == "Hsl" || i == 2 && kind == "Oklch";
        channels.push(if item.contains('(') {
            let Some(vars) = vars else {
                return Err("Unresolved var() in color".into());
            };
            Chan::E(expression::parse_number_with_vars(item, vars)?)
        } else if item == "none" {
            Chan::N(0.)
        } else if hue {
            Chan::N(if item.chars().last().is_some_and(|c| c.is_ascii_digit()) {
                number(item)?
            } else {
                angle(item)?
            })
        } else if let Some(p) = item.strip_suffix('%') {
            Chan::N(number(p)? / 100. * reference[i])
        } else {
            Chan::N(number(item)?)
        });
    }
    // Bornes des canaux constants, comme le compilateur l'imposait déjà pour rgb().
    for (i, c) in channels.iter().enumerate() {
        if let Chan::N(n) = c {
            let range = match (kind, i) {
                (_, 3) => 0. ..=1.,
                ("Rgb", _) => 0. ..=255.,
                ("Hsl", 1 | 2) | ("Oklch" | "Oklab", 0) => 0. ..=1.,
                ("Oklch", 1) => 0. ..=1.,
                ("Oklab", _) => -1. ..=1.,
                _ => f32::MIN..=f32::MAX,
            };
            if !range.contains(n) {
                return Err(format!("{function}() channel {} out of range", i + 1));
            }
        }
    }
    Ok(Ast::Func { kind, channels })
}
fn hsl_rgb([h, s, l]: [f32; 3]) -> [f32; 3] {
    let h = h.rem_euclid(360.);
    let f = |n: f32| {
        let k = (n + h / 30.) % 12.;
        let a = s * l.min(1. - l);
        l - a * (k - 3.).min(9. - k).clamp(-1., 1.)
    };
    [f(0.), f(8.), f(4.)]
}
fn encode(c: f32) -> f32 {
    if c.abs() <= 0.003_130_8 {
        12.92 * c
    } else {
        c.signum() * (1.055 * c.abs().powf(1. / 2.4) - 0.055)
    }
}
fn decode(c: f32) -> f32 {
    if c.abs() <= 0.040_45 {
        c / 12.92
    } else {
        c.signum() * ((c.abs() + 0.055) / 1.055).powf(2.4)
    }
}
fn oklab_rgb([l, a, b]: [f32; 3]) -> [f32; 3] {
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
fn rgb_oklab([r, g, b]: [f32; 3]) -> [f32; 3] {
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
fn lch_lab([l, c, h]: [f32; 3]) -> [f32; 3] {
    let h = h.to_radians();
    [l, c.max(0.) * h.cos(), c.max(0.) * h.sin()]
}
/// Mélange prémultiplié (même calcul que `ColorExpression::Mix` à l'exécution).
fn mix_const(space: &str, a: [f32; 4], b: [f32; 4], w: f32) -> [f32; 4] {
    let (wa, wb) = (w, 1. - w);
    let alpha = a[3] * wa + b[3] * wb;
    if alpha <= 0. {
        return [0.; 4];
    }
    let blend = |x: f32, y: f32| (x * a[3] * wa + y * b[3] * wb) / alpha;
    let [r, g, bl] = match space {
        "Srgb" => [blend(a[0], b[0]), blend(a[1], b[1]), blend(a[2], b[2])],
        "Oklab" => {
            let (p, q) = (rgb_oklab([a[0], a[1], a[2]]), rgb_oklab([b[0], b[1], b[2]]));
            oklab_rgb([blend(p[0], q[0]), blend(p[1], q[1]), blend(p[2], q[2])])
        }
        _ => {
            let lch = |c: [f32; 4]| {
                let [l, x, y] = rgb_oklab([c[0], c[1], c[2]]);
                [l, x.hypot(y), y.atan2(x).to_degrees()]
            };
            let (p, q) = (lch(a), lch(b));
            let (hp, hq) = match (p[1] < 1e-4, q[1] < 1e-4) {
                (true, false) => (q[2], q[2]),
                (false, true) => (p[2], p[2]),
                _ => (p[2], q[2]),
            };
            let mut delta = (hq - hp).rem_euclid(360.);
            if delta > 180. {
                delta -= 360.;
            }
            oklab_rgb(lch_lab([
                blend(p[0], q[0]),
                blend(p[1], q[1]),
                hp + delta * wb,
            ]))
        }
    };
    [r, g, bl, alpha]
}
/// Valeur constante d'une couleur, ou `None` si elle dépend de l'exécution.
pub(crate) fn constant(ast: &Ast) -> Option<[f32; 4]> {
    match ast {
        Ast::Const(c) => Some(*c),
        Ast::Current | Ast::Var { .. } => None,
        Ast::Func { kind, channels } => {
            let mut n = [0.; 4];
            for (i, c) in channels.iter().enumerate() {
                n[i] = match c {
                    Chan::N(v) => *v,
                    Chan::E(_) => return None,
                };
            }
            let [r, g, b] = match *kind {
                "Rgb" => [n[0] / 255., n[1] / 255., n[2] / 255.],
                "Hsl" => hsl_rgb([n[0], n[1], n[2]]),
                "Oklch" => oklab_rgb(lch_lab([n[0], n[1], n[2]])),
                _ => oklab_rgb([n[0], n[1], n[2]]),
            };
            Some([r, g, b, n[3]])
        }
        Ast::Mix {
            space,
            a,
            b,
            weight,
            alpha,
        } => {
            let mut c = mix_const(space, constant(a)?, constant(b)?, *weight);
            c[3] *= alpha;
            Some(c)
        }
    }
}
fn rgba_code(c: [f32; 4]) -> String {
    let q = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "{ROOT}::theme::Color::rgba({},{},{},{})",
        q(c[0]),
        q(c[1]),
        q(c[2]),
        q(c[3])
    )
}
/// Code d'une `ColorExpression` (sans référence) ; les sous-arbres constants sont
/// évalués ici.
pub(crate) fn expression_code(ast: &Ast) -> String {
    if let Some(c) = constant(ast) {
        return format!("{COLOR_EXPR}::Constant({})", rgba_code(c));
    }
    let number = |n: f32| format!("__ui_root::ui::layout::Expression::Number({n:?})");
    match ast {
        Ast::Const(_) => unreachable!(),
        Ast::Current => format!("{COLOR_EXPR}::CurrentColor"),
        Ast::Var { name, fallback } => format!(
            "{COLOR_EXPR}::Variable{{name:{name:?},fallback:&{}}}",
            expression_code(fallback)
        ),
        Ast::Func { kind, channels } => format!(
            "{COLOR_EXPR}::{kind}([{}])",
            channels
                .iter()
                .map(|c| match c {
                    Chan::N(n) => format!("&{}", number(*n)),
                    Chan::E(code) => code.clone(),
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        Ast::Mix {
            space,
            a,
            b,
            weight,
            alpha,
        } => format!(
            "{COLOR_EXPR}::Mix{{space:__ui_root::ui::styling::MixSpace::{space},first:&{},second:&{},weight:&{},alpha:&{}}}",
            expression_code(a),
            expression_code(b),
            number(*weight),
            number(*alpha)
        ),
    }
}
/// Couleur constante en code `Color` ; `currentColor` et les variables sont refusés
/// ici (seules les propriétés de couleur les évaluent à l'exécution).
pub fn color(value: &str) -> Result<String> {
    let ast = parse_color(value, None, 0)?;
    constant(&ast).map(rgba_code).ok_or_else(|| {
        "currentColor is only supported by color, background(-color), border(-color) and accent-color"
            .into()
    })
}
/// Code d'une `ColorExpression` (valeur, constante comprise) : couleur d'un arrêt de
/// dégradé ou d'une ombre évaluée à la résolution des styles.
pub(crate) fn color_expression(value: &str, vars: &BTreeMap<String, String>) -> Result<String> {
    Ok(expression_code(&parse_color(value, Some(vars), 0)?))
}
/// Valeur d'une propriété de couleur : constante, ou `ColorExpression` statique
/// (préfixe `dynamic:`) quand `currentColor` ou une variable interviennent.
pub(crate) fn color_property(
    value: &str,
    vars: Option<&BTreeMap<String, String>>,
) -> Result<String> {
    let ast = parse_color(value, vars, 0)?;
    Ok(match constant(&ast) {
        Some(c) => rgba_code(c),
        None => format!("dynamic:const{{&{}}}", expression_code(&ast)),
    })
}
