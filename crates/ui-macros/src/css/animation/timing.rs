pub(super) use super::super::{parser, values};
pub(super) type Result<T> = std::result::Result<T, String>;
pub(super) const ROOT: &str = "__ui_root::ui::animation";
/// Courbe CSS (`ease`, `cubic-bezier()`…) en `Easing` du moteur.
pub(super) fn easing(value: &str) -> Result<String> {
    if let Some(args) = value
        .strip_prefix("cubic-bezier(")
        .and_then(|v| v.strip_suffix(')'))
    {
        let n = parser::split(args, ',')?
            .iter()
            .map(|a| values::number(a))
            .collect::<Result<Vec<_>>>()?;
        let [x1, y1, x2, y2] = n.as_slice() else {
            return Err("cubic-bezier() needs x1, y1, x2, y2".into());
        };
        if !(0. ..=1.).contains(x1) || !(0. ..=1.).contains(x2) {
            return Err("cubic-bezier() x values must lie in 0..1".into());
        }
        return Ok(format!(
            "{ROOT}::Easing::CubicBezier({x1:?},{y1:?},{x2:?},{y2:?})"
        ));
    }
    Ok(format!(
        "{ROOT}::Easing::{}",
        match value {
            "linear" => "Linear",
            "ease" => "Ease",
            "ease-in" => "EaseIn",
            "ease-out" => "EaseOut",
            "ease-in-out" => "EaseInOut",
            _ =>
                return Err(format!(
                    "Unsupported animation timing function '{value}' (linear, ease, ease-in, ease-out, ease-in-out, cubic-bezier())"
                )),
        }
    ))
}
/// Durée CSS (`200ms`, `.3s`, négative comprise) en millisecondes arrondies.
pub(super) fn time(value: &str) -> Result<i64> {
    let n = if let Some(n) = value.strip_suffix("ms") {
        values::number(n)?
    } else if let Some(n) = value.strip_suffix('s') {
        values::number(n)? * 1000.
    } else {
        return Err("Animation time needs ms or s".into());
    };
    if !n.is_finite() || n.abs() > u32::MAX as f32 {
        return Err("Invalid animation time".into());
    }
    Ok(n.round() as i64)
}
