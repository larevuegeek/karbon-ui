//! Traduction d'une propriété CSS (valeur déjà résolue) en propriétés du moteur.
//!
//! Chaque famille (`spacing`, `boxes`, `position`, `flex_grid`, `borders`, `paint`, `text`,
//! `transforms`, `scroll`) reconnaît ses propriétés et passe les autres à la suivante ;
//! une propriété qu'aucune ne reconnaît est refusée.
mod borders;
mod boxes;
mod flex_grid;
mod paint;
mod position;
mod scroll;
mod spacing;
mod text;
mod transforms;
use super::*;

/// Famille suivante de la chaîne, appelée pour une propriété non reconnue.
type Next<'a> = &'a dyn Fn() -> Result<Properties>;
/// Famille de propriétés : traduit `name: value`, ou délègue à `next`.
type Family = fn(&str, &str, Next) -> Result<Properties>;
const FAMILIES: [Family; 9] = [
    spacing::property,
    boxes::property,
    position::property,
    flex_grid::property,
    borders::property,
    paint::property,
    text::property,
    transforms::property,
    scroll::property,
];

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
/// Propriété de couleur : constante sous `field`, ou `ColorExpression` évaluée à la
/// résolution des styles sous `color_expression_<target>`.
pub(super) fn color_prop(
    field: &str,
    target: &str,
    value: &str,
    vars: Option<&BTreeMap<String, String>>,
) -> Result<Properties> {
    let code = colors::color_property(value, vars)?;
    Ok(vec![match code.strip_prefix("dynamic:") {
        Some(expr) => (format!("color_expression_{target}"), expr.into()),
        None => (field.into(), code),
    }])
}
pub(super) use spacing::NAMES as SPACING;
/// Espacement dont la valeur lit des variables : mots en `var()` évalués à la résolution
/// des styles.
pub(super) fn spacing_with_vars(
    name: &str,
    value: &str,
    vars: &BTreeMap<String, String>,
) -> Result<Properties> {
    spacing::spacing(name, value, Some(vars))
}
/// Propriétés du moteur pour la déclaration `name: value`.
pub fn property(name: &str, value: &str) -> Result<Properties> {
    chain(&FAMILIES, name, value)
}
fn chain(families: &[Family], name: &str, value: &str) -> Result<Properties> {
    match families.split_first() {
        Some((family, rest)) => family(name, value, &|| chain(rest, name, value)),
        None => Err(format!(
            "CSS property '{name}' is not implemented; it has not been ignored"
        )),
    }
}
/// Une seule propriété du moteur.
fn one(property: &str, value: String) -> Result<Properties> {
    Ok(vec![(property.into(), value)])
}
/// Nombre écrit comme littéral Rust `f32` (`Debug`).
fn scalar(n: f32) -> String {
    format!("{n:?}")
}
fn container_type(value: &str) -> Result<String> {
    Ok(match value.trim() {
        "normal" => "None".into(),
        "inline-size" => format!("Some({ROOT}::styling::ContainerType::InlineSize)"),
        "size" => format!("Some({ROOT}::styling::ContainerType::Size)"),
        other => {
            return Err(format!(
                "container-type: {other} is not supported; use normal, inline-size or size"
            ));
        }
    })
}
fn container_names(value: &str) -> Result<String> {
    let names = words(value)?;
    if names.iter().any(|n| n == "none") {
        return if names.len() == 1 {
            Ok("&[]".into())
        } else {
            Err("container-name: none cannot be combined with names".into())
        };
    }
    for name in &names {
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || name.starts_with(|c: char| c.is_ascii_digit())
            || ["and", "or", "not", "style"].contains(&name.as_str())
        {
            return Err(format!("Invalid container name '{name}'"));
        }
    }
    Ok(format!(
        "&[{}]",
        names
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>()
            .join(",")
    ))
}
