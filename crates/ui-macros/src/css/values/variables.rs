//! Propriétés personnalisées : résolution de `var()`, valeurs typées des variables
//! d'élément et `content` des pseudo-éléments.
use super::*;
/// Valeur typée (`VariableValue`) d'une propriété personnalisée déclarée hors de
/// `:root`, en code constant : nombre, longueur, couleur ou famille entre guillemets.
pub fn custom_value(value: &str) -> Result<String> {
    const VALUE: &str = "__ui_root::ui::styling::VariableValue";
    let value = value.trim();
    if value.contains("var(") {
        return Err(
            "Custom properties referencing var() are only supported in :root; give a literal value"
                .into(),
        );
    }
    if let Ok(n) = number(value) {
        return Ok(format!("{VALUE}::Number({n:?})"));
    }
    if value.starts_with(['"', '\'']) {
        let family = quoted(value)?;
        if family.is_empty() || family.contains('\0') {
            return Err("Invalid font family".into());
        }
        return Ok(format!("{VALUE}::Family({family:?})"));
    }
    if let Some(n) = value.strip_suffix('%').and_then(|n| number(n).ok()) {
        return Ok(format!(
            "{VALUE}::Length(__ui_root::ui::element::Length::Percent({n:?}))"
        ));
    }
    if let Ok(n) = px(value, true) {
        return Ok(format!(
            "{VALUE}::Length(__ui_root::ui::element::Length::Px({n:?}))"
        ));
    }
    if let Ok(code) = expression::parse(value) {
        return Ok(format!(
            "{VALUE}::Length(__ui_root::ui::element::Length::Computed({code}))"
        ));
    }
    if let Ok(color) = color(value) {
        return Ok(format!("{VALUE}::Color({color})"));
    }
    Err(format!(
        "Unsupported custom property value '{value}'; use a number, a length, a color or a quoted family"
    ))
}
/// Valeur de `content` d'un pseudo-élément : texte (éventuellement vide), ou `None`
/// pour `none` / `normal` (pas de boîte générée).
pub fn pseudo_content(value: &str) -> Result<Option<String>> {
    let value = value.trim();
    match value {
        "none" | "normal" => Ok(None),
        v if v.starts_with(['"', '\'']) => {
            let text = quoted(v)?;
            if text.len() > 1024 {
                return Err("content strings are limited to 1024 bytes".into());
            }
            Ok(Some(text))
        }
        _ => Err("content supports a quoted string, \"\", none or normal".into()),
    }
}
pub fn resolve(value: &str, vars: &BTreeMap<String, String>) -> Result<String> {
    fn inner(
        value: &str,
        vars: &BTreeMap<String, String>,
        stack: &mut Vec<String>,
    ) -> Result<String> {
        if stack.len() > 32 {
            return Err("CSS variable nesting exceeds 32".into());
        }
        let mut output = String::new();
        let mut i = 0;
        let mut quote = None;
        let bytes = value.as_bytes();
        while i < value.len() {
            if output.len() > 2 * 1024 * 1024 {
                return Err("Expanded CSS value exceeds 2 MiB".into());
            }
            if bytes[i] == b'\\' {
                let next = value[i + 1..]
                    .chars()
                    .next()
                    .ok_or("Incomplete CSS escape")?;
                let end = i + 1 + next.len_utf8();
                output.push_str(&value[i..end]);
                i = end;
                continue;
            }
            if let Some(q) = quote {
                let c = value[i..].chars().next().unwrap();
                output.push(c);
                i += c.len_utf8();
                if c as u32 == q {
                    quote = None;
                }
                continue;
            }
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                quote = Some(bytes[i] as u32);
                output.push(bytes[i] as char);
                i += 1;
                continue;
            }
            if value[i..].starts_with("var(") {
                let mut depth = 1;
                let start = i + 4;
                let mut end = start;
                let mut quoted = None;
                let mut escaped = false;
                while end < value.len() && depth > 0 {
                    let b = bytes[end];
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if let Some(q) = quoted {
                        if b == q {
                            quoted = None;
                        }
                    } else if b == b'"' || b == b'\'' {
                        quoted = Some(b);
                    } else {
                        match b {
                            b'(' => depth += 1,
                            b')' => depth -= 1,
                            _ => {}
                        }
                    }
                    if depth > 0 {
                        end += 1;
                    }
                }
                if depth != 0 {
                    return Err("Unclosed var()".into());
                }
                let args = split(&value[start..end], ',')?;
                let name = args[0].trim();
                if !name.starts_with("--") || args.len() > 2 {
                    return Err("Use var(--name[, fallback])".into());
                }
                if stack.iter().any(|s| s == name) {
                    return Err(format!("Circular CSS variable '{name}'"));
                }
                let selected = vars
                    .get(name)
                    .map(String::as_str)
                    .or_else(|| args.get(1).copied())
                    .ok_or_else(|| format!("Unknown CSS variable '{name}'"))?;
                stack.push(name.into());
                output += &inner(selected, vars, stack)?;
                stack.pop();
                i = end + 1;
            } else {
                let c = value[i..].chars().next().unwrap();
                output.push(c);
                i += c.len_utf8();
            }
        }
        if output.len() > 2 * 1024 * 1024 {
            return Err("Expanded CSS value exceeds 2 MiB".into());
        }
        Ok(output)
    }
    inner(value, vars, &mut Vec::new())
}
