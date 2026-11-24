//! Une disposition ancrée en texte JSON, pour la garder d'une séance à l'autre :
//! `{"split":"h","fraction":0.25,"first":{…},"second":{…}}` pour une séparation,
//! `{"tabs":[1,2],"active":2}` pour un groupe, `null` pour une disposition vide. Les
//! clés des panneaux sont des entiers JSON (exacts jusqu'à 2⁵³).
use super::tree::{DockNode, DockTree};
use crate::{
    json::{Value, parse},
    ui::element::Axis,
};

impl DockTree {
    /// The layout as JSON text, read back by [`from_json`](Self::from_json).
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        match &self.root {
            Some(root) => write_node(root, &mut out),
            None => out.push_str("null"),
        }
        out
    }

    /// Read a layout written by [`to_json`](Self::to_json).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) when the text is not JSON, a
    /// node is neither a split nor a group, a group is empty, or a panel appears twice.
    pub fn from_json(text: &str) -> crate::Result<Self> {
        let invalid = |what: &str| crate::Error::invalid_input(format!("dock layout: {what}"));
        let value = parse(text).ok_or_else(|| invalid("not JSON"))?;
        let tree = match value {
            Value::Null => Self::default(),
            value => Self::from_node(read_node(&value).ok_or_else(|| invalid("bad node"))?),
        };
        let mut panels = tree.panels();
        let count = panels.len();
        panels.sort_unstable();
        panels.dedup();
        if panels.len() != count {
            return Err(invalid("a panel appears twice"));
        }
        Ok(tree)
    }
}

fn write_node(node: &DockNode, out: &mut String) {
    match node {
        DockNode::Tabs { panels, active } => {
            let list: Vec<String> = panels.iter().map(u64::to_string).collect();
            out.push_str(&format!(
                "{{\"tabs\":[{}],\"active\":{active}}}",
                list.join(",")
            ));
        }
        DockNode::Split {
            axis,
            fraction,
            first,
            second,
        } => {
            let axis = if *axis == Axis::Horizontal { "h" } else { "v" };
            out.push_str(&format!(
                "{{\"split\":\"{axis}\",\"fraction\":{fraction},\"first\":"
            ));
            write_node(first, out);
            out.push_str(",\"second\":");
            write_node(second, out);
            out.push('}');
        }
    }
}

fn member<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(members) => members.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// Un entier positif exact.
fn key_of(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) if *n >= 0. && n.fract() == 0. && *n < 9_007_199_254_740_992. => {
            Some(*n as u64)
        }
        _ => None,
    }
}

fn read_node(value: &Value) -> Option<DockNode> {
    if let Some(Value::Array(tabs)) = member(value, "tabs") {
        let panels = tabs.iter().map(key_of).collect::<Option<Vec<u64>>>()?;
        let active = member(value, "active").and_then(key_of)?;
        return (!panels.is_empty()).then(|| DockNode::tabs(panels, active));
    }
    let axis = match member(value, "split")? {
        Value::String(axis) if axis == "h" => Axis::Horizontal,
        Value::String(axis) if axis == "v" => Axis::Vertical,
        _ => return None,
    };
    let fraction = match member(value, "fraction")? {
        Value::Number(n) if n.is_finite() => *n as f32,
        _ => return None,
    };
    let first = read_node(member(value, "first")?)?;
    let second = read_node(member(value, "second")?)?;
    Some(DockNode::split(axis, fraction, first, second))
}
