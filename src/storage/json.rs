//! Le fichier d'un `LocalStorage` : un objet JSON de chaînes (`{"clé": "valeur"}`), lu par
//! [`crate::json`] et écrit à la main, au format qu'écrivent les navigateurs et
//! `serde_json` (indentation de deux espaces), pour qu'un fichier existant se relise tel
//! quel.
use crate::json::Value;
use std::collections::BTreeMap;

/// Objet `{"clé": "valeur", …}` ; `None` si le texte n'est pas un tel objet.
pub(super) fn parse(text: &str) -> Option<BTreeMap<String, String>> {
    let Value::Object(members) = crate::json::parse(text)? else {
        return None;
    };
    members
        .into_iter()
        .map(|(key, value)| match value {
            Value::String(value) => Some((key, value)),
            _ => None,
        })
        .collect()
}

/// Objet indenté de deux espaces, clés triées.
pub(super) fn write(map: &BTreeMap<String, String>) -> String {
    if map.is_empty() {
        return "{}".into();
    }
    let mut out = String::from("{\n");
    for (i, (key, value)) in map.iter().enumerate() {
        out += "  ";
        quote(&mut out, key);
        out += ": ";
        quote(&mut out, value);
        out += if i + 1 < map.len() { ",\n" } else { "\n" };
    }
    out.push('}');
    out
}

fn quote(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
