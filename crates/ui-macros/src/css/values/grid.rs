//! Pistes de grille (`grid-template-columns` / `-rows`) : tailles, `repeat()`,
//! `minmax()`, `fit-content()`.
use super::*;
pub(super) fn tracks(value: &str) -> Result<String> {
    fn breadth(value: &str, fraction: bool) -> Result<String> {
        let value = value.trim();
        let prefix = format!("{ROOT}::layout::TrackBreadth");
        match value {
            "auto" => return Ok(format!("{prefix}::Auto")),
            "max-content" => return Ok(format!("{prefix}::MaxContent")),
            "min-content" => return Ok(format!("{prefix}::MinContent")),
            _ => {}
        }
        if let Some(limit) = value
            .strip_prefix("fit-content(")
            .and_then(|v| v.strip_suffix(')'))
        {
            return Ok(format!(
                "{prefix}::FitContent(cx.px({:?}))",
                px(limit, false)?
            ));
        }
        if let Some(v) = value.strip_suffix("fr") {
            let n = number(v)?;
            if !fraction || n <= 0. {
                return Err("Invalid fractional grid track".into());
            }
            return Ok(format!("{prefix}::Fr({n:?})"));
        }
        if let Some(v) = value.strip_suffix('%') {
            let n = number(v)?;
            if n < 0. {
                return Err("Negative grid track".into());
            }
            return Ok(format!("{prefix}::Percent({n:?})"));
        }
        Ok(format!("{prefix}::Px(cx.px({:?}))", px(value, false)?))
    }
    fn expand(value: &str, depth: usize) -> Result<Vec<String>> {
        if depth > 8 {
            return Err("Grid repeat nesting exceeds 8".into());
        }
        let mut out = Vec::new();
        for v in words(value)? {
            if let Some(inner) = v.strip_prefix("repeat(").and_then(|s| s.strip_suffix(')')) {
                let args = split(inner, ',')?;
                if args.len() != 2 {
                    return Err("repeat() requires a count and tracks".into());
                }
                let n = args[0]
                    .parse::<usize>()
                    .map_err(|_| "Grid repeat count must be an integer")?;
                let tracks = expand(args[1], depth + 1)?;
                if n == 0 || n.saturating_mul(tracks.len()) > 128 {
                    return Err("Grid needs 1..128 tracks".into());
                }
                for _ in 0..n {
                    out.extend(tracks.clone());
                }
            } else if let Some(inner) = v.strip_prefix("minmax(").and_then(|s| s.strip_suffix(')'))
            {
                let args = split(inner, ',')?;
                if args.len() != 2 {
                    return Err("minmax() needs two breadths".into());
                }
                out.push(format!(
                    "{ROOT}::layout::GridTrack::minmax({},{})",
                    breadth(args[0], false)?,
                    breadth(args[1], true)?
                ));
            } else {
                out.push(format!(
                    "{ROOT}::layout::GridTrack::new({})",
                    breadth(&v, true)?
                ));
            }
            if out.len() > 128 {
                return Err("Grid exceeds 128 tracks".into());
            }
        }
        if out.is_empty() {
            return Err("Grid needs at least one track".into());
        }
        Ok(out)
    }
    Ok(format!(
        "::std::sync::Arc::from([{}])",
        expand(value, 0)?.join(",")
    ))
}
