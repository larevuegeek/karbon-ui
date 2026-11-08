//! États ARIA déclarés : `aria-pressed`, `aria-checked`, `aria-selected`,
//! `aria-current`. Valeurs littérales vérifiées à la compilation, expressions
//! converties à l'exécution (`ui::template::aria_check`, `aria_current`).
use super::{Generator, R};
use crate::{
    Result,
    template::markup::{Attr, Element, Value},
};

impl Generator {
    pub(super) fn aria_states(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        let role = match e.attr("role") {
            Some(a) => Some(Self::literal(a)?),
            None => None,
        };
        if let Some(a) = e.attr("aria-pressed") {
            let state = self.check_state(a)?;
            code +=
                &format!("__ui_el=__ui_el.aria_pressed(::core::option::Option::Some({state}));");
        }
        if let Some(a) = e.attr("aria-checked") {
            let state = match role {
                Some("radio" | "menuitemradio") => {
                    format!("{R}::ui::element::ChoiceState::Radio({})", self.flag(a)?)
                }
                Some("checkbox" | "switch" | "menuitemcheckbox") => format!(
                    "{R}::ui::element::ChoiceState::Toggle({})",
                    self.check_state(a)?
                ),
                _ => {
                    return Err(a.loc.error(
                        "aria-checked needs role=\"checkbox\", \"switch\", \"radio\", \"menuitemcheckbox\" or \"menuitemradio\" on the same element (native inputs use checked)",
                    ));
                }
            };
            code +=
                &format!("__ui_el=__ui_el.aria_checked(::core::option::Option::Some({state}));");
        }
        if let Some(a) = e.attr("aria-selected") {
            let value = self.flag(a)?;
            code +=
                &format!("__ui_el=__ui_el.aria_selected(::core::option::Option::Some({value}));");
        }
        if let Some(a) = e.attr("aria-current") {
            let value = match &a.value {
                Value::Text(t) => match t.trim() {
                    "" | "false" => None,
                    "page" => Some("Page"),
                    "step" => Some("Step"),
                    "location" => Some("Location"),
                    "date" => Some("Date"),
                    "time" => Some("Time"),
                    "true" => Some("Item"),
                    _ => {
                        return Err(a.loc.error(
                            "aria-current expects page, step, location, date, time, true, false or {expr}",
                        ));
                    }
                }
                .map(|v| format!("::core::option::Option::Some({R}::ui::element::CurrentItem::{v})")),
                Value::Expr(t) => Some(format!(
                    "{R}::ui::template::aria_current(&({}))",
                    self.out.value(t)
                )),
                Value::Bool => Some(format!(
                    "::core::option::Option::Some({R}::ui::element::CurrentItem::Item)"
                )),
            };
            if let Some(value) = value {
                code += &format!("__ui_el=__ui_el.aria_current({value});");
            }
        }
        Ok(code)
    }
    /// `"true"`, `"false"` ou `{bool}`.
    fn flag(&mut self, a: &Attr) -> Result<String> {
        match &a.value {
            Value::Text(t) if t == "true" => Ok("true".into()),
            Value::Text(t) if t == "false" => Ok("false".into()),
            Value::Expr(t) => Ok(format!("({})", self.out.value(t))),
            _ => Err(a.loc.error(format!(
                "{} expects \"true\", \"false\" or {{bool}}",
                a.name
            ))),
        }
    }
    /// `"true"`, `"false"`, `"mixed"` ou `{bool}` / `{CheckState}`.
    fn check_state(&mut self, a: &Attr) -> Result<String> {
        let state = |s: &str| format!("{R}::ui::element::CheckState::{s}");
        match &a.value {
            Value::Text(t) if t == "true" => Ok(state("On")),
            Value::Text(t) if t == "false" => Ok(state("Off")),
            Value::Text(t) if t == "mixed" => Ok(state("Mixed")),
            Value::Expr(t) => Ok(format!(
                "{R}::ui::template::aria_check(&({}))",
                self.out.value(t)
            )),
            _ => Err(a.loc.error(format!(
                "{} expects \"true\", \"false\", \"mixed\" or {{bool}}",
                a.name
            ))),
        }
    }
}
