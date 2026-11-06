//! Transitions et animations : `transition-*`, `duration-*`, `delay-*`, `ease-*`,
//! `animate-*`.
use super::{Miss, R, Val, composed::transition, decls, values::duration};
use crate::tailwind::{Config, candidate::Candidate};
use crate::template::html;

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    match root {
        "duration" => {
            let value = duration(v, c)?;
            Ok(decls(vec![
                ("--tw-duration", value.clone()),
                ("transition-duration", value),
            ]))
        }
        "delay" => one("transition-delay", duration(v, c)?),
        "ease" => {
            let value = match v {
                Val::Named(n) => cfg.lookup("ease", n).ok_or(Miss::Unknown)?,
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                Val::Bare => return Err(Miss::Unknown),
            };
            Ok(decls(vec![
                ("--tw-ease", value.clone()),
                ("transition-timing-function", value),
            ]))
        }
        "transition" => match v {
            Val::Arb(v, _) | Val::Var(v, _) => Ok(transition(&v.clone())),
            _ => Err(Miss::Unknown),
        },
        "animate" => {
            let (value, name) = match v {
                Val::Named(n) if n == "none" => return one("animation", "none".into()),
                Val::Named(n) => {
                    let value = cfg.lookup("animate", n).ok_or_else(|| {
                        let keys = cfg.keys("animate");
                        let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
                        match html::suggest(n, &keys) {
                            Some(s) => Miss::Invalid(format!("Unknown animation '{n}'; did you mean 'animate-{s}'?")),
                            None => Miss::Invalid(format!("Unknown animation '{n}': declare --animate-{n} and its @keyframes in @theme")),
                        }
                    })?;
                    let name = value.split_whitespace().next().unwrap_or("").to_owned();
                    (value, name)
                }
                Val::Arb(v, _) => (
                    v.clone(),
                    v.split_whitespace().next().unwrap_or("").to_owned(),
                ),
                _ => return Err(Miss::Unknown),
            };
            let mut o = one("animation", value)?;
            if let Some(frames) = cfg.keyframes(&name) {
                o.keyframes.push(frames.to_owned());
            }
            Ok(o)
        }
        _ => Err(Miss::Unknown),
    }
}
