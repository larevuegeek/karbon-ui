//! Compilation des `@keyframes` et des listes `animation` : code produit et erreurs.
use super::{declaration, frames_at, instance, names, parser, timing};
use std::collections::BTreeMap;

fn keyframes(css: &str) -> Result<String, (Option<usize>, String)> {
    let items = parser::parse(css).unwrap();
    let Some(parser::Item::Keyframes { frames, .. }) = items.into_iter().next() else {
        panic!("not a @keyframes: {css}");
    };
    frames_at(&frames, &BTreeMap::new())
}
fn animation(declarations: &[(&str, &str)]) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for (name, value) in declarations {
        declaration(name, value, &mut out)?;
    }
    Ok(out)
}
fn sheet() -> BTreeMap<String, String> {
    ["a", "b", "c"]
        .into_iter()
        .map(|n| (n.to_owned(), format!("FRAMES_{n}")))
        .collect()
}

#[test]
fn partial_steps_compile_to_independent_property_tracks() {
    let code = keyframes(
        "@keyframes k { 100% { opacity: 1 } 0% { opacity: 0 } 50% { transform: translateX(10px) } }",
    )
    .unwrap();
    // Triées, chaque étape ne déclare que ses propriétés.
    let zero = code.find("Keyframe::at(0.0).opacity(0.0)").unwrap();
    let half = code.find("Keyframe::at(0.5).transform(").unwrap();
    let end = code.find("Keyframe::at(1.0).opacity(1.0)").unwrap();
    assert!(zero < half && half < end, "{code}");
    assert!(!code.contains(
        "at(0.5).transform(__ui_root::ui::animation::Effect::NONE.translate(10.0,0.0)).opacity"
    ));
    // Sans 0 % ni 100 % : accepté (la valeur de l'élément complète les extrémités).
    let code = keyframes("@keyframes k { 50% { opacity: .5 } }").unwrap();
    assert_eq!(code.matches("Keyframe::at(").count(), 1, "{code}");
    assert_eq!(keyframes("@keyframes k { }").unwrap(), "&[]");
}

#[test]
fn shared_selectors_from_to_and_duplicates_merge_per_property() {
    let code = keyframes(
        "@keyframes k { 0%, to { opacity: .5; transform: scale(2) } 100% { opacity: 1 } from { opacity: .25 } }",
    )
    .unwrap();
    assert_eq!(code.matches("Keyframe::at(").count(), 2, "{code}");
    // La dernière déclaration d'une propriété à un offset gagne, les autres restent.
    assert!(
        code.contains("Keyframe::at(0.0).opacity(0.25).transform("),
        "{code}"
    );
    assert!(
        code.contains(".opacity(0.25)") && !code.contains(".opacity(0.5)"),
        "{code}"
    );
    let end = &code[code.find("Keyframe::at(1.0)").unwrap()..];
    assert!(
        end.contains(".opacity(1.0)") && end.contains("scale"),
        "{code}"
    );
}

#[test]
fn step_easings_stay_with_the_properties_they_were_declared_with() {
    let code = keyframes(
        "@keyframes k { 0% { opacity: 0; animation-timing-function: linear } 0% { transform: rotate(10deg); animation-timing-function: ease-in } to { opacity: 1; transform: none } }",
    )
    .unwrap();
    // Même offset, deux courbes : deux étapes du moteur.
    assert_eq!(code.matches("Keyframe::at(0.0)").count(), 2, "{code}");
    assert!(code.contains(".opacity(0.0).easing(__ui_root::ui::animation::Easing::Linear)"));
    assert!(code.contains(".easing(__ui_root::ui::animation::Easing::EaseIn)"));
    assert_eq!(code.matches("Keyframe::at(1.0)").count(), 1, "{code}");
}

#[test]
fn colour_properties_are_animatable() {
    let code = keyframes(
        "@keyframes k { from { background-color: #000; color: rgb(255 0 0) } to { background: #fff; border-color: transparent } }",
    )
    .unwrap();
    assert!(code.contains(".background_color(__ui_root::ui::theme::Color::rgba(0,0,0,255))"));
    assert!(code.contains(".color(__ui_root::ui::theme::Color::rgba(255,0,0,255))"));
    assert!(code.contains(".background_color(__ui_root::ui::theme::Color::rgba(255,255,255,255))"));
    assert!(code.contains(".border_color(__ui_root::ui::theme::Color::rgba(0,0,0,0))"));
}

