//! Liste composée de `transition` : minutage gardé par le moteur.
use super::token::{Token, Unit, group_end, layers};

/// `transition` : liste composée `[propriété] [durée] [courbe] [délai]`. Le moteur garde
/// un minutage par élément, celui du premier élément de la liste qu'il sait animer ; les
/// propriétés qu'il n'anime pas (`box-shadow`, `border-radius`…) sont ignorées. `None`
/// quand la valeur est invalide.
pub(super) fn transition(tokens: &[Token]) -> Option<crate::ui::animation::Transition> {
    use crate::ui::animation::{Easing, Transition};
    if tokens == [Token::Ident("none")] {
        return Some(Transition::new().duration(0).properties(false, false));
    }
    let (mut effects, mut colors, mut sizes) = (false, false, false);
    let mut timing = None;
    for item in layers(tokens) {
        let mut property = "all";
        let mut times = Vec::new();
        let mut easing = Easing::Ease;
        let mut at = 0;
        while at < item.len() {
            match item[at] {
                Token::Ident(word) => {
                    easing = match word {
                        "linear" => Easing::Linear,
                        "ease" => Easing::Ease,
                        "ease-in" => Easing::EaseIn,
                        "ease-out" => Easing::EaseOut,
                        "ease-in-out" => Easing::EaseInOut,
                        "step-start" | "step-end" => Easing::Linear,
                        _ => {
                            property = word;
                            easing
                        }
                    }
                }
                Token::Dimension(n, Unit::Ms) => times.push(n),
                Token::Dimension(n, Unit::S) => times.push(n * 1000.),
                Token::Number(0.) => times.push(0.),
                Token::Function(name) => {
                    let end = group_end(item, at + 1);
                    let args = item[at + 1..end.min(item.len())]
                        .iter()
                        .filter_map(|t| match t {
                            Token::Number(n) => Some(*n),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    easing = match (name, args.as_slice()) {
                        ("cubic-bezier", &[x1, y1, x2, y2])
                            if (0. ..=1.).contains(&x1) && (0. ..=1.).contains(&x2) =>
                        {
                            Easing::CubicBezier(x1, y1, x2, y2)
                        }
                        // Paliers : approchés par une progression linéaire.
                        ("steps", _) | ("linear", _) => Easing::Linear,
                        _ => return None,
                    };
                    at = end;
                }
                _ => return None,
            }
            at += 1;
        }
        if times.len() > 2 || times.iter().any(|t| !t.is_finite() || *t < 0.) {
            return None;
        }
        let supported = match property {
            "all" => {
                (effects, colors, sizes) = (true, true, true);
                true
            }
            "opacity" | "transform" | "translate" | "rotate" | "scale" => {
                effects = true;
                true
            }
            "color"
            | "background-color"
            | "background"
            | "border-color"
            | "border-top-color"
            | "border-right-color"
            | "border-bottom-color"
            | "border-left-color" => {
                colors = true;
                true
            }
            "width" | "height" => {
                sizes = true;
                true
            }
            _ => false,
        };
        if supported && timing.is_none() {
            timing = Some(
                Transition::new()
                    .duration(times.first().copied().unwrap_or(0.).round() as u32)
                    .delay(times.get(1).copied().unwrap_or(0.).round() as u32)
                    .easing(easing),
            );
        }
    }
    Some(
        timing
            .unwrap_or(Transition::new().duration(0))
            .properties(effects, colors)
            .sizes(sizes),
    )
}
