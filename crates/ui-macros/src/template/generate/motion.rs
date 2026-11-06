//! Animations déclarées dans le balisage : sortie (`exit`), apparition (`reveal`)
//! et décalage des enfants (`stagger`).
use super::{Generator, R};
use crate::{
    Result, css,
    template::{
        html::{self},
        markup::{Attr, Element, Value},
    },
};

impl Generator {
    pub(super) fn exit(&mut self, a: &Attr) -> Result<String> {
        let Value::Text(text) = &a.value else {
            return Err(a.loc.error("data-exit expects \"name [duration] [easing] [delay]\", e.g. data-exit=\"fade-out 200ms\""));
        };
        let words = text.split_whitespace().collect::<Vec<_>>();
        let Some(name) = words.first() else {
            return Err(a.loc.error("data-exit needs an animation name"));
        };
        let mut duration = None;
        let mut delay = None;
        let mut easing = None;
        for word in &words[1..] {
            if let Some(ms) = html::duration(word) {
                if duration.is_none() {
                    duration = Some(ms);
                } else if delay.is_none() {
                    delay = Some(ms);
                } else {
                    return Err(a
                        .loc
                        .error("data-exit takes at most a duration and a delay"));
                }
            } else if let Some(e) = html::easing(word) {
                easing = Some(e);
            } else {
                return Err(a.loc.error(format!(
                    "Invalid data-exit value '{word}': expected a duration (200ms, 0.3s) or an easing (linear, ease, ease-in, ease-out, ease-in-out)"
                )));
            }
        }
        let preset = html::EXIT_PRESETS.iter().find(|(n, _)| n == name);
        let base = match preset {
            Some((_, code)) => code
                .replace("Animation", &format!("{R}::ui::animation::Animation"))
                .replace("Effect", &format!("{R}::ui::animation::Effect")),
            None => {
                let names = html::EXIT_PRESETS
                    .iter()
                    .map(|(n, _)| *n)
                    .collect::<Vec<_>>();
                if let Some(s) = html::suggest(name, &names) {
                    return Err(a.loc.error(format!(
                        "Unknown exit animation '{name}'; did you mean '{s}'?"
                    )));
                }
                let module = self.styles.clone().ok_or_else(|| {
                    a.loc.error(format!(
                        "Unknown exit animation '{name}'. Presets: {}; a @keyframes name needs @styles(module) or ui_file!(…, styles = module)",
                        names.join(", ")
                    ))
                })?;
                let symbol = css::keyframes_symbol(name).map_err(|m| a.loc.error(m))?;
                format!("{R}::ui::animation::Animation::keyframes({module}::keyframes::{symbol})")
            }
        };
        let mut code = format!("{base}.duration({})", duration.unwrap_or(220));
        if let Some(delay) = delay {
            code += &format!(".delay({delay})");
        }
        code += &format!(
            ".easing({R}::ui::animation::Easing::{})",
            easing.unwrap_or("EaseIn")
        );
        Ok(format!("__ui_context.animation({code})"))
    }
    /// `data-reveal="350ms ease [délai]"` (minutage écrit), `data-reveal={expr}`
    /// (`Transition` Rust) ou `data-reveal` seul (`350ms ease`, celui de Bootstrap) ;
    /// `data-reveal=".classe"` est lu par la feuille de framework (`None` ici).
    pub(super) fn reveal(&mut self, a: &Attr) -> Result<Option<String>> {
        let usage = "data-reveal expects \"<duration> [easing] [delay]\" (e.g. \"350ms ease\"), a class of a framework stylesheet (\".collapsing\") or a Rust Transition";
        let text = match &a.value {
            Value::Bool => "350ms ease".to_owned(),
            Value::Text(t) => t.trim().to_owned(),
            Value::Expr(t) => return Ok(Some(self.out.value(t))),
        };
        if text.starts_with('.') {
            if self.styles.is_none() {
                return Err(a.loc.error(format!(
                    "data-reveal=\"{text}\" reads the height transition of a stylesheet class: it needs ui_file!(…, styles = module) with a framework stylesheet"
                )));
            }
            return Ok(None);
        }
        let mut duration = None;
        let mut delay = None;
        let mut easing = None;
        for word in text.split_whitespace() {
            if let Some(ms) = html::duration(word) {
                if duration.is_none() {
                    duration = Some(ms);
                } else if delay.is_none() {
                    delay = Some(ms);
                } else {
                    return Err(a.loc.error(usage));
                }
            } else if let Some(e) = html::easing(word).filter(|_| easing.is_none()) {
                easing = Some(e);
            } else {
                return Err(a
                    .loc
                    .error(format!("Invalid data-reveal value '{word}': {usage}")));
            }
        }
        let duration = duration.ok_or_else(|| a.loc.error(usage))?;
        Ok(Some(format!(
            "{R}::ui::animation::Transition::new().duration({duration}).delay({}).easing({R}::ui::animation::Easing::{})",
            delay.unwrap_or(0),
            easing.unwrap_or("Ease")
        )))
    }
    pub(super) fn stagger(&self, e: &Element) -> Result<String> {
        let Some(a) = e.attr("data-stagger") else {
            return Ok(String::new());
        };
        let ms = match &a.value {
            Value::Text(t) => html::duration(t.trim()).ok_or_else(|| {
                a.loc.error(format!(
                    "Invalid data-stagger '{t}': expected a duration like 40ms or 0.05s"
                ))
            })?,
            _ => {
                return Err(a
                    .loc
                    .error("data-stagger expects a duration, e.g. data-stagger=\"40ms\""));
            }
        };
        Ok(format!(
            "{R}::ui::template::stagger(&mut __ui_children,{ms});"
        ))
    }
}
