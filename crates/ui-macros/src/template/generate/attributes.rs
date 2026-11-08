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
    /// `data-drag-region` : la zone déplace la fenêtre (barres de titre personnalisées) ;
    /// `data-no-drag` : la partie en est exclue (`-webkit-app-region: no-drag`).
    pub(super) fn drag_region(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        for (name, method) in [
            ("data-drag-region", "drag_region"),
            ("data-no-drag", "no_drag"),
        ] {
            code += &match e.attr(name).map(|a| &a.value) {
                None => String::new(),
                Some(Value::Bool) => format!("__ui_el=__ui_el.{method}(true);"),
                Some(Value::Text(t)) if t == "true" || t.is_empty() => {
                    format!("__ui_el=__ui_el.{method}(true);")
                }
                Some(Value::Text(t)) if t == "false" => String::new(),
                Some(Value::Expr(t)) => {
                    let value = self.out.value(t);
                    format!("__ui_el=__ui_el.{method}({value});")
                }
                Some(_) => {
                    return Err(e.loc.error(format!(
                        "{name} expects no value, \"true\", \"false\" or a Rust bool"
                    )));
                }
            };
        }
        if e.attr("data-drag-region").is_some() && e.attr("data-no-drag").is_some() {
            return Err(e
                .loc
                .error("data-drag-region and data-no-drag cannot both be set on one element"));
        }
        Ok(code)
    }
    /// `data-resize-region="bottom-right"` : la zone redimensionne la fenêtre par ce bord
    /// ou ce coin (poignée d'une fenêtre sans cadre).
    pub(super) fn resize_region(&mut self, e: &Element) -> Result<String> {
        const EDGES: [(&str, &str); 8] = [
            ("left", "Left"),
            ("right", "Right"),
            ("top", "Top"),
            ("bottom", "Bottom"),
            ("top-left", "TopLeft"),
            ("top-right", "TopRight"),
            ("bottom-left", "BottomLeft"),
            ("bottom-right", "BottomRight"),
        ];
        let Some(a) = e.attr("data-resize-region") else {
            return Ok(String::new());
        };
        match &a.value {
            Value::Text(t) => match EDGES.iter().find(|(name, _)| name == t) {
                Some((_, edge)) => Ok(format!(
                    "__ui_el=__ui_el.resize_region({R}::window::ResizeEdge::{edge});"
                )),
                None => Err(a.loc.error(
                    "data-resize-region expects left, right, top, bottom, top-left, top-right, bottom-left or bottom-right",
                )),
            },
            _ => Err(a
                .loc
                .error("data-resize-region expects a literal edge, such as \"bottom-right\"")),
        }
    }
    /// `aria-hidden` : sous-arbre retiré de l'arbre d'accessibilité.
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
                "menuitem" | "menuitemcheckbox" | "menuitemradio" => "MenuItem",
                "menu" => "Menu",
                "listbox" | "list" => "List",
                "option" | "listitem" => "ListItem",
                "separator" => "Separator",
                "group" => "Group",
                "button" => "Button",
                "img" => "Image",
                "heading" => "Heading",
                "navigation" => "Navigation",
                "tab" => "Tab",
                "tablist" => "TabList",
                "tabpanel" => "TabPanel",
                "radio" => "RadioButton",
                "radiogroup" => "RadioGroup",
                "checkbox" => "Checkbox",
                "switch" => "Switch",
                "none" | "presentation" => "Custom",
                other => {
                    return Err(a.loc.error(format!(
                        "role=\"{other}\" is not supported; use menuitem, menuitemcheckbox, menuitemradio, menu, option, listbox, separator, group, button, img, heading, navigation, tab, tablist, tabpanel, radio, radiogroup, checkbox, switch or none"
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
        code += &self.aria_states(e)?;
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