#[test]
fn size_properties_are_animatable() {
    // `eq-bar-1` de RustMusic et la barre de progression d'un toast.
    let code =
        keyframes("@keyframes k { 0%, 100% { height: 4px } 50% { height: 18px; width: 120% } }")
            .unwrap();
    let length = "__ui_root::ui::element::Length";
    assert!(
        code.contains(&format!("Keyframe::at(0.0).height({length}::Px(4.0))")),
        "{code}"
    );
    assert!(
        code.contains(&format!(".height({length}::Px(18.0))")),
        "{code}"
    );
    assert!(
        code.contains(&format!(".width({length}::Percent(120.0))")),
        "{code}"
    );
    let code = keyframes("@keyframes k { from { width: 100% } to { width: 0 } }").unwrap();
    assert!(
        code.contains(&format!(".width({length}::Px(0.0))")),
        "{code}"
    );
    let code = keyframes("@keyframes k { to { height: auto } }").unwrap();
    assert!(code.contains(&format!(".height({length}::Auto)")), "{code}");
}

#[test]
fn refused_keyframes_report_the_offending_declaration() {
    let css = "@keyframes k { 0% { opacity: 0 } 100% { padding: 10px } }";
    let (at, message) = keyframes(css).unwrap_err();
    assert_eq!(at, css.find("padding"), "{message}");
    assert!(
        message.contains("Keyframe property 'padding' is not animatable"),
        "{message}"
    );
    for (css, expected) in [
        ("@keyframes k { 150% { opacity: 0 } }", "outside 0..100%"),
        (
            "@keyframes k { middle { opacity: 0 } }",
            "must be from, to or a percentage",
        ),
        (
            "@keyframes k { to { opacity: 2 } }",
            "Invalid keyframe opacity",
        ),
        (
            "@keyframes k { to { border-color: red blue } }",
            "single colour",
        ),
        (
            "@keyframes k { to { background: linear-gradient(#000, #fff) } }",
            "single colour",
        ),
        (
            "@keyframes k { to { color: currentColor } }",
            "currentColor",
        ),
        ("@keyframes k { to { width: -4px } }", "cannot be negative"),
        ("@keyframes k { to { height: 2em } }", "expects pixels"),
        (
            "@keyframes k { to { opacity: 1; animation-timing-function: steps(4) } }",
            "Unsupported animation timing function",
        ),
    ] {
        let (at, message) = keyframes(css).unwrap_err();
        assert!(message.contains(expected), "{css}: {message}");
        assert!(at.is_some_and(|at| at > 14), "{css}: {at:?}");
    }
}

#[test]
fn animation_lists_compile_to_a_group_and_shorter_lists_repeat() {
    let props = animation(&[("animation", "a 1s, b 2s ease-in infinite reverse")]).unwrap();
    assert_eq!(props["animation-name"], "a, b");
    assert_eq!(props["animation-iteration-count"], "1, infinite");
    assert_eq!(props["animation-direction"], "normal, reverse");
    let code = instance(&props, &sheet()).unwrap();
    assert!(
        code.starts_with("__ui_root::ui::animation::Animation::group(const { &["),
        "{code}"
    );
    let b = &code[code.find("FRAMES_b").unwrap()..];
    assert!(b.contains(".duration(2000)") && b.contains(".forever()") && b.contains(".reverse()"));
    // Longhands : trois noms, deux durées (la première se répète).
    let props = animation(&[
        ("animation-name", "a, b, c"),
        ("animation-duration", "1s, 250ms"),
        ("animation-direction", "alternate-reverse"),
    ])
    .unwrap();
    let code = instance(&props, &sheet()).unwrap();
    let c = &code[code.find("FRAMES_c").unwrap()..];
    assert!(c.starts_with("FRAMES_c }).duration(1000)"), "{code}");
    assert_eq!(code.matches(".alternate().reverse()").count(), 3, "{code}");
    // `none` dans une liste n'anime rien à sa place ; une seule animation reste simple.
    let props = animation(&[("animation", "none, b 1s")]).unwrap();
    let code = instance(&props, &sheet()).unwrap();
    assert!(code.starts_with("__ui_root::ui::animation::Animation::keyframes(const { FRAMES_b"));
    let props = animation(&[("animation", "none")]).unwrap();
    assert!(instance(&props, &sheet()).unwrap().contains(".duration(0)"));
    assert_eq!(names("animation", "a 1s, none, x 2s"), ["a", "x"]);
    assert_eq!(names("animation-name", "b"), ["b"]);
}

