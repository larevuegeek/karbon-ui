//! Tests de l'évaluation des déclarations dynamiques.
use super::{DynamicDeclaration, DynamicProperty, Evaluation, Token, Unit, evaluate};
use crate::ui::{
    element::Insets,
    styling::{CssVariables, Properties, VariableValue},
    theme::Color,
};

fn eval_with(vars: &CssVariables) -> Evaluation<'_> {
    Evaluation {
        variables: Some(vars),
        globals: None,
        font: 16.,
        parent_font: 16.,
        root_font: 16.,
        viewport: (1000., 800.),
        scale: 1.,
        current: Color::hex(0x112233),
        parent_corners: [0.; 4],
    }
}
#[test]
fn composed_transitions_keep_the_first_animatable_timing() {
    use crate::ui::animation::{Easing, Transition};
    // `transition: var(--t)`, `--t` = liste de Bootstrap (`--bs-accordion-transition`).
    static LIST: [Token; 14] = [
        Token::Ident("box-shadow"),
        Token::Dimension(0.15, Unit::S),
        Token::Ident("ease-in-out"),
        Token::Comma,
        Token::Ident("color"),
        Token::Dimension(150., Unit::Ms),
        Token::Ident("ease-in-out"),
        Token::Dimension(0.05, Unit::S),
        Token::Comma,
        Token::Dimension(0.2, Unit::S),
        Token::Function("cubic-bezier"),
        Token::Number(0.1),
        Token::Close,
        Token::Ident("transform"),
    ];
    let mut vars = CssVariables::default();
    vars.set("--t", VariableValue::Static(&LIST));
    static VALUE: [Token; 2] = [Token::Var("--t"), Token::Close];
    let mut out = Properties::default();
    evaluate(
        &[DynamicDeclaration {
            property: DynamicProperty::Transition,
            value: &VALUE,
        }],
        &eval_with(&vars),
        &mut out,
    );
    // `cubic-bezier(0.1)` invalide : toute la valeur l'est, la propriété reste vide.
    assert_eq!(out.transition, None);
    static FIXED: [Token; 10] = [
        Token::Ident("box-shadow"),
        Token::Dimension(0.15, Unit::S),
        Token::Comma,
        Token::Ident("color"),
        Token::Dimension(150., Unit::Ms),
        Token::Ident("ease-in-out"),
        Token::Dimension(0.05, Unit::S),
        Token::Comma,
        Token::Dimension(0.2, Unit::S),
        Token::Ident("transform"),
    ];
    vars.set("--t", VariableValue::Static(&FIXED));
    evaluate(
        &[DynamicDeclaration {
            property: DynamicProperty::Transition,
            value: &VALUE,
        }],
        &eval_with(&vars),
        &mut out,
    );
    let expected = Transition::new()
        .duration(150)
        .delay(50)
        .easing(Easing::EaseInOut)
        .properties(true, true);
    assert_eq!(out.transition, Some(expected));
    assert!(!expected.animates_sizes());
}
#[test]
fn bootstrap_rgba_with_variables() {
    static RGB: [Token; 5] = [
        Token::Number(13.),
        Token::Comma,
        Token::Number(110.),
        Token::Comma,
        Token::Number(253.),
    ];
    let mut vars = CssVariables::default();
    vars.set("--bs-primary-rgb", VariableValue::Static(&RGB));
    vars.set_number("--bs-text-opacity", 0.5);
    static VALUE: [Token; 7] = [
        Token::Function("rgba"),
        Token::Var("--bs-primary-rgb"),
        Token::Close,
        Token::Comma,
        Token::Var("--bs-text-opacity"),
        Token::Close,
        Token::Close,
    ];
    let mut out = Properties::default();
    evaluate(
        &[DynamicDeclaration {
            property: DynamicProperty::Color,
            value: &VALUE,
        }],
        &eval_with(&vars),
        &mut out,
    );
    assert_eq!(out.text.color, Some(Color::rgba(13, 110, 253, 128)));
}
#[test]
fn padding_calc_and_fallbacks() {
    let vars = CssVariables::default();
    static VALUE: [Token; 11] = [
        Token::Var("--missing"),
        Token::Comma,
        Token::Dimension(0.5, Unit::Rem),
        Token::Close,
        Token::Function("calc"),
        Token::Dimension(1., Unit::Em),
        Token::Delim('+'),
        Token::Dimension(2., Unit::Px),
        Token::Delim('*'),
        Token::Number(2.),
        Token::Close,
    ];
    let mut out = Properties::default();
    evaluate(
        &[DynamicDeclaration {
            property: DynamicProperty::Padding,
            value: &VALUE,
        }],
        &eval_with(&vars),
        &mut out,
    );
    assert_eq!(out.padding, Some(Insets::edges(8., 20., 8., 20.)));
}
#[test]
fn tailwind_shadow_layers() {
    let vars = CssVariables::default();
    static VALUE: [Token; 13] = [
        Token::Number(0.),
        Token::Number(0.),
        Token::Color(Color::rgba(0, 0, 0, 0)),
        Token::Comma,
        Token::Number(0.),
        Token::Number(0.),
        Token::Number(0.),
        Token::Dimension(2., Unit::Px),
        Token::Color(Color::hex(0x3b82f6)),
        Token::Comma,
        Token::Ident("inset"),
        Token::Number(0.),
        Token::Number(0.),
    ];
    let mut out = Properties::default();
    evaluate(
        &[DynamicDeclaration {
            property: DynamicProperty::BoxShadow,
            value: &VALUE[..9],
        }],
        &eval_with(&vars),
        &mut out,
    );
    let s = out.shadows.unwrap();
    assert_eq!(s.values.len(), 1);
    assert_eq!(s.values[0].spread, 2.);
}
/// L10 de la revue : les listes d'ombres calculées étaient internées à vie, et au-delà de
/// 4 096 listes différentes l'ombre n'était plus appliquée. Elles sont maintenant
/// partagées et libérées avec leur dernier utilisateur.
#[test]
fn computed_shadow_lists_are_never_capped_and_are_freed() {
    use crate::ui::rendering::component_shadow::ShadowList;
    let vars = CssVariables::default();
    // `0 0 4px` sans couleur : celle du texte, différente à chaque tour.
    static VALUE: [Token; 3] = [
        Token::Number(0.),
        Token::Number(0.),
        Token::Dimension(4., Unit::Px),
    ];
    for i in 0..5_000u32 {
        let mut eval = eval_with(&vars);
        eval.current = Color::rgb((i % 256) as u8, (i / 256) as u8, 7);
        let mut out = Properties::default();
        evaluate(
            &[DynamicDeclaration {
                property: DynamicProperty::BoxShadow,
                value: &VALUE,
            }],
            &eval,
            &mut out,
        );
        let shadows = out
            .shadows
            .expect("ombre appliquée, même après 4 096 listes");
        assert_eq!(shadows.values[0].color, eval.current);
        let ShadowList::Shared(list) = &shadows.values else {
            panic!("liste calculée partagée");
        };
        assert_eq!(
            std::sync::Arc::strong_count(list),
            1,
            "libérée avec l'élément"
        );
    }
}
