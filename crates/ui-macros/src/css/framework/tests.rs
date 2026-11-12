//! Cascade des feuilles de framework : spécificité, ordre, `!important`, couches,
//! `:where()`, règles conditionnelles, variables héritées et at-rules.
use super::selectors::{Facts, Tri};
use super::*;

fn facts(tag: &str, classes: &[&str], attrs: &[(&str, &str)]) -> Facts {
    Facts {
        tag: tag.into(),
        classes: classes.iter().map(|s| s.to_string()).collect(),
        attrs: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}
/// Déclarations retenues pour un élément : (propriété, valeur, conditionnelle).
fn cascade_of(sheet: &Sheet, f: &Facts) -> String {
    let mut class = crate::css::framework_class();
    let mut notes = Vec::new();
    element::compile(sheet, f, "m", &mut class, &mut notes);
    format!("{:?} {:?}", class.props, class.state_props)
}

#[test]
fn cascade_specificity_order_and_important() {
    let sheet = Sheet::compile(
        ".a{color:red;padding:1px} .b{color:blue} .a.b{padding:2px} .c{color:green!important} .a{color:black}",
    );
    let out = cascade_of(&sheet, &facts("div", &["a", "b"], &[]));
    // `.a.b` l'emporte sur `.a` pour padding ; `.a` (plus tard) sur `.b` pour color.
    assert!(out.contains("2.0"), "{out}");
    assert!(!out.contains("1.0,"), "{out}");
    assert!(
        out.contains("Color::rgba(0,0,0")
            || out.contains("0x000000")
            || out.contains("[0.0, 0.0, 0.0"),
        "{out}"
    );
    let out = cascade_of(&sheet, &facts("div", &["a", "c"], &[]));
    assert!(!out.contains("0x000000"), "{out}");
}
#[test]
fn layers_and_where() {
    let sheet = Sheet::compile(
        "@layer base, utilities; @layer utilities{.p{padding:4px}} @layer base{div.p{padding:8px}} :where(.q){padding:16px} .q{padding:32px}",
    );
    // Couche ultérieure plus forte malgré une spécificité moindre.
    let out = cascade_of(&sheet, &facts("div", &["p"], &[]));
    assert!(out.contains("4.0") && !out.contains("8.0"), "{out}");
    let out = cascade_of(&sheet, &facts("div", &["q"], &[]));
    assert!(out.contains("32.0") && !out.contains("16.0"), "{out}");
}
#[test]
fn attribute_and_conditional_rules() {
    let sheet = Sheet::compile(
        "[data-bs-theme=dark]{--bs-body-color:#dee2e6} p{color:var(--bs-body-color)} .btn:hover{color:var(--bs-btn-hover-color)} .btn{--bs-btn-hover-color:#fff} input[type=checkbox]{margin:0}",
    );
    let out = cascade_of(&sheet, &facts("div", &[], &[("data-bs-theme", "dark")]));
    assert!(out.contains("--bs-body-color"), "{out}");
    // La variable de l'élément est connue : la valeur de survol devient statique.
    let out = cascade_of(&sheet, &facts("button", &["btn"], &[]));
    assert!(out.contains("Hover"), "{out}");
    assert!(!out.contains("DynamicDeclaration"), "{out}");
    let tri = selectors::subject_and_context(
        &selectors::parse("input[type=checkbox]").unwrap(),
        Some(&facts("input", &[], &[("type", "text")])),
    );
    assert_eq!(tri, Tri::False);
}
#[test]
fn inherited_variables_are_dynamic() {
    let sheet = Sheet::compile(
        ":root{--bs-body-color:#212529} [data-bs-theme=dark]{--bs-body-color:#dee2e6} body{color:var(--bs-body-color)}",
    );
    assert!(sheet.globals.contains_key("--bs-body-color"));
    assert!(!sheet.constants.contains_key("--bs-body-color"));
    let out = cascade_of(&sheet, &facts("body", &[], &[]));
    assert!(out.contains("DynamicProperty::Color"), "{out}");
}
#[test]
fn supports_media_and_keyframes() {
    let sheet = Sheet::compile(
        "@supports (color: color-mix(in lab, red, red)){.a{color:red}} @supports (display:grid){.b{display:grid}} @media print{.c{color:red}} @media (prefers-reduced-motion: reduce){.d{color:red}} @media (min-width:576px){.e{padding:1px}} @keyframes spin{to{transform:rotate(360deg)}} @font-face{font-family:x}",
    );
    assert_eq!(sheet.report.rules_inactive, 3);
    assert!(sheet.index.contains_key(&Key::Class("b".into())));
    assert!(sheet.index.contains_key(&Key::Class("e".into())));
    assert!(sheet.animations().contains_key("spin"));
    assert_eq!(sheet.report.skipped.len(), 1);
}

/// Entrées mal formées ou non ASCII d'une feuille existante : une erreur ou une valeur
/// intacte, jamais une panique de la macro.
#[test]
fn malformed_and_non_ascii_input_never_panics() {
    assert!(selectors::parse("a[href^]").is_err());
    assert!(selectors::parse("a[lang|]").is_err());
    assert!(selectors::parse("a[href^=\"https\"]").is_ok());
    assert!(super::values::rem_to_px("\"Hélvetica Neue\", 1rem").contains("\"Hélvetica Neue\""));
    let _ = super::values::tokenize("rgba(0, 0, 0, var(--opacité))");
}
