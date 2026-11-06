//! Évaluation des déclarations dynamiques : substitution des `var()` puis application de
//! la valeur à la propriété, répartie par famille de propriétés.
mod border;
mod control;
mod effect;
mod layout;
mod paint;
mod text;

use super::{
    property::{DynamicDeclaration, DynamicProperty},
    reader::{Evaluation, Reader},
    substitution::{lookup, substitute},
    token::Token,
};
use crate::ui::styling::{ControlPart, Properties, StateEffect};

thread_local! {
    static EVALUATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
/// Number of dynamic declarations evaluated on this thread since it started; used to
/// measure the run-time cost of framework stylesheets.
#[doc(hidden)]
pub fn evaluation_count() -> u64 {
    EVALUATIONS.with(std::cell::Cell::get)
}
/// Évalue `declarations` dans l'ordre ; les valeurs invalides au moment du calcul
/// laissent la propriété inchangée.
pub(crate) fn evaluate(
    declarations: &[DynamicDeclaration],
    eval: &Evaluation,
    out: &mut Properties,
) {
    EVALUATIONS.with(|c| c.set(c.get() + declarations.len() as u64));
    let mut buffer = Vec::new();
    for d in declarations {
        buffer.clear();
        let mut look =
            |n, o: &mut Vec<Token>, depth| lookup(eval.variables, eval.globals, n, o, depth);
        if !substitute(d.value, &mut look, &mut buffer, 0) || buffer.is_empty() {
            continue;
        }
        apply(d.property, &buffer, eval, out);
    }
}
/// Applique la valeur substituée `tokens` à `property`, selon sa famille.
fn apply(property: DynamicProperty, tokens: &[Token], eval: &Evaluation, out: &mut Properties) {
    use DynamicProperty as P;
    let font = if property == P::FontSize {
        eval.parent_font
    } else {
        eval.font
    };
    let mut r = Reader {
        tokens,
        at: 0,
        eval,
        font,
    };
    let r = &mut r;
    match property {
        P::Color
        | P::BackgroundColor
        | P::Background
        | P::BackgroundImage
        | P::BackgroundPosition
        | P::BackgroundSize
        | P::BackgroundRepeat
        | P::Appearance
        | P::AccentColor
        | P::OutlineColor
        | P::Outline
        | P::OutlineWidth
        | P::OutlineOffset
        | P::BoxShadow => paint::apply(property, tokens, eval, font, r, out),
        P::ThumbBackground
        | P::ThumbWidth
        | P::ThumbHeight
        | P::ThumbRadius
        | P::ThumbBorder
        | P::ThumbShadow
        | P::TrackBackground
        | P::TrackHeight
        | P::TrackRadius
        | P::TrackBorder => control::apply(property, tokens, eval, font, r, out),
        P::BorderColor
        | P::BorderTopColor
        | P::BorderRightColor
        | P::BorderBottomColor
        | P::BorderLeftColor
        | P::Border
        | P::BorderTop
        | P::BorderRight
        | P::BorderBottom
        | P::BorderLeft
        | P::BorderWidth
        | P::BorderTopWidth
        | P::BorderRightWidth
        | P::BorderBottomWidth
        | P::BorderLeftWidth
        | P::BorderRadius
        | P::BorderTopLeftRadius
        | P::BorderTopRightRadius
        | P::BorderBottomRightRadius
        | P::BorderBottomLeftRadius => border::apply(property, tokens, eval, font, r, out),
        P::Padding
        | P::PaddingTop
        | P::PaddingRight
        | P::PaddingBottom
        | P::PaddingLeft
        | P::Margin
        | P::MarginTop
        | P::MarginRight
        | P::MarginBottom
        | P::MarginLeft
        | P::Width
        | P::Height
        | P::MinWidth
        | P::MinHeight
        | P::MaxWidth
        | P::MaxHeight
        | P::FlexBasis
        | P::FlexGrow
        | P::FlexShrink
        | P::Gap
        | P::RowGap
        | P::ColumnGap => layout::apply(property, r, out),
        P::FontSize | P::FontWeight | P::LineHeight | P::LetterSpacing => {
            text::apply(property, r, out)
        }
        P::Opacity | P::Transform | P::Translate | P::Scale | P::Rotate | P::Transition => {
            effect::apply(property, tokens, eval, font, r, out)
        }
    }
}
/// Pièce de contrôle visée par une propriété de pseudo-élément de curseur.
fn part(out: &mut Properties, property: DynamicProperty) -> &mut ControlPart {
    use DynamicProperty as P;
    match property {
        P::TrackBackground | P::TrackHeight | P::TrackRadius | P::TrackBorder => {
            &mut out.slider_track
        }
        _ => &mut out.slider_thumb,
    }
}
fn side(property: DynamicProperty) -> usize {
    use DynamicProperty as P;
    match property {
        P::BorderTopColor | P::BorderTop | P::BorderTopWidth | P::PaddingTop | P::MarginTop => 0,
        P::BorderRightColor
        | P::BorderRight
        | P::BorderRightWidth
        | P::PaddingRight
        | P::MarginRight => 1,
        P::BorderBottomColor
        | P::BorderBottom
        | P::BorderBottomWidth
        | P::PaddingBottom
        | P::MarginBottom => 2,
        _ => 3,
    }
}
fn effect(out: &mut Properties, f: impl FnOnce(&mut StateEffect)) {
    let mut e = out.effect.unwrap_or(StateEffect {
        effect: crate::ui::animation::Effect::NONE,
        opacity: false,
        transform: false,
    });
    f(&mut e);
    out.effect = Some(e);
}
