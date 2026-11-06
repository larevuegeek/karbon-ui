//! Propriétés CSS : polices, texte et décorations.
use super::super::*;
use super::{Next, one, scalar};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    match name {
        "font-size" => {
            if px(value, false).is_err() {
                return one("font_size_expression", expression::parse(value)?);
            }
            let n = px(value, false)?;
            if n <= 0. {
                return Err("The native text renderer requires a positive font-size".into());
            }
            one("font_size", scalar(n))
        }
        "text-align" => one(
            "text_align",
            format!(
                "{ROOT}::styling::TextAlign::{}",
                match value {
                    "left" | "start" => "Start",
                    "right" | "end" => "End",
                    "center" => "Center",
                    "justify" => "Justify",
                    _ => return Err("Unsupported text-align".into()),
                }
            ),
        ),
        "line-height" => one(
            "line_height",
            if value == "normal" {
                format!("{ROOT}::styling::LineHeight::Normal")
            } else if value.ends_with("px") {
                format!("{ROOT}::styling::LineHeight::Px({:?})", px(value, false)?)
            } else {
                let n = number(value)?;
                if n <= 0. {
                    return Err("line-height must be positive".into());
                }
                format!("{ROOT}::styling::LineHeight::Factor({n:?})")
            },
        ),
        "text-decoration" | "text-decoration-line" => {
            let words = words(value)?;
            if words
                .iter()
                .any(|v| !["none", "underline", "line-through", "overline"].contains(&v.as_str()))
            {
                return Err("Use text-decoration: none/underline/line-through/overline".into());
            }
            one(
                "text_decoration",
                format!(
                    "{ROOT}::styling::TextDecoration{{underline:{},line_through:{},overline:{}}}",
                    words.iter().any(|v| v == "underline"),
                    words.iter().any(|v| v == "line-through"),
                    words.iter().any(|v| v == "overline")
                ),
            )
        }
        // Couleur, décalage et épaisseur des lignes de décoration (dessinées par le
        // moteur) ; `auto`/`from-font` gardent celles de la police.
        "text-decoration-color" => one("decoration_color", color(value)?),
        "text-underline-offset" | "text-decoration-thickness" => match value {
            "auto" | "from-font" => Ok(Vec::new()),
            v => {
                let n = px(v, true)?;
                if name == "text-decoration-thickness" && n < 0. {
                    return Err("text-decoration-thickness cannot be negative".into());
                }
                one(
                    if name == "text-underline-offset" {
                        "underline_offset"
                    } else {
                        "decoration_thickness"
                    },
                    scalar(n),
                )
            }
        },
        "text-decoration-style" => match value {
            "solid" => Ok(Vec::new()),
            _ => Err("Only solid decoration lines are implemented".into()),
        },
        "font-weight" => {
            let n = match value {
                "normal" => 400.,
                "bold" => 700.,
                v => number(v)?,
            };
            if !(1. ..=999.).contains(&n) || n.fract() != 0. {
                return Err("font-weight must be an integer from 1 to 999".into());
            }
            one("weight", format!("{}", n as u16))
        }
        "font-family" => {
            // Liste de repli : familles jointes par des virgules, la première disponible
            // est choisie par le backend.
            let mut families: Vec<String> = Vec::new();
            for item in split(value, ',')? {
                let item = item.trim();
                let family = if item.starts_with(['"', '\'']) {
                    quoted(item)?
                } else {
                    let words = words(item)?;
                    if words.is_empty()
                        || words.iter().any(|w| {
                            !w.chars()
                                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
                        })
                    {
                        return Err(format!("Invalid font family '{item}'"));
                    }
                    match item {
                        "system-ui" | "sans-serif" | "ui-sans-serif" | "-apple-system"
                        | "BlinkMacSystemFont" => "Segoe UI".into(),
                        "serif" | "ui-serif" => "Times New Roman".into(),
                        "monospace" | "ui-monospace" => "Consolas".into(),
                        "cursive" => "Comic Sans MS".into(),
                        "inherit" | "initial" | "unset" => {
                            return Err("CSS-wide keywords are not implemented".into());
                        }
                        _ => words.join(" "),
                    }
                };
                if family.is_empty() || family.contains([',', '\0']) {
                    return Err(format!("Invalid font family '{item}'"));
                }
                if !families.contains(&family) {
                    families.push(family);
                }
            }
            if families.is_empty() || families.len() > 16 {
                return Err("font-family needs 1..16 families".into());
            }
            one("font_family", format!("{:?}", families.join(",")))
        }
        "font-style" => one(
            "font_italic",
            match value {
                "normal" => "false",
                "italic" | "oblique" => "true",
                _ => return Err("Use font-style: normal, italic or oblique".into()),
            }
            .into(),
        ),
        "font-variant-numeric" => one(
            "tabular_numbers",
            match value {
                "normal" | "proportional-nums" => "false",
                "tabular-nums" => "true",
                _ => {
                    return Err(
                        "font-variant-numeric supports normal, tabular-nums and proportional-nums"
                            .into(),
                    );
                }
            }
            .into(),
        ),
        "letter-spacing" => one(
            "letter_spacing",
            if value == "normal" {
                format!("{ROOT}::styling::LetterSpacing::Px(0.0)")
            } else if let Some(n) = value.strip_suffix("em").filter(|v| !v.ends_with('r')) {
                format!("{ROOT}::styling::LetterSpacing::Em({:?})", number(n)?)
            } else {
                format!("{ROOT}::styling::LetterSpacing::Px({:?})", px(value, true)?)
            },
        ),
        "text-transform" => one(
            "text_transform",
            format!(
                "{ROOT}::styling::TextTransform::{}",
                match value {
                    "none" => "None",
                    "uppercase" => "Uppercase",
                    "lowercase" => "Lowercase",
                    "capitalize" => "Capitalize",
                    _ =>
                        return Err(
                            "Use text-transform: none, uppercase, lowercase or capitalize".into()
                        ),
                }
            ),
        ),
        "white-space" | "text-wrap" | "text-wrap-mode" => one(
            "white_space",
            format!(
                "{ROOT}::styling::WhiteSpace::{}",
                match value {
                    "normal" | "wrap" => "Normal",
                    "nowrap" => "NoWrap",
                    "pre" => "Pre",
                    "pre-wrap" | "break-spaces" => "PreWrap",
                    _ => return Err("Use white-space: normal, nowrap, pre or pre-wrap".into()),
                }
            ),
        ),
        "direction" => one(
            "direction_rtl",
            match value {
                "ltr" => "false",
                "rtl" => "true",
                _ => return Err("Use direction: ltr or rtl".into()),
            }
            .into(),
        ),
        "text-overflow" => one(
            "text_overflow",
            format!(
                "{ROOT}::styling::TextOverflow::{}",
                match value {
                    "clip" => "Clip",
                    "ellipsis" => "Ellipsis",
                    _ => return Err("Use text-overflow: clip or ellipsis".into()),
                }
            ),
        ),
        "line-clamp" | "-webkit-line-clamp" => one(
            "line_clamp",
            if value == "none" {
                "0u16".into()
            } else {
                let n = value
                    .parse::<u16>()
                    .map_err(|_| "line-clamp requires none or a positive integer")?;
                if n == 0 || n > 1000 {
                    return Err("line-clamp requires an integer from 1 to 1000".into());
                }
                format!("{n}u16")
            },
        ),
        "-webkit-box-orient" => {
            if value == "vertical" {
                Ok(Vec::new())
            } else {
                Err("Only -webkit-box-orient: vertical (with line-clamp) is accepted".into())
            }
        }
        _ => next(),
    }
}
