//! Valeurs composées écrites comme dans la feuille de Tailwind : listes de variables
//! `--tw-*` de `box-shadow`, `filter`, `backdrop-filter`, dégradés et transitions.
use super::{Out, out, values::looks_like_color};
/// Ombre du thème ou arbitraire, chaque couleur remplacée par
/// `var(--tw-shadow-color, couleur)` comme le fait Tailwind.
pub(super) fn shadow_layers(value: &str, var: &str) -> String {
    let layers = crate::css::parser::split(value, ',').unwrap_or_default();
    layers
        .iter()
        .map(|layer| {
            let words = crate::css::values_words(layer);
            let colored = words
                .iter()
                .position(|w| looks_like_color(w) && !w.starts_with("var(--tw"));
            words
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    if Some(i) == colored {
                        format!("var({var}, {w})")
                    } else {
                        w.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
pub(super) const SHADOW: &str = "var(--tw-inset-shadow), var(--tw-inset-ring-shadow), var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow)";
pub(super) const FILTER: &str = "var(--tw-blur,) var(--tw-brightness,) var(--tw-contrast,) var(--tw-grayscale,) var(--tw-hue-rotate,) var(--tw-invert,) var(--tw-saturate,) var(--tw-sepia,) var(--tw-drop-shadow,)";
pub(super) const BACKDROP: &str = "var(--tw-backdrop-blur,) var(--tw-backdrop-brightness,) var(--tw-backdrop-contrast,) var(--tw-backdrop-grayscale,) var(--tw-backdrop-hue-rotate,) var(--tw-backdrop-invert,) var(--tw-backdrop-opacity,) var(--tw-backdrop-saturate,) var(--tw-backdrop-sepia,)";
pub(super) const GRADIENT_STOPS: &str = "var(--tw-gradient-via-stops, var(--tw-gradient-position), var(--tw-gradient-from) var(--tw-gradient-from-position), var(--tw-gradient-to) var(--tw-gradient-to-position))";
pub(super) const TRANSITION_DEFAULT: &str = "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to, opacity, box-shadow, transform, translate, scale, rotate, filter, -webkit-backdrop-filter, backdrop-filter, display, content-visibility, overlay, pointer-events";
pub(super) const TRANSITION_COLORS: &str = "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to";
pub(super) fn transition(property: &str) -> Out {
    out(&[
        ("transition-property", property),
        (
            "transition-timing-function",
            "var(--tw-ease, var(--default-transition-timing-function))",
        ),
        (
            "transition-duration",
            "var(--tw-duration, var(--default-transition-duration))",
        ),
    ])
}
