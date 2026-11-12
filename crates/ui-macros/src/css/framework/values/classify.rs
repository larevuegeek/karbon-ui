//! Classement d'une déclaration : traduite à la compilation par le compilateur commun,
//! évaluée à l'exécution (variante `DynamicProperty` et jetons), propriété
//! personnalisée, sans effet natif, ou refusée avec la catégorie du rapport.
use super::normalize::{find_var, rem_to_px};
use super::tokens::tokenize;
use crate::css::{animation, values as common};
use std::collections::BTreeMap;

/// Variante `DynamicProperty` d'une propriété évaluable à l'exécution.
fn dynamic_property(name: &str) -> Option<String> {
    const NAMES: &[&str] = &[
        "color",
        "background-color",
        "background",
        "border-color",
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
        "accent-color",
        "outline-color",
        "outline",
        "outline-width",
        "outline-offset",
        "border",
        "border-top",
        "border-right",
        "border-bottom",
        "border-left",
        "border-width",
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
        "border-radius",
        "padding",
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
        "margin",
        "margin-top",
        "margin-right",
        "margin-bottom",
        "margin-left",
        "width",
        "height",
        "min-width",
        "min-height",
        "max-width",
        "max-height",
        "flex-basis",
        "flex-grow",
        "flex-shrink",
        "gap",
        "row-gap",
        "column-gap",
        "font-size",
        "font-weight",
        "line-height",
        "letter-spacing",
        "opacity",
        "box-shadow",
        "transform",
        "translate",
        "scale",
        "rotate",
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
        "background-image",
        "background-position",
        "background-size",
        "background-repeat",
        "appearance",
    ];
    if !NAMES.contains(&name) {
        return None;
    }
    Some(
        name.split('-')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                    .unwrap_or_default()
            })
            .collect(),
    )
}
/// Classement d'une déclaration d'un élément.
#[derive(Clone, Debug)]
pub enum Kind {
    /// Traduite à la compilation par le compilateur commun (valeur normalisée).
    Static(String),
    /// Évaluée à l'exécution : variante `DynamicProperty` et jetons.
    Dynamic(String, Vec<String>),
    /// Propriété personnalisée (jetons de sa valeur).
    Custom(Vec<String>),
    /// Sans effet natif, acceptée (`display: inline-block` dans un flux en blocs…).
    Ignored,
    /// Refusée : catégorie du rapport.
    Skip(String),
}
/// Propriétés sans effet visuel dans le moteur, acceptées sans rien produire.
const IGNORED: &[&str] = &[
    "-webkit-text-size-adjust",
    "text-size-adjust",
    "-webkit-tap-highlight-color",
    "-webkit-font-smoothing",
    "-moz-osx-font-smoothing",
    "text-rendering",
    "font-feature-settings",
    "font-variation-settings",
    "tab-size",
    "-moz-tab-size",
    "color-scheme",
    "scroll-behavior",
    "print-color-adjust",
    "-webkit-print-color-adjust",
    "will-change",
    "contain",
    "user-select",
    "-webkit-user-select",
    "-moz-user-select",
    "touch-action",
    "-webkit-appearance",
    "-moz-appearance",
    "word-wrap",
    "overflow-wrap",
    "word-break",
    "hyphens",
    "-webkit-hyphens",
    "speak",
    "forced-color-adjust",
    "font-kerning",
    "-webkit-box-decoration-break",
    "box-decoration-break",
    "unicode-bidi",
    "caret-color",
    "resize",
    "backface-visibility",
    "-webkit-backface-visibility",
    "perspective",
    "isolation",
    "zoom",
    "scroll-margin-top",
    "scroll-padding-top",
    "-webkit-overflow-scrolling",
    "orphans",
    "widows",
    "page-break-inside",
    "break-inside",
];
/// Propriétés de texte héritées par le moteur.
const INHERITED: &[&str] = &[
    "color",
    "font",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "line-height",
    "letter-spacing",
    "text-align",
    "text-transform",
    "white-space",
    "cursor",
    "direction",
    "visibility",
    "word-spacing",
    "text-indent",
];
/// Classe une déclaration après substitution des variables connues.
pub fn classify(name: &str, value: &str, vars: &BTreeMap<String, String>) -> Kind {
    if name.starts_with("--") {
        // `rem` gardés : la valeur est évaluée à l'exécution.
        return match tokenize(value) {
            Ok(t) => Kind::Custom(t),
            Err(e) => Kind::Skip(format!("custom property value ({e})")),
        };
    }
    let lower = value.trim().to_ascii_lowercase();
    // `border-*-radius: inherit` : rayons du parent, lus pendant la résolution.
    if lower == "inherit"
        && (name == "border-radius" || (name.starts_with("border-") && name.ends_with("-radius")))
        && let Some(variant) = dynamic_property(name)
        && let Ok(tokens) = tokenize("inherit")
    {
        return Kind::Dynamic(variant, tokens);
    }
    if ["inherit", "initial", "unset", "revert", "revert-layer"].contains(&lower.as_str()) {
        // Propriétés héritées par le moteur : `inherit` / `unset` ne changent rien.
        return if INHERITED.contains(&name) && (lower == "inherit" || lower == "unset") {
            Kind::Ignored
        } else {
            Kind::Skip(format!("css-wide keyword {lower}"))
        };
    }
    if IGNORED.contains(&name) {
        return Kind::Ignored;
    }
    // `transition` : liste composée, souvent lue par `var()` (`--bs-accordion-transition`),
    // évaluée à la résolution des styles comme les autres déclarations dynamiques, dans
    // l'ordre de la cascade.
    if name == "transition" {
        return match tokenize(value) {
            Ok(tokens) => Kind::Dynamic("Transition".into(), tokens),
            Err(e) => Kind::Skip(format!("transition: {e}")),
        };
    }
    // Animations `@keyframes` : compilées par le compilateur commun quand la valeur est
    // connue à la compilation (variables de l'élément substituées, `.spinner-border`).
    if name == "animation" || name.starts_with("animation-") {
        if find_var(value).is_some() {
            return Kind::Skip(format!("{name} with var()"));
        }
        let mut scratch = BTreeMap::new();
        return match animation::declaration(name, &rem_to_px(value), &mut scratch) {
            Ok(()) => Kind::Static(value.trim().to_owned()),
            Err(e) => Kind::Skip(format!("{name}: {e}")),
        };
    }
    // Préfixes des autres moteurs : ignorés comme par un navigateur Chromium.
    if name.starts_with("-moz-") || name.starts_with("-ms-") || name.starts_with("-o-") {
        return Kind::Ignored;
    }
    if name == "content" {
        return match common::pseudo_content(value) {
            Ok(_) => Kind::Static(value.to_owned()),
            Err(_) => Kind::Skip("content: value".into()),
        };
    }
    // `appearance: none` : contrôle dessiné par sa boîte CSS (cases, curseur, liste).
    if name == "appearance" {
        return match tokenize(value) {
            Ok(tokens) => Kind::Dynamic("Appearance".into(), tokens),
            Err(e) => Kind::Skip(format!("appearance: {e}")),
        };
    }
    // Fond avec image SVG en URL `data:` (`transparent url(…) center/1em no-repeat`) :
    // couleur et couche d'image évaluées ensemble.
    if name == "background"
        && value.contains("url(")
        && let Ok(tokens) = tokenize(&rem_to_px(value))
    {
        return Kind::Dynamic("Background".into(), tokens);
    }
    // Autre image (fichier) : la couche de couleur seule, l'image n'est pas rendue.
    if name == "background" && value.contains("url(") {
        let color = common::words(value)
            .unwrap_or_default()
            .into_iter()
            .find(|w| {
                !w.contains("url(")
                    && !w.contains("var(")
                    && common::constant_color(w).is_ok_and(|c| c.is_some())
            });
        return match color {
            Some(c) => Kind::Static(c),
            None => Kind::Skip("background: url()".into()),
        };
    }
    if name == "z-index" && lower == "auto" {
        return Kind::Static("0".into());
    }
    if name == "background-clip" && (lower == "padding-box" || lower == "border-box") {
        return Kind::Ignored;
    }
    if let Some(unprefixed) = name.strip_prefix("-webkit-")
        && common::declaration(name, value, vars).is_err()
    {
        return match classify(unprefixed, value, vars) {
            Kind::Skip(c) => Kind::Skip(c),
            Kind::Static(_) | Kind::Dynamic(..) if unprefixed != name => {
                // Doublon d'une propriété standard déclarée à côté.
                Kind::Ignored
            }
            other => other,
        };
    }
    if name == "display" {
        match lower.as_str() {
            "inline-block" | "inline" | "list-item" | "table" | "table-row" | "table-cell"
            | "table-row-group" | "table-header-group" | "table-footer-group" | "flow-root"
            | "contents" | "table-caption" | "table-column" => return Kind::Ignored,
            _ => {}
        }
    }
    if name == "vertical-align" || name == "float" || name == "clear" || name == "list-style" {
        return Kind::Ignored;
    }
    let mut normalized = rem_to_px(value);
    if name.contains("radius") {
        // Rayons en pourcentage : 50 % et plus arrondissent la boîte entière (cercle ou
        // pilule), rendu par un rayon en px borné par la normalisation des coins.
        normalized = normalized
            .split(' ')
            .map(
                |w| match w.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
                    Some(n) if n >= 50. => "9999px".to_owned(),
                    _ => w.to_owned(),
                },
            )
            .collect::<Vec<_>>()
            .join(" ");
    }
    let has_var = find_var(&normalized).is_some();
    let has_em = has_unit(&normalized, "em") || has_unit(&normalized, "ch");
    let dynamic = |reason: String| -> Kind {
        match dynamic_property(name) {
            Some(variant) => match tokenize(value) {
                Ok(tokens) => Kind::Dynamic(variant, tokens),
                Err(e) => Kind::Skip(format!("{name}: {e}")),
            },
            None => Kind::Skip(reason),
        }
    };
    // Espacements hors px (vw, vh, calc()…) : évalués à l'exécution par les jetons, comme
    // avant que le compilateur commun ne les accepte, pour garder le rendu des feuilles.
    let spacing_expression = common::spacing_expression(name, &normalized);
    if !has_var && !(has_em && name != "font-size") && !spacing_expression {
        match common::declaration(name, &normalized, vars) {
            Ok(_) => return Kind::Static(normalized),
            Err(e) => {
                let reason = category(name, &e);
                // Calcul ou unité refusés à la compilation : l'évaluation à l'exécution
                // les accepte souvent (padding en calc()…).
                return dynamic(reason);
            }
        }
    }
    dynamic(if has_var {
        format!("var() in {name}")
    } else if spacing_expression && !has_em {
        format!("{name}: value")
    } else {
        format!("em in {name}")
    })
}
fn has_unit(value: &str, unit: &str) -> bool {
    let bytes = value.as_bytes();
    let mut search = 0;
    while let Some(i) = value[search..].find(unit) {
        let at = search + i;
        let before = at.checked_sub(1).map(|j| bytes[j]);
        let after = bytes.get(at + unit.len());
        if before.is_some_and(|b| b.is_ascii_digit() || b == b'.')
            && after.is_none_or(|b| !b.is_ascii_alphanumeric())
        {
            return true;
        }
        search = at + unit.len();
    }
    false
}
/// Catégorie du rapport pour une erreur du compilateur commun.
fn category(name: &str, error: &str) -> String {
    let e = error.to_ascii_lowercase();
    if e.contains("css property '") && e.contains("is not implemented") {
        format!("property {name}")
    } else if e.contains("url(") {
        format!("{name}: url()")
    } else if e.contains("gradient") {
        format!("{name}: gradient")
    } else {
        format!("{name}: value")
    }
}

