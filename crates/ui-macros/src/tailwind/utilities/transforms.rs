//! Transformations : `translate-*`, `scale-*`, `rotate-*`, `origin-*`, `skew-*` et
//! transformations 3D refusées.
use super::{
    Miss, R, Val, decls, invalid,
    values::{Len, length, negate, percent_number},
};
use crate::tailwind::{Config, candidate::Candidate};

/// Utilitaires fonctionnels de la famille ; `Miss::Unknown` pour une autre racine.
pub(super) fn functional(root: &str, v: &Val, c: &Candidate, cfg: &Config) -> R {
    let one = |prop: &str, value: String| Ok(decls(vec![(prop, value)]));
    let len = |opts: Len| length(v, c, cfg, opts);
    match root {
        "translate-x" | "translate-y" | "translate" => {
            let value = len(Len {
                fraction: true,
                negative: true,
                keywords: &[("full", "100%"), ("px", "1px")],
                ..Default::default()
            })?;
            let mut list = Vec::new();
            if root != "translate-y" {
                list.push(("--tw-translate-x", value.clone()));
            }
            if root != "translate-x" {
                list.push(("--tw-translate-y", value));
            }
            list.push((
                "translate",
                "var(--tw-translate-x) var(--tw-translate-y)".into(),
            ));
            Ok(decls(list))
        }
        "scale" | "scale-x" | "scale-y" => {
            let value = percent_number(v, c)?;
            let mut list = Vec::new();
            if root != "scale-y" {
                list.push(("--tw-scale-x", value.clone()));
            }
            if root != "scale-x" {
                list.push(("--tw-scale-y", value.clone()));
            }
            if root == "scale" {
                list.push(("--tw-scale-z", value));
            }
            list.push(("scale", "var(--tw-scale-x) var(--tw-scale-y)".into()));
            Ok(decls(list))
        }
        "rotate" => {
            let value = match v {
                Val::Named(n) if n.parse::<f32>().is_ok() && c.modifier.is_none() => {
                    format!("{n}deg")
                }
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                _ => return Err(Miss::Unknown),
            };
            one("rotate", if c.negative { negate(&value) } else { value })
        }
        "origin" => {
            let value = match v {
                // Mots-clés en pourcentages, la forme que lit le moteur.
                Val::Named(n) => match n.as_str() {
                    "center" => "50% 50%",
                    "top" => "50% 0%",
                    "right" => "100% 50%",
                    "bottom" => "50% 100%",
                    "left" => "0% 50%",
                    "top-right" => "100% 0%",
                    "top-left" => "0% 0%",
                    "bottom-right" => "100% 100%",
                    "bottom-left" => "0% 100%",
                    _ => return Err(Miss::Unknown),
                }
                .to_owned(),
                Val::Arb(v, _) | Val::Var(v, _) => v.clone(),
                Val::Bare => return Err(Miss::Unknown),
            };
            one("transform-origin", value)
        }
        "skew" | "skew-x" | "skew-y" => {
            invalid("skew transforms are not implemented by the engine")
        }
        "perspective" | "rotate-x" | "rotate-y" | "translate-z" | "scale-z" => {
            invalid("3D transforms are not implemented by the engine")
        }
        _ => Err(Miss::Unknown),
    }
}
