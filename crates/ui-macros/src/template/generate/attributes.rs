//! Attributs d'état et d'accessibilité : `disabled`, région de déplacement de
//! fenêtre, `aria-hidden`, `role`.
use super::{Generator, R};
use crate::{
    Result,
    template::markup::{Element, Value},
};

impl Generator {
    pub(super) fn disabled(&mut self, e: &Element) -> Result<String> {
        match e.attr("disabled") {
            Some(a) => self.boolean(a),
            None => Ok("false".into()),
        }
    }
    /// `aria-hidden` : sous-arbre retiré de l'arbre d'accessibilité.
    /// `data-drag-region` : la zone déplace la fenêtre (barres de titre personnalisées).
    pub(super) fn drag_region(&mut self, e: &Element) -> Result<String> {
        Ok(match e.attr("data-drag-region").map(|a| &a.value) {
            None => String::new(),
            Some(Value::Bool) => "__ui_el=__ui_el.drag_region(true);".into(),
            Some(Value::Text(t)) if t == "true" || t.is_empty() => {
                "__ui_el=__ui_el.drag_region(true);".into()
            }
            Some(Value::Text(t)) if t == "false" => String::new(),
            Some(Value::Expr(t)) => {
                let value = self.out.value(t);
                format!("__ui_el=__ui_el.drag_region({value});")
            }
            Some(_) => {
                return Err(e.loc.error(
                    "data-drag-region expects no value, \"true\", \"false\" or a Rust bool",
                ));
            }
        })
    }
    pub(super) fn aria_hidden(&mut self, e: &Element) -> Result<String> {
        Ok(match e.attr("aria-hidden").map(|a| &a.value) {
            None => String::new(),
            Some(Value::Bool) => "__ui_el=__ui_el.accessibility_hidden(true);".into(),
            Some(Value::Text(t)) if t == "true" => {
                "__ui_el=__ui_el.accessibility_hidden(true);".into()
            }
            Some(Value::Text(t)) if t == "false" => String::new(),
            Some(Value::Expr(t)) => {
                let value = self.out.value(t);
                format!("__ui_el=__ui_el.accessibility_hidden({value});")
            }
            Some(_) => {
                return Err(e.loc.error(
                    "aria-hidden expects \"true\", \"false\" or a Rust bool: aria-hidden={cond}",
                ));
            }
        })
    }
    /// `role="…"`, `aria-expanded` et `aria-haspopup` : rôle d'accessibilité remplacé et
    /// état développé d'un déclencheur de menu.
    pub(super) fn aria_role(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        if let Some(a) = e.attr("role") {
            let role = match Self::literal(a)? {
                "menuitem" => "MenuItem",
                "menu" => "Menu",
                "listbox" | "list" => "List",
                "option" | "listitem" => "ListItem",
                "separator" => "Separator",
                "group" => "Group",
                "button" => "Button",
                "img" => "Image",
                "heading" => "Heading",
                "navigation" => "Navigation",
                "none" | "presentation" => "Custom",
                other => {
                    return Err(a.loc.error(format!(
                        "role=\"{other}\" is not supported; use menuitem, menu, option, listbox, separator, group, button, img, heading, navigation or none"
                    )));
                }
            };
            code += &format!(
                "__ui_el={R}::ui::template::role(__ui_el,{R}::ui::element::Role::{role});"
            );
        }
        if let Some(a) = e.attr("aria-expanded") {
            let value = match &a.value {
                Value::Text(t) if t == "true" => "true".to_owned(),
                Value::Text(t) if t == "false" => "false".to_owned(),
                Value::Expr(t) => format!("({})", self.out.value(t)),
                _ => {
                    return Err(a
                        .loc
                        .error("aria-expanded expects \"true\", \"false\" or {bool}"));
                }
            };
            code += &format!("__ui_el=__ui_el.expanded(::core::option::Option::Some({value}));");
        }
        if let Some(a) = e.attr("aria-haspopup") {
            if !matches!(Self::literal(a)?, "true" | "menu" | "listbox" | "dialog") {
                return Err(a
                    .loc
                    .error("aria-haspopup expects true, menu, listbox or dialog"));
            }
            if e.attr("aria-expanded").is_none() {
                return Err(a.loc.error(
                    "aria-haspopup needs aria-expanded={open}: UI Automation exposes popup triggers with the ExpandCollapse pattern",
                ));
            }
        }
        Ok(code)
    }
}