#[cfg(test)]
mod tests {
    use super::super::logical;
    use super::*;
    #[test]
    fn classification() {
        let vars = BTreeMap::new();
        assert!(matches!(
            classify("padding", ".375rem .75rem", &vars),
            Kind::Static(_)
        ));
        assert!(matches!(
            classify("padding", "var(--a) var(--b)", &vars),
            Kind::Dynamic(..)
        ));
        assert!(matches!(classify("width", "1em", &vars), Kind::Dynamic(..)));
        assert!(matches!(
            classify("display", "inline-block", &vars),
            Kind::Ignored
        ));
        assert!(matches!(classify("color", "inherit", &vars), Kind::Ignored));
        assert!(matches!(
            classify("border-bottom-left-radius", "inherit", &vars),
            Kind::Dynamic(..)
        ));
        assert!(matches!(
            classify("border-width", "inherit", &vars),
            Kind::Skip(_)
        ));
        assert_eq!(
            logical("padding-inline", "1px 2px"),
            [
                ("padding-left".to_string(), "1px".to_string()),
                ("padding-right".into(), "2px".into())
            ]
        );
        assert!(matches!(
            classify("--x", "13, 110, 253", &vars),
            Kind::Custom(_)
        ));
        // `transition` : toujours évaluée à l'exécution (liste composée, `var()`).
        for value in [
            "var(--bs-accordion-transition)",
            "color .15s ease-in-out, box-shadow .15s ease-in-out",
            "height .35s ease",
        ] {
            assert!(
                matches!(classify("transition", value, &vars), Kind::Dynamic(v, _) if v == "Transition"),
                "{value}"
            );
        }
    }
    #[test]
    fn svg_backgrounds_and_appearance_are_evaluated_at_run_time() {
        let vars = BTreeMap::new();
        let url = "url(\"data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg'/%3e\")";
        let dynamic = |name: &str, value: &str| match classify(name, value, &vars) {
            Kind::Dynamic(variant, tokens) => (variant, tokens),
            other => panic!("{name}: {value} → {other:?}"),
        };
        let (variant, tokens) = dynamic("background-image", url);
        assert_eq!(variant, "BackgroundImage");
        assert!(tokens[0].contains("Token::Image("));
        assert_eq!(
            dynamic(
                "background",
                &format!("transparent {url} center/1em auto no-repeat")
            )
            .0,
            "Background"
        );
        assert_eq!(
            dynamic("background-position", "right .75rem center").0,
            "BackgroundPosition"
        );
        assert_eq!(dynamic("background-size", "16px 12px").0, "BackgroundSize");
        assert_eq!(
            dynamic("background-repeat", "no-repeat").0,
            "BackgroundRepeat"
        );
        assert_eq!(dynamic("appearance", "none").0, "Appearance");
        // Variable portant l'image (`--bs-form-select-bg-img`).
        assert!(
            matches!(classify("--icon", url, &vars), Kind::Custom(t) if t[0].contains("Image"))
        );
        // Image de fichier : non rendue, la couleur du raccourci reste.
        assert!(matches!(
            classify("background", "#fff url(icon.png) no-repeat", &vars),
            Kind::Static(c) if c == "#fff"
        ));
        assert!(matches!(
            classify("background-image", "url(icon.png)", &vars),
            Kind::Skip(_)
        ));
    }
}
