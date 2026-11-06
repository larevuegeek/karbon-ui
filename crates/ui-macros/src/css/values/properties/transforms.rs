//! Propriétés CSS : transformations et transitions.
use super::super::*;
use super::{Next, one};

/// Traduit les propriétés de la famille ; passe les autres à `next`.
pub(super) fn property(name: &str, value: &str, next: Next) -> Result<Properties> {
    match name {
        "transform" => one("transform", transform(value)?),
        "transform-origin" => {
            let args = words(value)?;
            if args.len() != 2 {
                return Err("Use transform-origin: X% Y%".into());
            }
            let parse = |s: &str| -> Result<f32> {
                let n = match s {
                    "left" | "top" => 0.,
                    "center" => 0.5,
                    "right" | "bottom" => 1.,
                    s => {
                        number(
                            s.strip_suffix('%')
                                .ok_or("Transform origin needs percentages")?,
                        )? / 100.
                    }
                };
                if !(0. ..=1.).contains(&n) {
                    return Err("Transform origin outside 0..100%".into());
                }
                Ok(n)
            };
            one(
                "origin",
                format!("({:?},{:?})", parse(&args[0])?, parse(&args[1])?),
            )
        }
        "transition" => one("transition", transition(value)?),
        _ => next(),
    }
}
