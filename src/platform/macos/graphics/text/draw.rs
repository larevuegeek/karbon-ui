//! Dessin d'une mise en page : glyphes run par run (couleur de chaque portion), puis
//! soulignements, barrés et surlignes dessinés par le moteur.
use super::super::super::ffi::{cg_color, rect as cg_rect};
use super::layout::{Layout, Line};
use crate::ui::{scene::RectF, theme::Color};
use objc2_core_foundation::{CFArray, CGAffineTransform, CGFloat};
use objc2_core_graphics::{CGContext, CGTextDrawingMode};
use objc2_core_text::CTRun;

/// Matrice de texte d'un repère dont l'axe y descend.
const FLIPPED: CGAffineTransform = CGAffineTransform {
    a: 1.,
    b: 0.,
    c: 0.,
    d: -1.,
    tx: 0.,
    ty: 0.,
};
impl Layout {
    /// Dessine la mise en page dans `rect` (repère logique, y vers le bas). `color` colore
    /// le texte sans couleur propre ; `ink` remplace toutes les couleurs (masques).
    pub fn draw(&self, context: &CGContext, rect: RectF, color: Color, ink: Option<Color>) {
        let c = Some(context);
        let format = self.paragraph.format;
        CGContext::save_g_state(c);
        if format.clip {
            CGContext::clip_to_rect(c, cg_rect(rect));
        }
        CGContext::set_text_matrix(c, FLIPPED);
        CGContext::set_text_drawing_mode(c, CGTextDrawingMode::Fill);
        let colour = |index: usize| -> Color {
            ink.unwrap_or_else(|| {
                self.bytes
                    .get(index)
                    .and_then(|byte| self.paragraph.style_at(*byte).color)
                    .unwrap_or(color)
            })
        };
        let mut current = None;
        for line in &self.lines {
            // SAFETY : ligne valide ; tableau de CTRun d'après Core Text.
            let runs = unsafe { line.line.glyph_runs() };
            // SAFETY : idem.
            let runs: &CFArray<CTRun> = unsafe { runs.cast_unchecked() };
            for run in runs {
                // SAFETY : run valide.
                let start = unsafe { run.string_range() }.location.max(0) as usize;
                let run_color = colour(start);
                if current != Some(run_color) {
                    CGContext::set_fill_color_with_color(c, Some(&cg_color(run_color)));
                    current = Some(run_color);
                }
                CGContext::set_text_position(
                    c,
                    CGFloat::from(rect.left + line.x),
                    CGFloat::from(rect.top + line.baseline),
                );
                // SAFETY : contexte et run valides ; intervalle vide = tout le run.
                unsafe { run.draw(context, super::super::super::ffi::range(0, 0)) };
            }
        }
        self.decorations(context, rect, color, ink);
        CGContext::restore_g_state(c);
    }
    fn decorations(&self, context: &CGContext, rect: RectF, color: Color, ink: Option<Color>) {
        for (r, c) in self.decoration_rects(rect, color, ink) {
            CGContext::set_fill_color_with_color(Some(context), Some(&cg_color(c)));
            CGContext::fill_rect(Some(context), cg_rect(r));
        }
    }
    /// Soulignements, barrés et surlignes de la mise en page posée dans `rect`, dans
    /// l'ordre du dessin (partagés avec le renderer Metal).
    pub fn decoration_rects(
        &self,
        rect: RectF,
        color: Color,
        ink: Option<Color>,
    ) -> Vec<(RectF, Color)> {
        let format = self.paragraph.format;
        let size = format.size;
        // SAFETY : police valide.
        let (position, thickness, x_height) = unsafe {
            (
                self.base.underline_position() as f32,
                (self.base.underline_thickness() as f32).max(1.),
                self.base.x_height() as f32,
            )
        };
        let style = format.decoration_style;
        let custom = style.custom();
        let mut result = Vec::new();
        let mut fill = |r: RectF, c: Color| result.push((r, c));
        for line in &self.lines {
            for segment in &self.segments {
                let d = segment.style.decoration;
                let (underline, strike, overline) = (
                    format.decoration.underline || d.underline,
                    format.decoration.line_through || d.line_through,
                    format.decoration.overline || d.overline,
                );
                let (a, b) = (segment.start.max(line.start), segment.end.min(line.end));
                if !(underline || strike || overline) || a >= b {
                    continue;
                }
                let Some((left, right)) = span(line, a, b) else {
                    continue;
                };
                let (x, w) = (rect.left + line.x + left, right - left);
                let base = rect.top + line.baseline;
                let ink_color = ink.unwrap_or(segment.style.color.unwrap_or(color));
                if underline {
                    if custom {
                        let t = style.thickness.unwrap_or((size / 14.).max(1.)).max(0.);
                        let y = base + style.offset.unwrap_or(size * 0.12);
                        fill(
                            RectF::new(x, y, w, t),
                            ink.unwrap_or(style.color.unwrap_or(ink_color)),
                        );
                    } else {
                        let y = base - position - thickness * 0.5;
                        fill(RectF::new(x, y, w, thickness), ink_color);
                    }
                }
                if strike {
                    let y = base - x_height * 0.5 - thickness * 0.5;
                    fill(RectF::new(x, y, w, thickness), ink_color);
                }
                if overline {
                    let t = (size / 16.).max(1.);
                    let y = rect.top + line.top + size * 0.08 - t * 0.5;
                    fill(RectF::new(x, y, w, t), ink_color);
                }
            }
        }
        result
    }
}
/// Abscisses (depuis l'origine de la ligne) de l'intervalle UTF-16 `a..b`, espaces de fin
/// de ligne exclus.
fn span(line: &Line, a: usize, b: usize) -> Option<(f32, f32)> {
    let (x0, x1) = (line.offset(a), line.offset(b));
    let (left, right) = (x0.min(x1), x0.max(x1).min(line.width - line.trailing));
    (right > left).then_some((left, right))
}
