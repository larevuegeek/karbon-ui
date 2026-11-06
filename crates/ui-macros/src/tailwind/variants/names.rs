//! Variantes connues : pseudo-classes simples, rangs des familles dans l'ordre de
//! Tailwind, noms proposés en suggestion et raisons des variantes refusées.
use super::STATE;
use crate::template::html;
/// Variantes à simple pseudo-classe, avec leur rang dans l'ordre de Tailwind.
const PSEUDO: &[(&str, &str, u32)] = &[
    ("first", "FirstChild", 20),
    ("last", "LastChild", 21),
    ("only", "OnlyChild", 22),
    ("odd", "NthChild(2,1)", 23),
    ("even", "NthChild(2,0)", 24),
    ("checked", "Checked", 30),
    ("indeterminate", "Indeterminate", 31),
    ("placeholder-shown", "PlaceholderShown", 32),
    ("read-only", "ReadOnly", 42),
    ("empty", "Empty", 43),
    ("focus-within", "FocusWithin", 44),
    ("hover", "Hover", 45),
    ("focus", "Focus", 46),
    ("focus-visible", "FocusVisible", 47),
    ("active", "Active", 48),
    ("enabled", "Enabled", 49),
    ("disabled", "Disabled", 50),
];
/// Rangs des familles de variantes (ordre d'enregistrement de Tailwind v4).
pub(super) mod rank {
    pub const NOT: u32 = 1;
    pub const GROUP: u32 = 2;
    pub const PEER: u32 = 3;
    pub const IN: u32 = 4;
    pub const ARIA: u32 = 51;
    pub const DATA: u32 = 52;
    pub const BEFORE: u32 = 13;
    pub const AFTER: u32 = 14;
    pub const PLACEHOLDER: u32 = 16;
    pub const MOTION: u32 = 53;
    pub const ARBITRARY: u32 = 55;
    pub const NTH: u32 = 56;
    /// Points de rupture : rang + largeur en px (max-* décroissants, puis min).
    pub const MAX: u32 = 1 << 12;
    pub const MIN: u32 = 2 << 12;
    pub const CONTAINER_MAX: u32 = 3 << 12;
    pub const CONTAINER_MIN: u32 = 4 << 12;
    pub const ORIENTATION: u32 = 5 << 12;
    pub const DARK: u32 = (5 << 12) + 10;
}
/// Noms des variantes reconnues, pour les suggestions.
const NAMES: &[&str] = &[
    "hover",
    "focus",
    "focus-visible",
    "focus-within",
    "active",
    "disabled",
    "enabled",
    "checked",
    "indeterminate",
    "read-only",
    "placeholder-shown",
    "first",
    "last",
    "only",
    "odd",
    "even",
    "empty",
    "before",
    "after",
    "dark",
    "sm",
    "md",
    "lg",
    "xl",
    "2xl",
    "max-sm",
    "max-md",
    "max-lg",
    "max-xl",
    "max-2xl",
    "portrait",
    "landscape",
    "group-hover",
    "group-focus",
    "group-focus-within",
    "peer-checked",
    "peer-hover",
    "peer-focus",
];
pub(super) fn pseudo_class(name: &str) -> Option<(String, u32)> {
    PSEUDO
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, s, r)| (format!("{STATE}::{s}"), *r))
}
/// Attributs ARIA booléens de Tailwind : `aria-expanded:` vaut `[aria-expanded="true"]`.
pub(super) const ARIA: &[&str] = &[
    "busy", "checked", "disabled", "expanded", "hidden", "pressed", "readonly", "required",
    "selected",
];
pub(super) fn unsupported(variant: &str) -> String {
    let family = variant.split(['-', '[']).next().unwrap_or(variant);
    let reason = match family {
        "has" => ":has() is not implemented by the engine",
        "selection" | "marker" | "file" | "first" | "backdrop" | "details" => {
            "this pseudo-element cannot be styled by the engine"
        }
        "contrast" | "forced" | "print" | "pointer" | "any" | "noscript" | "inverted" => {
            "this media feature is not implemented by the engine"
        }
        "supports" | "starting" => "this at-rule is not implemented by the engine",
        "ltr" | "rtl" => "direction variants are not implemented",
        "open" | "inert" | "visited" | "target" | "default" | "optional" | "required" | "valid"
        | "invalid" | "user" | "autofill" => "this pseudo-class is not implemented by the engine",
        _ => "",
    };
    if reason.is_empty() {
        match html::suggest(variant, NAMES) {
            Some(s) => format!("Unknown variant '{variant}:'; did you mean '{s}:'?"),
            None => format!("Unknown variant '{variant}:'"),
        }
    } else {
        format!("Variant '{variant}:' is not supported: {reason}")
    }
}