#[test]
fn negative_delays_play_state_and_fill_modes_are_compiled() {
    let props = animation(&[(
        "animation",
        "a 1s -250ms linear both paused alternate-reverse 3",
    )])
    .unwrap();
    let code = instance(&props, &sheet()).unwrap();
    for part in [
        ".delay(0)",
        ".advance(250)",
        ".repeat(3)",
        ".alternate().reverse()",
        "FillMode::Both",
        ".paused(true)",
        "Easing::Linear",
    ] {
        assert!(code.contains(part), "{part}: {code}");
    }
    let props = animation(&[("animation", "a 1s 2.0")]).unwrap();
    assert!(instance(&props, &sheet()).unwrap().contains(".repeat(2)"));
    let props = animation(&[("animation", "a 1s"), ("animation-play-state", "running")]).unwrap();
    assert!(!instance(&props, &sheet()).unwrap().contains("paused"));
}

#[test]
fn refused_animation_values_are_explicit() {
    for (name, value, expected) in [
        (
            "animation",
            "a 1s 1.5",
            "fractional counts are not supported",
        ),
        ("animation", "a 1s 0", "Zero animation iterations"),
        ("animation", "a b 1s", "Duplicate animation name"),
        ("animation", "a 1s 2s 3s", "Too many animation times"),
        ("animation", "a -1s", "duration cannot be negative"),
        ("animation", "a 1s,", "Empty item"),
        (
            "animation-direction",
            "backwards",
            "Invalid animation-direction",
        ),
        (
            "animation-play-state",
            "stopped",
            "Invalid animation-play-state",
        ),
        ("animation-fill-mode", "all", "Invalid animation-fill-mode"),
        (
            "animation-timing-function",
            "ease, steps(2)",
            "Unsupported animation timing",
        ),
        (
            "animation-composition",
            "add",
            "Unsupported animation-composition",
        ),
    ] {
        let error = animation(&[(name, value)]).unwrap_err();
        assert!(error.contains(expected), "{name}: {value}: {error}");
    }
    let many = vec!["a 1s"; 17].join(", ");
    assert!(
        animation(&[("animation", &many)])
            .unwrap_err()
            .contains("At most 16")
    );
    let props = animation(&[("animation", "missing 1s")]).unwrap();
    assert!(
        instance(&props, &sheet())
            .unwrap_err()
            .contains("Unknown @keyframes 'missing'")
    );
}

#[test]
fn state_rules_override_timing_longhands_but_not_the_keyframes() {
    use super::state_override;
    assert_eq!(
        state_override("animation-delay", "-1.4s").unwrap().1,
        "delay:Some(-1400),"
    );
    assert_eq!(
        state_override("animation-direction", "alternate-reverse")
            .unwrap()
            .1,
        "direction:Some((true,true)),"
    );
    assert_eq!(
        state_override("animation-play-state", "paused").unwrap().1,
        "paused:Some(true),"
    );
    assert_eq!(state_override("animation", "none").unwrap().1, "none:true,");
    for (name, value) in [
        ("animation-name", "spin"),
        ("animation", "spin 1s"),
        ("animation-delay", "1s, 2s"),
        ("animation-direction", "sideways"),
    ] {
        assert!(state_override(name, value).is_err(), "{name}: {value}");
    }
}
#[test]
fn an_animation_time_beyond_u32_is_refused() {
    // 2^32 ms passait la borne et devenait un littéral `u32` hors plage.
    assert_eq!(timing::time("4294967040ms"), Ok(4294967040));
    assert!(timing::time("4294967296ms").is_err());
}
