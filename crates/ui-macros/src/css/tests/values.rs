//! Traduction des valeurs et propriétés CSS en propriétés du moteur.
use crate::css::values;
use std::collections::BTreeMap;
#[test]
fn css_variables_resolve_fallbacks_without_touching_strings_and_reject_cycles() {
    let vars = BTreeMap::from([
        ("--Accent".into(), "#f80".into()),
        ("--bg".into(), "var(--Accent)".into()),
    ]);
    assert_eq!(values::resolve("var(--bg)", &vars).unwrap(), "#f80");
    assert_eq!(
        values::resolve("var(--missing, rgba(1, 2, 3, .5))", &vars).unwrap(),
        "rgba(1, 2, 3, .5)"
    );
    assert_eq!(
        values::resolve("\"var(--bg)\"", &vars).unwrap(),
        "\"var(--bg)\""
    );
    assert_eq!(
        values::resolve("var(--font, \"A ) B\")", &vars).unwrap(),
        "\"A ) B\""
    );
    assert!(values::resolve("var(--accent)", &vars).is_err());
    let cycle = BTreeMap::from([
        ("--x".into(), "var(--y)".into()),
        ("--y".into(), "var(--x)".into()),
    ]);
    assert!(values::resolve("var(--x)", &cycle).is_err());
}
#[test]
fn colors_gradients_and_css_angles_are_translated_explicitly() {
    assert!(
        values::color("#abcd")
            .unwrap()
            .ends_with("rgba(170,187,204,221)")
    );
    assert!(
        values::color("rgb(100% 0% 50% / 50%)")
            .unwrap()
            .ends_with("rgba(255,0,128,128)")
    );
    assert!(
        values::color("rgba(10,20,30,.5)")
            .unwrap()
            .ends_with("rgba(10,20,30,128)")
    );
    assert!(values::color("#xyz").is_err());
    assert!(values::color("rgb(300,0,0)").is_err());
    let gradient = values::property(
        "background",
        "linear-gradient(90deg, #000 0%, #333, #fff 100%)",
    )
    .unwrap();
    assert!(gradient[0].1.contains("new(0.0,"));
    assert!(gradient[0].1.contains("position:0.5"));
    assert!(
        values::property("background", "linear-gradient(#000,#fff)").unwrap()[0]
            .1
            .contains("new(90.0,")
    );
    assert!(values::property("background", "linear-gradient(#000 80%,#fff 20%)").is_err());
}
#[test]
fn supported_properties_translate_and_unknown_values_fail_explicitly() {
    assert_eq!(
        values::property("padding", "2px 4px 6px").unwrap()[0].1,
        "[2.0,4.0,6.0]"
    );
    assert_eq!(
        values::property("margin", "-1px").unwrap()[0].0,
        "css_margin"
    );
    assert_eq!(
        values::property("font-family", "\"Consolas\"").unwrap()[0].0,
        "font_family"
    );
    assert_eq!(values::property("font-weight", "bold").unwrap()[0].1, "700");
    assert_eq!(
        values::property("grid-template-columns", "repeat(3, 1fr)").unwrap()[0].0,
        "grid_tracks"
    );
    for (name, value) in [("position", "sticky"), ("backdrop-filter", "url(#x)")] {
        assert!(values::property(name, value).is_err(), "{name}: {value}");
    }
}
#[test]
fn spacing_accepts_relative_units_and_expressions_but_not_percentages() {
    let names = |n: &str, v: &str| {
        values::property(n, v)
            .unwrap()
            .into_iter()
            .map(|p| p.0)
            .collect::<Vec<_>>()
    };
    // Tout en px ou auto : valeurs constantes inchangées.
    assert_eq!(
        values::property("gap", "4px").unwrap()[0],
        ("gap".into(), "4.0".into())
    );
    assert_eq!(
        values::property("margin", "-1px auto").unwrap()[0].1,
        "[Some(-1.0),None,Some(-1.0),None]"
    );
    // Expressions : un emplacement par côté, après les côtés constants du raccourci.
    let gap = values::property("gap", "1em").unwrap();
    assert_eq!(gap.len(), 2);
    assert!(gap[0].1.starts_with("(8usize,") && gap[0].1.contains("Em(1.0)"));
    assert!(gap[1].1.starts_with("(9usize,"));
    assert_eq!(
        names("padding", "1em 4px"),
        ["padding", "spacing_expression", "spacing_expression"]
    );
    assert_eq!(
        values::property("padding", "1em 4px").unwrap()[0].1,
        "[0.0,4.0,0.0,4.0]"
    );
    assert_eq!(
        names("padding", "calc(1rem + 2px)"),
        ["spacing_expression"; 4]
    );
    assert_eq!(
        names("margin", "1em auto"),
        ["css_margin", "spacing_expression", "spacing_expression"]
    );
    assert_eq!(
        values::property("margin", "1em auto").unwrap()[0].1,
        "[Some(0.0),None,Some(0.0),None]"
    );
    let left = values::property("margin-left", "-1em").unwrap();
    assert!(left[0].1.starts_with("(7usize,") && left[0].1.contains("Mul("));
    for (name, value) in [
        ("padding-top", "2vw"),
        ("row-gap", "clamp(4px, 1vh, 20px)"),
        ("column-gap", "min(1em, 10px)"),
        ("gap", "1rem 2vw"),
        ("margin-bottom", "max(2px, .5em)"),
    ] {
        assert!(
            names(name, value).iter().all(|n| n == "spacing_expression"),
            "{name}: {value}"
        );
    }
    // Variables : le côté lit la variable à l'exécution, la feuille donne le repli.
    let vars = BTreeMap::from([("--space".into(), "1.5em".into())]);
    let p = values::declaration("padding", "var(--space) 0", &vars).unwrap();
    assert_eq!(p[0], ("padding".into(), "[0.0,0.0,0.0,0.0]".into()));
    assert!(p[1].1.contains("Variable{name:\"--space\"") && p[1].1.contains("Em(1.5)"));
    let error = |n: &str, v: &str| values::property(n, v).unwrap_err();
    assert!(error("padding", "5%").contains("percentages are not supported"));
    assert!(error("gap", "calc(10% + 1em)").contains("percentages are not supported"));
    assert!(error("padding", "-1em").contains("Negative spacing"));
    assert!(error("padding-left", "-2px").contains("Negative spacing"));
    assert!(error("margin", "1foo").contains("is not a spacing length"));
    assert!(error("padding", "auto").contains("is not a spacing length"));
    assert!(error("gap", "1px 2px 3px").contains("one or two"));
    assert!(error("padding", "1px 2px 3px 4px 5px").contains("1..4"));
}
#[test]
fn transforms_keep_order_and_transitions_share_timing() {
    let transform = values::transform("translate(12px, -4px) rotate(.25turn) scale(1.1)").unwrap();
    assert!(transform.contains(".translate(12.0,-4.0).rotate(90.0).scale(1.1)"));
    assert!(values::transform("scale(2) translateX(10px)").is_err());
    assert!(values::transform("translate(1px) translateX(2px)").is_err());
    assert!(
        values::property(
            "transition",
            "transform 0.15s ease-out, opacity 150ms ease-out"
        )
        .is_ok()
    );
    assert!(
        values::property(
            "transition",
            "transform 100ms ease-out, opacity 200ms ease-out"
        )
        .is_err()
    );
    // Dimensions : interpolées seulement sur demande (ou `all`).
    let width = values::property("transition", "width 200ms ease").unwrap();
    assert!(format!("{width:?}").contains(".properties(false,false).sizes(true)"));
    let all = values::property("transition", "all 150ms").unwrap();
    assert!(format!("{all:?}").contains(".properties(true,true).sizes(true)"));
    let colors = values::property("transition", "color 150ms").unwrap();
    assert!(!format!("{colors:?}").contains("sizes"));
    assert!(values::property("transition", "padding 150ms").is_err());
}
#[test]
fn text_properties_compile_to_typed_values_and_reject_unknown_keywords() {
    let p = |n: &str, v: &str| values::property(n, v).map(|p| p[0].1.clone());
    assert!(
        p("letter-spacing", "-0.025em")
            .unwrap()
            .ends_with("Em(-0.025)")
    );
    assert!(p("letter-spacing", "1.5px").unwrap().ends_with("Px(1.5)"));
    assert!(
        p("text-transform", "capitalize")
            .unwrap()
            .ends_with("Capitalize")
    );
    assert!(p("white-space", "pre-wrap").unwrap().ends_with("PreWrap"));
    assert!(
        p("text-overflow", "ellipsis")
            .unwrap()
            .ends_with("Ellipsis")
    );
    assert_eq!(p("-webkit-line-clamp", "3").unwrap(), "3u16");
    assert_eq!(p("font-style", "italic").unwrap(), "true");
    assert_eq!(p("font-variant-numeric", "tabular-nums").unwrap(), "true");
    assert_eq!(
        p(
            "font-family",
            "'Inter Var', Inter, ui-sans-serif, system-ui"
        )
        .unwrap(),
        "\"Inter Var,Inter,Segoe UI\""
    );
    assert_eq!(p("font-family", "Segoe UI").unwrap(), "\"Segoe UI\"");
    assert!(
        values::property("-webkit-box-orient", "vertical")
            .unwrap()
            .is_empty()
    );
    for (name, value) in [
        ("letter-spacing", "1rem"),
        ("text-transform", "full-width"),
        ("white-space", "pre-line"),
        ("text-overflow", "\"-\""),
        ("line-clamp", "0"),
        ("font-style", "oblique 10deg"),
        ("font-variant-numeric", "slashed-zero"),
        ("font-family", "inherit"),
        ("font-family", "A, , B"),
        ("-webkit-box-orient", "horizontal"),
    ] {
        assert!(values::property(name, value).is_err(), "{name}: {value}");
    }
}
#[test]
fn color_functions_fold_constants_and_keep_runtime_references() {
    let c = |v: &str| values::color(v).unwrap();
    assert!(c("oklch(62.8% 0.2577 29.23)").ends_with("rgba(255,0,0,255)"));
    assert!(c("oklch(0.628 0.2577 29.23deg / 50%)").ends_with("rgba(255,0,0,128)"));
    assert!(c("oklab(1 0 0)").ends_with("rgba(255,255,255,255)"));
    assert!(c("hsl(240 100% 50%)").ends_with("rgba(0,0,255,255)"));
    assert!(c("hsla(120, 100%, 25%, .5)").ends_with("rgba(0,128,0,128)"));
    assert!(c("color-mix(in srgb, white 25%, black)").ends_with("rgba(64,64,64,255)"));
    assert!(c("color-mix(in srgb, red 20%, blue 20%)").ends_with("rgba(128,0,128,102)"));
    let vars = BTreeMap::from([("--h".into(), "210".into())]);
    let p = |v: &str| values::declaration("background-color", v, &vars).unwrap();
    let dynamic = p("oklch(70% 0.1 var(--h))");
    assert_eq!(dynamic[0].0, "color_expression_background");
    assert!(dynamic[0].1.contains("Variable{name:\"--h\""));
    let current = values::property("border-color", "currentColor").unwrap();
    assert!(current[0].1.contains("CurrentColor"));
    assert!(
        values::property("color", "currentcolor")
            .unwrap()
            .is_empty()
    );
    for bad in [
        "oklch(70%, 0.1, 210)",
        "hsl(10 20%)",
        "color-mix(in hsl, red, blue)",
        "color-mix(in srgb, red 120%, blue)",
        "lab(50% 20 30)",
        "oklch(150% 0.1 20)",
        "currentColor",
    ] {
        assert!(values::color(bad).is_err(), "{bad}");
    }
    assert!(values::declaration("color", "var(--missing)", &vars).is_err());
}
#[test]
fn element_custom_properties_compile_to_typed_literal_values() {
    let v = |s: &str| values::custom_value(s).unwrap();
    assert!(v("210").ends_with("Number(210.0)"));
    assert!(v("-4px").ends_with("Length::Px(-4.0))"));
    assert!(v("50%").ends_with("Length::Percent(50.0))"));
    assert!(v("calc(1em + 2px)").contains("Length::Computed("));
    assert!(
        v("oklch(62.8% 0.2577 29.23)")
            .contains("Color(__ui_root::ui::theme::Color::rgba(255,0,0,255))")
    );
    assert!(v("\"Inter\"").ends_with("Family(\"Inter\")"));
    for bad in ["var(--h)", "auto", "currentColor", "1fr"] {
        assert!(values::custom_value(bad).is_err(), "{bad}");
    }
}
#[test]
fn box_properties_expand_sides_outlines_and_ratios() {
    let names = |n: &str, v: &str| {
        values::property(n, v)
            .unwrap()
            .into_iter()
            .map(|p| p.0)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names("border-width", "0 0 1px"),
        [
            "border_edge_top",
            "border_edge_right",
            "border_edge_bottom",
            "border_edge_left"
        ]
    );
    assert_eq!(
        names("border-bottom", "1px solid #fff"),
        ["border_edge_bottom", "border_color_bottom"]
    );
    assert_eq!(
        names("border-left", "2px solid currentColor"),
        ["border_edge_left"]
    );
    assert_eq!(
        names("outline", "2px solid red"),
        ["outline_width", "outline_color"]
    );
    assert_eq!(
        names("inset", "0"),
        ["offset_top", "offset_right", "offset_bottom", "offset_left"]
    );
    assert_eq!(
        values::property("aspect-ratio", "16 / 9").unwrap()[0].1,
        "Some(1.7777778)"
    );
    assert!(values::property("grid-template-columns", "max-content fit-content(20px) 1fr").is_ok());
    assert!(
        values::property(
            "grid-template-columns",
            "min-content minmax(min-content, 1fr)"
        )
        .is_ok()
    );
    assert!(values::property("align-items", "baseline").is_ok());
    for (n, v) in [
        ("border-top", "1px dashed red"),
        ("outline", "1px dotted red"),
        ("aspect-ratio", "0"),
        ("inset", "1px 2px 3px 4px 5px"),
        ("justify-self", "baseline"),
        ("align-items", "first-baseline"),
    ] {
        assert!(values::property(n, v).is_err(), "{n}: {v}");
    }
}
