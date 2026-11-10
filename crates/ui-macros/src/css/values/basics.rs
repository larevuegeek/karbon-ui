pub(super) use super::super::parser::split;
pub(super) use super::effects::transition;
pub(super) use super::gradients::background;
use super::*;
pub(super) use std::collections::BTreeMap;
pub type Result<T> = std::result::Result<T, String>;
pub type Properties = Vec<(String, String)>;
pub(super) const ROOT: &str = "__ui_root::ui";
pub fn number(value: &str) -> Result<f32> {
    let n = value
        .trim()
        .parse::<f32>()
        .map_err(|_| format!("Invalid number '{value}'"))?;
    if n.is_finite() {
        Ok(n)
    } else {
        Err("CSS numbers must be finite".into())
    }
}
pub fn px(value: &str, negative: bool) -> Result<f32> {
    let value = value.trim();
    let n = if value == "0" {
        0.
    } else {
        number(
            value
                .strip_suffix("px")
                .ok_or_else(|| format!("Expected px or 0, received '{value}'"))?,
        )?
    };
    if !negative && n < 0. {
        return Err("Negative spacing is not implemented".into());
    }
    Ok(n)
}
/// Espacement (`padding`, `margin`, `gap`…) dont un mot n'est ni en px ni `auto` :
/// compilé en expression évaluée à la résolution des styles.
pub fn spacing_expression(name: &str, value: &str) -> bool {
    properties::SPACING.contains(&name)
        && !words(value).is_ok_and(|w| w.iter().all(|w| w == "auto" || px(w, true).is_ok()))
}
pub fn words(value: &str) -> Result<Vec<String>> {
    let normalized = value.replace(['\r', '\n', '\t'], " ");
    Ok(split(&normalized, ' ')?
        .into_iter()
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect())
}
pub(super) fn quoted(value: &str) -> Result<String> {
    let v = value.trim();
    if v.starts_with('"') && v.ends_with('"') {
        crate::input::files::string(v)
    } else if v.starts_with('\'') && v.ends_with('\'') {
        let inner = &v[1..v.len() - 1];
        if inner.contains('\\') {
            return Err("Use a double-quoted CSS string for escapes".into());
        }
        Ok(inner.into())
    } else {
        Err("Expected a quoted CSS string".into())
    }
}
pub fn angle(value: &str) -> Result<f32> {
    let angle = if let Some(v) = value.strip_suffix("deg") {
        number(v)
    } else if let Some(v) = value.strip_suffix("turn") {
        Ok(number(v)? * 360.)
    } else if let Some(v) = value.strip_suffix("rad") {
        Ok(number(v)?.to_degrees())
    } else if value == "0" {
        Ok(0.)
    } else {
        Err("Use deg, turn or rad for angles".into())
    }?;
    if angle.is_finite() {
        Ok(angle)
    } else {
        Err("CSS angle exceeds the native numeric range".into())
    }
}

/// Couleur constante, `None` pour `currentColor` seul ; erreur si elle dépend de l'exécution.
pub fn constant_color(value: &str) -> Result<Option<[f32; 4]>> {
    let ast = colors::parse_color(value, None, 0)?;
    if matches!(ast, colors::Ast::Current) {
        return Ok(None);
    }
    colors::constant(&ast).map(Some).ok_or_else(|| {
        "colours depending on currentColor or variables are not supported here".into()
    })
}
