//! La boîte d'un champ réglé par `NumberField::tagged` ou `NumberField::font` (ajout
//! 0.3.25) : la lettre de l'axe sur un onglet teinté, la valeur dans la police donnée.
//! Les champs sans ces réglages sont peints comme avant par `FieldPainter::paint`.
use super::{AXES, FieldPainter, with_alpha};
use crate::ui::{
    element::PaintContext,
    scene::{DrawCommand, RectF, Scene},
};

/// Largeur de l'onglet de la lettre d'un axe (`NumberField::tagged`).
const TAG_WIDTH: f32 = 18.;

impl FieldPainter {
    /// La boîte d'un champ réglé par `tagged` ou `font` : un fond aux coins de 6 et un
    /// filet ; la lettre de l'axe sur un onglet teinté de sa couleur (`tagged`), sinon le
    /// trait de l'axe ; la valeur dans la police donnée.
    pub(super) fn paint_tagged(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let p = &self.palette;
        let height = (r.bottom - r.top).max(0.);
        let active = self.dragging || context.state.hovered;
        scene.fill(r, 6., if active { p.hover } else { p.field });
        if let Some(fraction) = self.fraction {
            let width = (r.right - r.left).max(0.) * fraction.clamp(0., 1.);
            scene.fill(
                RectF::new(r.left, r.top, width, height),
                6.,
                with_alpha(p.accent, 0x48),
            );
        }
        let text_top = r.top + ((height - p.font_size * 1.35) / 2.).max(0.);
        let mut left = r.left + 6.;
        match (self.axis, self.tagged) {
            (Some(axis), true) => {
                let color = p.axes[axis];
                let tab = RectF::new(r.left, r.top, TAG_WIDTH, height);
                scene.commands.push(DrawCommand::PushShapeClip {
                    rect: r,
                    radii: [6.; 4],
                });
                scene.fill(tab, 0., with_alpha(color, 0x1F));
                scene.fill(
                    RectF::new(r.left + TAG_WIDTH - 1., r.top, 1., height),
                    0.,
                    with_alpha(color, 0x59),
                );
                scene.commands.push(DrawCommand::PopShapeClip);
                let letter_top = r.top + ((height - 10.5 * 1.35) / 2.).max(0.);
                scene.text_single_line(
                    RectF::new(r.left + 5., letter_top, TAG_WIDTH - 5., 10.5 * 1.4),
                    AXES[axis],
                    10.5,
                    true,
                    color,
                );
                left = r.left + TAG_WIDTH + 5.;
            }
            (Some(axis), false) => {
                scene.fill(RectF::new(r.left, r.top, 3., height), 1.5, p.axes[axis]);
                left += 6.;
            }
            _ => {}
        }
        let place = RectF::new(
            left,
            text_top,
            (r.right - left - 4.).max(0.),
            p.font_size * 1.4,
        );
        match self.font {
            Some(font) => scene.font_text(place, self.text.clone(), p.font_size, font, p.text),
            None => scene.text_single_line(place, self.text.clone(), p.font_size, false, p.text),
        }
        let edge = if self.dragging { p.accent } else { p.border };
        scene.commands.push(DrawCommand::Stroke {
            rect: r,
            radius: 6.,
            width: 1.,
            color: edge,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::{
        editor::{EditorPalette, FieldMsg, FieldState, VectorField, testing::window},
        element::Element,
        font::Font,
        prelude::{Application, ViewContext, column},
        scene::DrawCommand,
    };

    /// Deux vecteurs : l'un sans réglage, l'autre à virgule, en chasse fixe, à onglets.
    struct Place {
        fields: FieldState,
    }

    impl Application for Place {
        type Message = FieldMsg;
        fn view(&self, _: &ViewContext) -> Element {
            let plain = VectorField::new(1, [1.5, 0., 0.], &self.fields, |m| m)
                .step(0.01)
                .element();
            let tagged = VectorField::new(2, [2.25, 0., 0.], &self.fields, |m| m)
                .step(0.01)
                .font(Font::new("Consolas", 500))
                .decimal_comma(true)
                .tagged(true)
                .element();
            column("page", [plain, tagged]).gap(4.).padding(10.)
        }
        fn update(&mut self, _: FieldMsg) -> bool {
            false
        }
    }

    #[test]
    fn a_tagged_field_shows_its_letter_on_a_tab_and_its_value_with_a_comma_in_its_font() {
        let app = Place {
            fields: FieldState::default(),
        };
        let window = window(app, 400., 120.);
        let commands = &window.scene().commands;
        // Sans réglage : comme avant, le texte du système et le point.
        assert!(commands.iter().any(
            |c| matches!(c, DrawCommand::Text { text, .. } if &**text == "1.50")
        ));
        // Réglé : la virgule, dans la police donnée.
        assert!(commands.iter().any(|c| matches!(c,
            DrawCommand::FontText { text, font, .. }
                if &**text == "2,25" && font.family == "Consolas" && font.weight == 500)));
        assert!(!commands.iter().any(
            |c| matches!(c, DrawCommand::Text { text, .. } if &**text == "2.25")
        ));
        // L'onglet de la lettre X, teinté de la couleur de l'axe.
        let tint = crate::ui::theme::Color {
            alpha: 0x1F,
            ..EditorPalette::DARK.axes[0]
        };
        assert!(commands.iter().any(
            |c| matches!(c, DrawCommand::Fill { color, rect, .. }
                if *color == tint && (rect.right - rect.left - super::TAG_WIDTH).abs() < 0.01)
        ));
    }
}
