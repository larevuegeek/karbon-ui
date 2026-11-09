//! Visual effects: liquid glass panels and animated light backgrounds.
use super::{
    css,
    space::Space,
    theme::palette,
    tone::{CustomColor, translucent},
};
use crate::ui::{
    element::Element,
    rendering::brush::{GradientStop, RadialGradient},
    styling::color_scheme,
    template::{Component, Slots, TemplateContext},
    theme::Color,
};

/// Liquid glass panel: blurs and saturates what lies behind it, with a light edge and a
/// reflection. Place it over an image, a gradient or an [`Aurora`].
///
/// ```ignore
/// <Aurora>
///     <Glass padding="lg"><Title h3>Now playing</Title></Glass>
/// </Aurora>
/// ```
#[derive(Default)]
pub struct Glass {
    /// Inner margin.
    pub padding: Space,
    /// Distance between children.
    pub gap: Space,
    /// Thicker glass: more blur and more opacity.
    pub strong: bool,
    /// Fully rounded ends.
    pub pill: bool,
    /// Colour of the glass.
    pub tint: CustomColor,
}
impl<R> Component<R> for Glass {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let (padding, gap) = (self.padding.or(16.), self.gap.or(10.));
        if let Some(color) = self.tint.0 {
            let tint = translucent(color, 0.22);
            return kit_ui!(cx, slots;
                <div class="k-glass" class:k-glass-strong={self.strong} class:k-glass-pill={self.pill} style:background={tint}>
                    <div class="k-flex k-glass-body" style:padding={padding} style:gap={gap}><slot /></div>
                </div>
            );
        }
        kit_ui!(cx, slots;
            <div class="k-glass" class:k-glass-strong={self.strong} class:k-glass-pill={self.pill}>
                <div class="k-flex k-glass-body" style:padding={padding} style:gap={gap}><slot /></div>
            </div>
        )
    }
}

/// Halo de `color` qui s'efface vers son propre transparent.
fn halo(color: Color) -> RadialGradient {
    RadialGradient::new(
        [0.5, 0.5],
        [0.5, 0.5],
        [
            GradientStop {
                position: 0.,
                color: translucent(color, if color_scheme().0 { 0.34 } else { 0.2 }),
            },
            GradientStop {
                position: 1.,
                color: translucent(color, 0.),
            },
        ],
    )
}

/// Soft coloured light, drawn behind its children.
#[derive(Default)]
pub struct Aurora {
    /// Takes the whole space of its parent.
    pub full: bool,
    /// Square corners.
    pub flat: bool,
    /// The light drifts slowly, redrawing the window continuously.
    pub animated: bool,
}
impl<R> Component<R> for Aurora {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let p = palette();
        kit_ui!(cx, slots;
            <div class="k-aurora" class:k-full-size={self.full} class:k-aurora-flat={self.flat}>
                <div class="k-blob k-blob-1" class:k-drift-1={self.animated} style:background={halo(p.primary)}></div>
                <div class="k-blob k-blob-2" class:k-drift-2={self.animated} style:background={halo(p.accent)}></div>
                <div class="k-aurora-body"><slot /></div>
            </div>
        )
    }
}
