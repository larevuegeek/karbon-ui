//! Dessin d'une mise en page : glyphes run par run (couleur de chaque portion), puis
//! soulignements, barrés et surlignes dessinés par le moteur.
use super::layout::{Layout, Line};
use crate::{
    platform::linux::ffi::{
        cairo::Context,
        glib::GSList,
        pango::{GlyphItem, SCALE, pango_cairo_show_glyph_string, pango_glyph_string_get_width},
    },
    ui::{scene::RectF, theme::Color},
};

/// Couleur Cairo d'une couleur du moteur, opacité `alpha` comprise.
pub(in crate::platform::linux) fn rgba(color: Color, alpha: f32) -> [f64; 4] {
    [
        f64::from(color.red) / 255.,
        f64::from(color.green) / 255.,
        f64::from(color.blue) / 255.,
        f64::from(color.alpha) / 255. * f64::from(alpha),
    ]
}
impl Layout {
    /// Dessine la mise en page dans `rect` (repère logique). `color` colore le texte sans
    /// couleur propre ; `ink` remplace toutes les couleurs (masques) ; `alpha` est
    /// l'opacité du groupe courant.
    pub fn draw(
        &self,
        context: &Context,
        rect: RectF,
        color: Color,
        ink: Option<Color>,
        alpha: f32,
    ) {
        let format = self.paragraph.format;
        context.save();
        if format.clip {
            context.rectangle(
                rect.left.into(),
                rect.top.into(),
                (rect.right - rect.left).into(),
                (rect.bottom - rect.top).into(),
            );
            context.clip();
        }
        let colour = |byte: usize| -> Color {
            ink.unwrap_or_else(|| self.paragraph.style_at(byte).color.unwrap_or(color))
        };
        let mut current = None;
        for line in &self.lines {
            let mut x = f64::from(rect.left + line.x);
            let y = f64::from(rect.top + line.baseline);
            for run in runs(line) {
                // SAFETY : run d'une ligne valide ; item et glyphes possédés par la mise en
                // page de la ligne.
                let (offset, font, glyphs, rise) = unsafe {
                    let item = &*(*run).item;
                    (
                        item.offset.max(0) as usize,
                        item.analysis.font,
                        (*run).glyphs,
                        (*run).y_offset,
                    )
                };
                let run_color = colour(offset + line.shift);
                if current != Some(run_color) {
                    let [r, g, b, a] = rgba(run_color, alpha);
                    context.set_source_rgba(r, g, b, a);
                    current = Some(run_color);
                }
                context.move_to(x, y - f64::from(rise) / SCALE);
                // SAFETY : contexte, police et glyphes valides pendant l'appel.
                unsafe { pango_cairo_show_glyph_string(context.raw(), font, glyphs) };
                // SAFETY : glyphes valides.
                x += f64::from(unsafe { pango_glyph_string_get_width(glyphs) }) / SCALE;
            }
        }
        context.new_path();
        for (r, c) in self.decoration_rects(rect, color, ink) {
            let [red, green, blue, a] = rgba(c, alpha);
            context.set_source_rgba(red, green, blue, a);
            context.rectangle(
                r.left.into(),
                r.top.into(),
                (r.right - r.left).into(),
                (r.bottom - r.top).into(),
            );
            context.fill();
        }
        context.restore();
    }
    /// Soulignements, barrés et surlignes de la mise en page posée dans `rect`, dans
    /// l'ordre du dessin.
    pub fn decoration_rects(
        &self,
        rect: RectF,
        color: Color,
        ink: Option<Color>,
    ) -> Vec<(RectF, Color)> {
        let format = self.paragraph.format;
        let size = format.size;
        let m = self.metrics;
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
                        // Position Pango : haut du trait, au-dessus de la ligne de base.
                        let t = m.underline_thickness.max(1.);
                        fill(RectF::new(x, base - m.underline_position, w, t), ink_color);
                    }
                }
                if strike {
                    let t = m.strikethrough_thickness.max(1.);
                    fill(
                        RectF::new(x, base - m.strikethrough_position, w, t),
                        ink_color,
                    );
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
/// Runs (`PangoGlyphItem`) d'une ligne, dans l'ordre visuel.
pub(super) fn runs(line: &Line) -> impl Iterator<Item = *mut GlyphItem> {
    // SAFETY : ligne valide tant que sa mise en page vit (emprunt de `line`).
    let mut node: *mut GSList = unsafe { (*line.raw).runs };
    std::iter::from_fn(move || {
        if node.is_null() {
            return None;
        }
        // SAFETY : maillon valide de la liste des runs.
        let (data, next) = unsafe { ((*node).data, (*node).next) };
        node = next;
        Some(data.cast::<GlyphItem>())
    })
}
/// Abscisses (depuis l'origine de la ligne) des octets `a..b`, espaces de fin de ligne
/// exclus.
fn span(line: &Line, a: usize, b: usize) -> Option<(f32, f32)> {
    let (x0, x1) = (line.offset(a), line.offset(b));
    let (left, right) = (x0.min(x1), x0.max(x1).min(line.width - line.trailing));
    (right > left).then_some((left, right))
}
