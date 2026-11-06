//! Tests des couleurs évaluées pendant la résolution des styles.
use super::{ColorExpression, MixSpace};
use crate::ui::{layout::Expression, styling::CssVariables, theme::Color};

#[test]
fn color_functions_match_reference_conversions() {
    let e = |c: ColorExpression| c.evaluate(Color::hex(0x123456), None);
    // oklch(62.8% 0.2577 29.23) ≈ #ff0000 (CSS Color 4).
    assert_eq!(
        e(ColorExpression::Oklch([
            &Expression::Number(0.628),
            &Expression::Number(0.2577),
            &Expression::Number(29.23),
            &Expression::Number(1.)
        ])),
        Color::hex(0xff0000)
    );
    assert_eq!(
        e(ColorExpression::Hsl([
            &Expression::Number(120.),
            &Expression::Number(1.),
            &Expression::Number(0.25),
            &Expression::Number(0.5)
        ])),
        Color::rgba(0, 128, 0, 128)
    );
    assert_eq!(e(ColorExpression::CurrentColor), Color::hex(0x123456));
    const WHITE: ColorExpression = ColorExpression::Constant(Color::hex(0xffffff));
    const BLACK: ColorExpression = ColorExpression::Constant(Color::hex(0x000000));
    let mixed = e(ColorExpression::Mix {
        space: MixSpace::Srgb,
        first: &WHITE,
        second: &BLACK,
        weight: &Expression::Number(0.25),
        alpha: &Expression::Number(1.),
    });
    assert_eq!(mixed, Color::hex(0x404040));
    let mut vars = CssVariables::default();
    vars.set_number("--h", 29.23);
    const HUE: Expression = Expression::Variable {
        name: "--h",
        fallback: &Expression::Number(250.),
        length: false,
    };
    let red = ColorExpression::Oklch([
        &Expression::Number(0.628),
        &Expression::Number(0.2577),
        &HUE,
        &Expression::Number(1.),
    ]);
    assert_eq!(
        red.evaluate(Color::hex(0), Some(&vars)),
        Color::hex(0xff0000)
    );
    assert_ne!(red.evaluate(Color::hex(0), None), Color::hex(0xff0000));
    assert!(red.valid());
}
