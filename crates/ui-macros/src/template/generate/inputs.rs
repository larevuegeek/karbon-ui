//! Champs de saisie : `<input>` de tous types, libellés `<label>` (nom accessible,
//! bascule des cases) et groupes de boutons radio.
use super::{Generator, R, text::static_text};
use crate::{
    Result,
    template::{
        html::Kind,
        markup::{Attr, Element, Node, Value},
    },
};
use std::collections::HashMap;

pub(super) fn wraps_input(e: &Element) -> bool {
    e.children
        .iter()
        .any(|c| matches!(c, Node::Element(i) if i.name == "input"))
}
pub(super) fn input_type(e: &Element) -> Result<String> {
    match e.attr("type").map(|a| &a.value) {
        None => Ok("text".into()),
        Some(Value::Text(t)) => Ok(t.to_ascii_lowercase()),
        Some(_) => Err(e
            .loc
            .error("input type must be a literal, e.g. type=\"checkbox\"")),
    }
}
pub(super) fn collect_labels(nodes: &[Node], labels: &mut HashMap<String, String>) {
    for node in nodes {
        match node {
            Node::Element(e) => {
                if e.name == "label"
                    && let Some(Attr {
                        value: Value::Text(id),
                        ..
                    }) = e.attr("for")
                    && let Some(text) = static_text(&e.children)
                {
                    labels.insert(id.clone(), text);
                }
                collect_labels(&e.children, labels);
            }
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                for (_, b) in branches {
                    collect_labels(b, labels);
                }
                if let Some(b) = otherwise {
                    collect_labels(b, labels);
                }
            }
            Node::Each(e) => collect_labels(&e.body, labels),
            Node::Fragment { nodes, .. } => collect_labels(nodes, labels),
            _ => {}
        }
    }
}
/// Libellés `<label for>` voisins d'une case active du même parent : seuls les frères
/// partagent sûrement la portée Rust de `checked` et `onchange` (même boucle, même
/// branche).
pub(super) fn collect_label_toggles(
    nodes: &[Node],
    out: &mut HashMap<String, (Option<Attr>, Attr)>,
) {
    let mut boxes = HashMap::new();
    for node in nodes {
        if let Node::Element(e) = node
            && e.name == "input"
            && input_type(e).is_ok_and(|t| t == "checkbox")
            && e.attr("disabled").is_none()
            && let (
                Some(Attr {
                    value: Value::Text(id),
                    ..
                }),
                Some(change),
            ) = (e.attr("id"), e.attr("onchange"))
        {
            boxes.insert(id.clone(), (e.attr("checked").cloned(), change.clone()));
        }
    }
    for node in nodes {
        match node {
            Node::Element(e) => {
                if e.name == "label"
                    && e.attr("onclick").is_none()
                    && let Some(Attr {
                        value: Value::Text(id),
                        ..
                    }) = e.attr("for")
                    && let Some(target) = boxes.get(id)
                {
                    out.insert(id.clone(), target.clone());
                }
                collect_label_toggles(&e.children, out);
            }
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                for (_, b) in branches {
                    collect_label_toggles(b, out);
                }
                if let Some(b) = otherwise {
                    collect_label_toggles(b, out);
                }
            }
            Node::Each(e) => collect_label_toggles(&e.body, out),
            Node::Fragment { nodes, .. } => collect_label_toggles(nodes, out),
            _ => {}
        }
    }
}
/// Noms des groupes de boutons radio directement contenus (blocs compris).
pub(super) fn radio_names(nodes: &[Node]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    fn walk(nodes: &[Node], names: &mut Vec<String>) {
        for node in nodes {
            match node {
                Node::Element(e) if e.name == "input" => {
                    if matches!(e.attr("type").map(|a| &a.value), Some(Value::Text(t)) if t == "radio")
                        && let Some(Attr {
                            value: Value::Text(name),
                            ..
                        }) = e.attr("name")
                        && !names.contains(name)
                    {
                        names.push(name.clone());
                    }
                }
                Node::Element(e) if e.name == "label" => walk(&e.children, names),
                Node::If {
                    branches,
                    otherwise,
                    ..
                } => {
                    for (_, b) in branches {
                        walk(b, names);
                    }
                    if let Some(b) = otherwise {
                        walk(b, names);
                    }
                }
                Node::Each(e) => walk(&e.body, names),
                Node::Fragment { nodes, .. } => walk(nodes, names),
                _ => {}
            }
        }
    }
    walk(nodes, &mut names);
    names
}
impl Generator {
    pub(super) fn label_input(&mut self, e: &Element) -> Result<String> {
        // <label><input type="checkbox"> Texte</label> : libellé de la case.
        let mut input = None;
        let mut rest = Vec::new();
        for child in &e.children {
            match child {
                Node::Element(i) if i.name == "input" => {
                    if input.replace(i).is_some() {
                        return Err(i.loc.error("A <label> can wrap a single <input>"));
                    }
                }
                other => rest.push(other),
            }
        }
        let input = input.unwrap();
        if !e.attrs.is_empty() {
            return Err(e.loc.error(
                "A <label> wrapping an <input> takes no attributes; put them on the <input>",
            ));
        }
        let label = self.plain_text(&rest, "a label")?;
        self.input(input, Some(label))
    }
    pub(super) fn input(&mut self, e: &Element, label: Option<String>) -> Result<String> {
        let ty = input_type(e)?;
        let site = self.next();
        let id = self.id(e, site)?;
        if !e.children.is_empty() {
            return Err(e.loc.error("<input> cannot have children"));
        }
        let disabled = self.disabled(e)?;
        let aria = match e.attr("aria-label") {
            Some(a) => Some(self.text_attr(a)?),
            None => None,
        };
        let for_label = match e.attr("id").map(|a| &a.value) {
            Some(Value::Text(id)) => self.labels.get(id).map(|t| format!("{t:?}")),
            _ => None,
        };
        let accessible = aria.or(for_label);
        let constructor = match ty.as_str() {
            "text" | "search" | "password" => {
                self.check(
                    e,
                    Kind::Input,
                    &[
                        "type",
                        "value",
                        "placeholder",
                        "disabled",
                        "readonly",
                        "maxlength",
                        "for",
                    ],
                )?;
                let value = match e.attr("value") {
                    Some(a) => self.text_attr(a)?,
                    None => "\"\"".into(),
                };
                let readonly = match e.attr("readonly") {
                    Some(a) => self.boolean(a)?,
                    None => "false".into(),
                };
                let change = match e.attr("oninput") {
                    Some(a) => self.message(a)?,
                    None if e.attr("readonly").is_some() || e.attr("disabled").is_some() => {
                        "|_: ::std::string::String|()".into()
                    }
                    None => {
                        return Err(e.loc.error(
                            "<input type=\"text\"> needs oninput={Msg::Variant} (a Variant(String) message), or readonly",
                        ));
                    }
                };
                let mut widget = format!("{R}::ui::widgets::TextInput::new({value},{change})");
                if let Some(a) = e.attr("placeholder") {
                    widget += &format!(".placeholder({})", self.text_attr(a)?);
                }
                if let Some(label) = accessible.clone() {
                    widget += &format!(".label({label})");
                }
                if ty == "password" {
                    widget += ".password()";
                }
                if let Some(a) = e.attr("maxlength") {
                    widget += &format!(".max_chars(({}) as usize)", self.number(a)?);
                }
                let submit = match e.attr("onchange") {
                    Some(a) => Some(self.message(a)?),
                    None => self.forms.iter().rev().flatten().next().cloned(),
                };
                if let Some(m) = submit {
                    widget += &format!(".on_submit({m})");
                }
                if let Some(a) = e.attr("onkeydown") {
                    widget += &format!(".on_key({})", self.message(a)?);
                }
                format!(
                    "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={widget};__ui_widget.enabled=!{disabled};__ui_widget.read_only={readonly};__ui_widget}})"
                )
            }
            "checkbox" => {
                self.check(
                    e,
                    Kind::Input,
                    &[
                        "type",
                        "checked",
                        "switch",
                        "indeterminate",
                        "disabled",
                        "value",
                    ],
                )?;
                let checked = match e.attr("checked") {
                    Some(a) => self.boolean(a)?,
                    None => "false".into(),
                };
                let label = label.or(accessible).unwrap_or_else(|| "\"\"".into());
                let change = match e.attr("onchange") {
                    Some(a) => self.message(a)?,
                    None if e.attr("disabled").is_some() => "|_:bool|()".into(),
                    None => {
                        return Err(e.loc.error(
                            "<input type=\"checkbox\"> needs onchange={Msg::Variant} (a Variant(bool) message with the new value)",
                        ));
                    }
                };
                let widget = if e.attr("switch").is_some() {
                    "Switch"
                } else {
                    "Checkbox"
                };
                let mixed = match e.attr("indeterminate") {
                    Some(a) if widget == "Switch" => {
                        return Err(a.loc.error("A switch cannot be indeterminate"));
                    }
                    Some(a) => {
                        let cond = self.boolean(a)?;
                        format!(
                            "let __ui_widget=if {cond}{{__ui_widget.state({R}::ui::element::CheckState::Mixed)}}else{{__ui_widget}};"
                        )
                    }
                    None => String::new(),
                };
                format!(
                    "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={R}::ui::widgets::{widget}::new({label},{checked},{change});__ui_widget.enabled=!{disabled};{mixed}__ui_widget}})"
                )
            }
            "radio" => {
                self.check(
                    e,
                    Kind::Input,
                    &["type", "checked", "name", "disabled", "value"],
                )?;
                if e.attr("name").is_none() {
                    return Err(e.loc.error("<input type=\"radio\"> needs name=\"group\""));
                }
                let checked = match e.attr("checked") {
                    Some(a) => self.boolean(a)?,
                    None => "false".into(),
                };
                let label = label.or(accessible).unwrap_or_else(|| "\"\"".into());
                let message = match e.attr("onchange") {
                    Some(a) => self.message(a)?,
                    None if e.attr("disabled").is_some() => "()".into(),
                    None => {
                        return Err(e.loc.error(
                            "<input type=\"radio\"> needs onchange={Msg::Choice} (message sent when chosen)",
                        ));
                    }
                };
                format!(
                    "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={R}::ui::widgets::RadioButton::new({label},{checked},{message});__ui_widget.enabled=!{disabled};__ui_widget}})"
                )
            }
            "range" => {
                self.check(
                    e,
                    Kind::Input,
                    &[
                        "type", "value", "min", "max", "step", "orient", "disabled", "readonly",
                    ],
                )?;
                let value = match e.attr("value") {
                    Some(a) => self.number(a)?,
                    None => "0f64".into(),
                };
                let (input, change) = (e.attr("oninput"), e.attr("onchange"));
                let handler = match input.or(change) {
                    Some(a) => self.message(a)?,
                    None if e.attr("readonly").is_some() || e.attr("disabled").is_some() => {
                        "|_:f64|()".into()
                    }
                    None => {
                        return Err(e.loc.error(
                            "<input type=\"range\"> needs oninput={Msg::Variant} (a Variant(f64) message) or onchange",
                        ));
                    }
                };
                let mut widget = format!("{R}::ui::widgets::Slider::new({value},{handler})");
                if input.is_some()
                    && let Some(a) = change
                {
                    widget += &format!(".on_commit({})", self.message(a)?);
                }
                let min = match e.attr("min") {
                    Some(a) => self.number(a)?,
                    None => "0f64".into(),
                };
                let max = match e.attr("max") {
                    Some(a) => self.number(a)?,
                    None => "100f64".into(),
                };
                widget += &format!(".range({min},{max})");
                if let Some(a) = e.attr("step") {
                    widget += &format!(".step({})", self.number(a)?);
                }
                if let Some(a) = e.attr("orient") {
                    match &a.value {
                        Value::Text(t) if t == "vertical" => {
                            widget += &format!(".axis({R}::ui::element::Axis::Vertical)")
                        }
                        Value::Text(t) if t == "horizontal" => {}
                        _ => {
                            return Err(a
                                .loc
                                .error("orient must be \"horizontal\" or \"vertical\""));
                        }
                    }
                }
                if let Some(label) = accessible.clone() {
                    widget += &format!(".label({label})");
                }
                let readonly = match e.attr("readonly") {
                    Some(a) => self.boolean(a)?,
                    None => "false".into(),
                };
                format!(
                    "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={widget};__ui_widget.enabled=!{disabled};__ui_widget.read_only={readonly};__ui_widget}})"
                )
            }
            other => {
                return Err(e.loc.error(format!(
                    "<input type=\"{other}\"> is not supported; supported types: text, search, checkbox (with switch), radio, range"
                )));
            }
        };
        self.finish(e, Kind::Input, &id, "", &constructor)
    }
}
