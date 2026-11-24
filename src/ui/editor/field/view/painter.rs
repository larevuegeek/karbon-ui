//! La boîte d'un champ numérique : fond, part de la plage, couleur de l'axe, lettre,
//! valeur et anneau du clavier.
use super::AXES;
use crate::ui::{
    editor::EditorPalette,
    element::{PaintContext, Role, Semantics, SizeF, TextMeasurer, Widget},
    scene::{RectF, Scene},
    theme::Color,
};

/// La boîte d'un champ : fond, part de la plage, couleur de l'axe, lettre et valeur.
pub(super) struct FieldPainter {
    pub(super) text: String,
    pub(super) fraction: Option<f32>,
    pub(super) axis: Option<usize>,
    pub(super) dragging: bool,
    pub(super) palette: EditorPalette,
    pub(super) font: Option<crate::ui::font::Font>,
    pub(super) tagged: bool,
    pub(super) tab_stop: bool,
}

impl Widget for FieldPainter {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn focusable(&self) -> bool {
        self.tab_stop
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Slider,
            name: "",
            enabled: true,
        }
    }

    fn accessible_value(&self) -> Option<String> {
        Some(self.text.clone())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        if self.tagged || self.font.is_some() {
            self.paint_tagged(context, scene);
            return paint_focus(context, &self.palette, scene);
        }
        let r = context.rect;
        let p = &self.palette;
        let width = (r.right - r.left).max(0.);
        let height = (r.bottom - r.top).max(0.);
        let active = self.dragging || context.state.hovered;
        scene.fill(r, 3., if active { p.hover } else { p.field });
        if let Some(fraction) = self.fraction {
            let filled = RectF::new(r.left, r.top, width * fraction.clamp(0., 1.), height);
            scene.fill(filled, 3., with_alpha(p.accent, 0x48));
        }
        let mut left = r.left + 6.;
        let text_top = r.top + ((height - p.font_size * 1.35) / 2.).max(0.);
        if let Some(axis) = self.axis {
            let color = p.axes[axis];
            scene.fill(RectF::new(r.left, r.top, 3., height), 1.5, color);
            scene.text_single_line(
                RectF::new(left, text_top, 12., p.font_size * 1.4),
                AXES[axis],
                p.font_size,
                true,
                color,
            );
            left += 14.;
        }
        scene.text_single_line(
            RectF::new(
                left,
                text_top,
                (r.right - left - 4.).max(0.),
                p.font_size * 1.4,
            ),
            self.text.clone(),
            p.font_size,
            false,
            p.text,
        );
        if self.dragging {
            scene.commands.push(crate::ui::scene::DrawCommand::Stroke {
                rect: r,
                radius: 3.,
                width: 1.,
                color: p.accent,
            });
        }
        paint_focus(context, p, scene);
    }
}

/// Anneau du champ qui a le clavier (atteint par Tab).
fn paint_focus(context: &PaintContext, palette: &EditorPalette, scene: &mut Scene) {
    if context.state.focused {
        scene.commands.push(crate::ui::scene::DrawCommand::Stroke {
            rect: context.rect,
            radius: 3.,
            width: 2.,
            color: palette.accent,
        });
    }
}

pub(super) fn with_alpha(color: Color, alpha: u8) -> Color {
    Color { alpha, ..color }
}
