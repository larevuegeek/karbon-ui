//! Couleurs d'un composant selon sa teinte (`primary`, `danger`…).
use super::theme::palette;
use crate::ui::template::IntoProp;
use crate::ui::theme::Color;

/// Colour family of a component, for values chosen at run time
/// (`variant={Variant::Danger}`); in templates, `primary`, `danger`… are shorter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Neutral colours.
    #[default]
    Neutral,
    /// Brand colour.
    Primary,
    /// Positive state.
    Success,
    /// Attention state.
    Warning,
    /// Destructive state.
    Danger,
    /// Informative state.
    Info,
}

/// One-off colour of a component: `color="#e11d48"` or `color={Color::hex(0xe11d48)}`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CustomColor(pub(super) Option<Color>);

impl IntoProp<CustomColor> for Color {
    fn into_prop(self) -> CustomColor {
        CustomColor(Some(self))
    }
}
impl IntoProp<CustomColor> for &str {
    fn into_prop(self) -> CustomColor {
        let digits = self.trim_start_matches('#');
        CustomColor(match (digits.len(), u32::from_str_radix(digits, 16)) {
            (6, Ok(rgb)) => Some(Color::hex(rgb)),
            _ => None,
        })
    }
}

/// Couleurs résolues : remplissage, texte sur le remplissage, texte sur fond transparent,
/// trait du contour.
#[derive(Clone, Copy)]
pub(super) struct Tone {
    pub fill: Color,
    pub on: Color,
    pub fg: Color,
    pub line: Color,
}
/// `color` à l'opacité `alpha` (0 à 1).
pub(super) fn translucent(color: Color, alpha: f32) -> Color {
    Color::rgba(
        color.red,
        color.green,
        color.blue,
        (alpha.clamp(0., 1.) * 255.) as u8,
    )
}
/// Texte lisible sur `fill` : noir ou blanc selon sa luminance.
pub(super) fn on(fill: Color) -> Color {
    let channel = |v: u8| {
        let v = f32::from(v) / 255.;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * channel(fill.red) + 0.7152 * channel(fill.green) + 0.0722 * channel(fill.blue);
    if luminance > 0.40 {
        Color::hex(0x18181b)
    } else {
        Color::hex(0xffffff)
    }
}
pub(super) fn tone(variant: Variant, custom: CustomColor) -> Tone {
    let p = palette();
    let fill = custom.0.or(match variant {
        Variant::Neutral => None,
        Variant::Primary => Some(p.primary),
        Variant::Success => Some(p.success),
        Variant::Warning => Some(p.warning),
        Variant::Danger => Some(p.danger),
        Variant::Info => Some(p.info),
    });
    match fill {
        Some(fill) => Tone {
            fill,
            on: on(fill),
            fg: fill,
            line: fill,
        },
        None => Tone {
            fill: p.border,
            on: p.text,
            fg: p.text,
            line: p.border,
        },
    }
}
/// Teinte désignée par les attributs courts d'un composant.
pub(super) fn pick(
    variant: Variant,
    [primary, success, warning, danger, info]: [bool; 5],
) -> Variant {
    if primary {
        Variant::Primary
    } else if success {
        Variant::Success
    } else if warning {
        Variant::Warning
    } else if danger {
        Variant::Danger
    } else if info {
        Variant::Info
    } else {
        variant
    }
}
