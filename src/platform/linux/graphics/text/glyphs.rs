//! Glyphes placés d'une mise en page, pour le renderer Vulkan : mêmes runs et mêmes
//! positions que le dessin Cairo (`draw.rs`), couleur de chaque portion.
use super::{draw::runs, layout::Layout};
use crate::{
    platform::linux::ffi::{
        glib::Object,
        pango::{SCALE, pango_glyph_string_get_width},
    },
    ui::{scene::PointF, theme::Color},
};
use std::rc::Rc;

/// Glyphes consécutifs d'une même police et d'une même couleur.
pub(in crate::platform::linux) struct GlyphRun {
    /// Police réellement employée par Pango (substitutions comprises).
    pub font: Object,
    /// Couleur propre de la portion ; `None` : couleur du texte.
    pub color: Option<Color>,
    /// La police dessine ces glyphes en couleur (émojis) : ils ne se teintent pas.
    pub colored: bool,
    /// Origine de la ligne (abscisse, ligne de base) depuis le coin de la boîte.
    pub line: PointF,
    /// Avance des runs précédents et relèvement (`rise`) du run, en unités logiques.
    pub shift: [f64; 2],
    pub glyphs: Vec<u32>,
    /// Décalage de chaque glyphe depuis le point de départ du run (repère logique, y vers
    /// le bas).
    pub offsets: Vec<PointF>,
}
impl GlyphRun {
    /// Point de départ du run (repère logique) dans une boîte de coin `corner`, calculé
    /// comme le `move_to` de `draw.rs`.
    pub fn start(&self, corner: PointF) -> [f64; 2] {
        [
            f64::from(corner.x + self.line.x) + self.shift[0],
            f64::from(corner.y + self.line.y) + self.shift[1],
        ]
    }
}
impl Layout {
    /// Runs de glyphes de toutes les lignes, extraits au premier appel.
    pub fn glyph_runs(&self) -> Rc<[GlyphRun]> {
        Rc::clone(self.glyphs.get_or_init(|| self.extract().into()))
    }
    fn extract(&self) -> Vec<GlyphRun> {
        let mut result = Vec::new();
        for line in &self.lines {
            let mut x = 0f64;
            for run in runs(line) {
                // SAFETY : run d'une ligne valide ; item, police et glyphes possédés par
                // la mise en page de la ligne ; `num_glyphs` glyphes valides.
                let (offset, font, rise, infos, width) = unsafe {
                    let item = &*(*run).item;
                    let glyphs = &*(*run).glyphs;
                    let infos = std::slice::from_raw_parts(
                        glyphs.glyphs,
                        glyphs.num_glyphs.max(0) as usize,
                    );
                    (
                        item.offset.max(0) as usize,
                        Object::from_none(item.analysis.font),
                        (*run).y_offset,
                        infos,
                        pango_glyph_string_get_width((*run).glyphs),
                    )
                };
                let Some(font) = font else {
                    continue;
                };
                let shift = [x, -f64::from(rise) / SCALE];
                let (mut glyphs, mut offsets, mut colored) = (Vec::new(), Vec::new(), false);
                let mut advance = 0i64;
                for info in infos {
                    if info.has_ink() {
                        glyphs.push(info.glyph);
                        offsets.push(PointF {
                            x: ((advance + i64::from(info.geometry.x_offset)) as f64 / SCALE)
                                as f32,
                            y: (f64::from(info.geometry.y_offset) / SCALE) as f32,
                        });
                        colored |= info.colored();
                    }
                    advance += i64::from(info.geometry.width);
                }
                x += f64::from(width) / SCALE;
                if glyphs.is_empty() {
                    continue;
                }
                result.push(GlyphRun {
                    font,
                    color: self.paragraph.style_at(offset + line.shift).color,
                    colored,
                    line: PointF {
                        x: line.x,
                        y: line.baseline,
                    },
                    shift,
                    glyphs,
                    offsets,
                });
            }
        }
        result
    }
}
