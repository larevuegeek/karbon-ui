//! Propriétés CSS : espacements (`padding`, `margin`, `gap`, `row-gap`, `column-gap` et
//! leurs côtés). Les px et `auto` restent des valeurs constantes ; `em`, `rem`, `vw`,
//! `vh` et `calc()/min()/max()/clamp()` deviennent des expressions évaluées à la
//! résolution des styles (emplacements de `SpacingExpressions`).
use super::super::*;
use super::{Next, SIDES, one, scalar};

/// Valeur d'un côté.
enum Side {
    Px(f32),
    Auto,
    /// Code d'une `ComputedLength`.
    Expression(String),
}
/// Noms des propriétés de la famille.
pub(in crate::css::values) const NAMES: [&str; 13] = [
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "gap",
    "row-gap",
    "column-gap",
];

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    if NAMES.contains(&name) {
        spacing(name, value, None)
    } else {
        next()
    }
}
/// Déclaration d'espacement ; avec `vars`, les mots contenant `var()` deviennent des
/// expressions qui lisent la variable à la résolution des styles.
pub(in crate::css::values) fn spacing(
    name: &str,
    value: &str,
    vars: Option<&BTreeMap<String, String>>,
) -> Result<Properties> {
    let margin = name.starts_with("margin");
    let side = |word: &str| -> Result<Side> {
        if let Some(vars) = vars.filter(|_| word.contains("var(")) {
            if margin && resolve(word, vars)?.trim() == "auto" {
                return Ok(Side::Auto);
            }
            return expression_side(name, word, |w| expression::parse_with_vars(w, vars));
        }
        if margin && word == "auto" {
            return Ok(Side::Auto);
        }
        match px(word, margin) {
            Ok(n) => Ok(Side::Px(n)),
            Err(e) if e.starts_with("Negative") => Err(e),
            Err(_) if !margin && word.starts_with('-') => {
                Err("Negative spacing is not implemented".into())
            }
            Err(_) => expression_side(name, word, expression::parse),
        }
    };
    let sides = words(value)?
        .iter()
        .map(|w| side(w))
        .collect::<Result<Vec<_>>>()?;
    let key = name.replace('-', "_");
    match name {
        "padding" | "margin" => shorthand(name, sides),
        "gap" => match sides.as_slice() {
            [Side::Px(n)] => one("gap", scalar(*n)),
            [Side::Expression(e)] => Ok(vec![slot(8, e), slot(9, e)]),
            [row, column] => Ok(vec![gap(8, row), gap(9, column)]),
            _ => Err("gap accepts one or two lengths".into()),
        },
        "row-gap" | "column-gap" => match sides.as_slice() {
            [side] => Ok(vec![gap(if name == "row-gap" { 8 } else { 9 }, side)]),
            _ => Err(format!("{name} accepts one length")),
        },
        _ => {
            let [side] = sides.as_slice() else {
                return Err(format!("{name} accepts one length"));
            };
            let edge = SIDES
                .iter()
                .position(|s| key.ends_with(s))
                .unwrap_or_default();
            Ok(vec![match side {
                Side::Expression(e) => slot(edge + if margin { 4 } else { 0 }, e),
                Side::Auto => (format!("css_{key}"), "None".into()),
                Side::Px(n) if margin => (format!("css_{key}"), format!("Some({n:?})")),
                Side::Px(n) => (key, scalar(*n)),
            }])
        }
    }
}
/// Expression d'un côté ; les pourcentages sont refusés (pas de boîte de référence).
fn expression_side(name: &str, word: &str, parse: impl Fn(&str) -> Result<String>) -> Result<Side> {
    let code = parse(word).map_err(|e| {
        format!("{name}: '{word}' is not a spacing length ({e}); use px, em, rem, vw, vh or calc()/min()/max()/clamp() of them")
    })?;
    if code.contains("::Percent(") {
        return Err(format!(
            "{name}: percentages are not supported for spacing; use px, em, rem, vw, vh or calc()"
        ));
    }
    Ok(Side::Expression(code))
}
/// Emplacement `index` de `SpacingExpressions`.
fn slot(index: usize, code: &str) -> (String, String) {
    (
        "spacing_expression".into(),
        format!("({index}usize,{code})"),
    )
}
fn gap(index: usize, side: &Side) -> (String, String) {
    let field = if index == 8 { "row_gap" } else { "column_gap" };
    match side {
        Side::Expression(e) => slot(index, e),
        Side::Px(n) => (field.into(), scalar(*n)),
        Side::Auto => unreachable!("auto is a margin keyword"),
    }
}
/// `padding` / `margin` en 1 à 4 valeurs. Tout en px (ou `auto`) : la valeur constante
/// d'avant ; sinon les côtés en px (0 pour les autres) puis les expressions, qui
/// l'emportent sur leur côté.
fn shorthand(name: &str, sides: Vec<Side>) -> Result<Properties> {
    let margin = name == "margin";
    if !(1..=4).contains(&sides.len()) {
        return Err(if margin {
            "margin needs 1..4 values".into()
        } else {
            "Spacing shorthand needs 1..4 values".into()
        });
    }
    let constant = |side: &Side| match side {
        Side::Px(n) if margin => format!("Some({n:?})"),
        Side::Px(n) => scalar(*n),
        Side::Auto => "None".into(),
        Side::Expression(_) if margin => "Some(0.0)".into(),
        Side::Expression(_) => scalar(0.),
    };
    let edges: [&Side; 4] = match sides.as_slice() {
        [a] => [a, a, a, a],
        [a, b] => [a, b, a, b],
        [a, b, c] => [a, b, c, b],
        [a, b, c, d] => [a, b, c, d],
        _ => unreachable!(),
    };
    let expressions = edges
        .iter()
        .enumerate()
        .filter_map(|(i, side)| match side {
            Side::Expression(e) => Some(slot(i + if margin { 4 } else { 0 }, e)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    if margin {
        if expressions.len() < 4 {
            let values = edges.map(constant);
            out.push(("css_margin".into(), format!("[{}]", values.join(","))));
        }
    } else if expressions.is_empty() {
        let values = sides.iter().map(constant).collect::<Vec<_>>();
        out.push((
            "padding".into(),
            if values.len() == 1 {
                values[0].clone()
            } else {
                format!("[{}]", values.join(","))
            },
        ));
    } else if expressions.len() < 4 {
        out.push((
            "padding".into(),
            format!("[{}]", edges.map(constant).join(",")),
        ));
    }
    out.extend(expressions);
    Ok(out)
}
