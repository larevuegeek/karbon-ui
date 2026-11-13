//! Listes déroulantes : `<select>` et ses `<option>` / `<optgroup>`.
use super::{Generator, R, tree::is_whitespace};
use crate::{
    Error, Result,
    template::{
        html::Kind,
        markup::{Element, Node, Value},
    },
};
use proc_macro::Span;

impl Generator {
    pub(super) fn select(&mut self, e: &Element) -> Result<String> {
        self.check(e, Kind::Select, &["value", "disabled", "placeholder"])?;
        let site = self.next();
        let id = self.id(e, site)?;
        let disabled = self.disabled(e)?;
        let change = match e.attr("onchange") {
            Some(a) => self.message(a)?,
            None if e.attr("disabled").is_some() => "|_:u64|()".into(),
            None => {
                return Err(e.loc.error(
                    "<select> needs onchange={Msg::Variant} (a Variant(u64) message with the option value)",
                ));
            }
        };
        let options = self.options(&e.children)?;
        let selected = match e.attr("value") {
            Some(a) => match &a.value {
                Value::Expr(t) => format!("{R}::ui::template::selected({})", self.out.value(t)),
                Value::Text(t) => format!(
                    "Some({}u64)",
                    t.parse::<u64>()
                        .map_err(|_| a.loc.error("select value must be a u64 option value"))?
                ),
                Value::Bool => return Err(a.loc.error("select value needs a value")),
            },
            None => "__ui_selected".into(),
        };
        let mut widget = format!("{R}::ui::widgets::Select::new({selected},__ui_options,{change})");
        if let Some(a) = e.attr("placeholder") {
            widget += &format!(".placeholder({})", self.text_attr(a)?);
        }
        let accessible = match e.attr("aria-label") {
            Some(a) => Some(self.text_attr(a)?),
            None => match e.attr("id").map(|a| &a.value) {
                Some(Value::Text(id)) => self.labels.get(id).map(|t| format!("{t:?}")),
                _ => None,
            },
        };
        if let Some(label) = accessible {
            widget += &format!(".label({label})");
        }
        let before = format!(
            "#[allow(unused_mut)] let mut __ui_options=::std::vec::Vec::<{R}::ui::widgets::SelectOption>::new();#[allow(unused_mut,unused_assignments)] let mut __ui_selected: ::core::option::Option<u64>=None;{options}"
        );
        self.finish(
            e,
            Kind::Select,
            &id,
            &before,
            &format!(
                "{R}::ui::template::Node::widget(__ui_id,{{let mut __ui_widget={widget};__ui_widget.enabled=!{disabled};__ui_widget}})"
            ),
        )
    }
    fn options(&mut self, nodes: &[Node]) -> Result<String> {
        let mut code = String::new();
        for node in nodes {
            match node {
                n if is_whitespace(n) => {}
                Node::Element(o) if o.name == "option" => {
                    for a in &o.attrs {
                        if !["value", "disabled", "selected"].contains(&a.name.as_str()) {
                            return Err(a
                                .loc
                                .error(format!("Unknown attribute '{}' on <option>", a.name)));
                        }
                    }
                    let value = match o.attr("value") {
                        Some(a) => match &a.value {
                            Value::Text(t) => format!(
                                "{}u64",
                                t.parse::<u64>().map_err(|_| a.loc.error(
                                    "option value must be a u64 identifier, or value={expr}"
                                ))?
                            ),
                            Value::Expr(t) => self.out.value(t),
                            Value::Bool => return Err(a.loc.error("option value needs a value")),
                        },
                        None => {
                            return Err(o
                                .loc
                                .error("<option> needs value=\"…\" (a u64 identifier)"));
                        }
                    };
                    let label =
                        self.plain_text(&o.children.iter().collect::<Vec<_>>(), "an <option>")?;
                    // Valeur liée une fois : l'option et `selected` lisent la même.
                    let mut option =
                        format!("{R}::ui::widgets::SelectOption::new(__ui_option_value,{label})");
                    if let Some(a) = o.attr("disabled") {
                        let d = self.boolean(a)?;
                        option = format!(
                            "{{let __ui_o={option};if {d}{{__ui_o.disabled()}}else{{__ui_o}}}}"
                        );
                    }
                    let selected = match o.attr("selected") {
                        Some(a) => {
                            let s = self.boolean(a)?;
                            format!("if {s}{{__ui_selected=Some(__ui_option_value);}}")
                        }
                        None => String::new(),
                    };
                    code += &format!(
                        "{{let __ui_option_value:u64={value};__ui_options.push({option});{selected}}}"
                    );
                }
                Node::If {
                    branches,
                    otherwise,
                    ..
                } => {
                    for (i, (cond, body)) in branches.iter().enumerate() {
                        let cond = self.out.value(cond);
                        if i > 0 {
                            code += "else ";
                        }
                        code += &format!("if {cond}{{{}}}", self.options(body)?);
                    }
                    if let Some(body) = otherwise {
                        code += &format!("else{{{}}}", self.options(body)?);
                    }
                }
                Node::Each(each) => {
                    let header = self.each_header(each);
                    code += &format!("{header}{{{}}}", self.options(&each.body)?);
                }
                Node::Fragment { nodes, .. } => code += &self.options(nodes)?,
                other => {
                    return Err(other.loc().map_or_else(
                        || Error::new("<select> contains <option> elements", Span::call_site()),
                        |l| l.error("<select> can only contain <option>, {#if} and {#each}"),
                    ));
                }
            }
        }
        Ok(code)
    }
}
