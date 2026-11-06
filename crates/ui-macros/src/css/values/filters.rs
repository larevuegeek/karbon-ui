//! `filter` et `backdrop-filter` : liste de fonctions de filtre du moteur.
use super::*;
pub(super) fn parse(value: &str) -> Result<String> {
    let mut items = Vec::new();
    let mut rest = if value == "none" { "" } else { value.trim() };
    while !rest.is_empty() {
        let (name, tail) = rest.split_once('(').ok_or("Expected a filter function")?;
        let mut depth = 1;
        let mut end = 0;
        for c in tail.chars() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += c.len_utf8();
        }
        if depth != 0 {
            return Err("Unclosed filter function".into());
        }
        let args = tail[..end].trim();
        rest = tail[end + 1..].trim();
        let root = format!("{ROOT}::rendering::filter::Filter");
        let code = match name.trim() {
            "blur" => {
                let n = if args.is_empty() {
                    0.
                } else {
                    px(args, false)?
                };
                if n > 250. {
                    return Err("Native blur is limited to 250 logical pixels".into());
                }
                format!("{root}::Blur({n:?})")
            }
            "hue-rotate" => format!(
                "{root}::HueRotate({:?})",
                if args.is_empty() { 0. } else { angle(args)? }
            ),
            "drop-shadow" => {
                let mut lengths = Vec::new();
                let mut tint = None;
                for word in words(args)? {
                    if let Ok(c) = color(&word) {
                        if tint.replace(c).is_some() {
                            return Err("Multiple drop-shadow colors".into());
                        }
                    } else {
                        lengths.push(px(&word, true)?);
                    }
                }
                if !(2..=3).contains(&lengths.len()) {
                    return Err(
                        "drop-shadow needs X Y [blur] and a color; spread/inset are not valid"
                            .into(),
                    );
                }
                let color = tint.ok_or("Use an explicit drop-shadow color")?;
                let x = lengths[0];
                let y = lengths[1];
                let blur = *lengths.get(2).unwrap_or(&0.);
                if !(0. ..=250.).contains(&blur) {
                    return Err("drop-shadow blur outside 0..250px".into());
                }
                format!("{root}::DropShadow{{x:{x:?},y:{y:?},blur:{blur:?},color:{color}}}")
            }
            name => {
                let kind = match name {
                    "brightness" => "Brightness",
                    "contrast" => "Contrast",
                    "grayscale" => "Grayscale",
                    "sepia" => "Sepia",
                    "saturate" => "Saturate",
                    "invert" => "Invert",
                    "opacity" => "Opacity",
                    _ => return Err(format!("Filter '{name}' is not supported")),
                };
                let mut n = if args.is_empty() {
                    1.
                } else if let Some(n) = args.strip_suffix('%') {
                    number(n)? / 100.
                } else {
                    number(args)?
                };
                if n < 0. {
                    return Err("Filter factors must be nonnegative".into());
                }
                if ["grayscale", "sepia", "invert", "opacity"].contains(&name) {
                    n = n.min(1.);
                } else if n > 10000. {
                    return Err("Filter factor exceeds the native limit 10000".into());
                }
                format!("{root}::{kind}({n:?})")
            }
        };
        items.push(code);
        if items.len() > 16 {
            return Err("At most 16 filter functions per element".into());
        }
    }
    Ok(format!(
        "{ROOT}::rendering::filter::Filters::new(const{{&[{}]}})",
        items.join(",")
    ))
}
