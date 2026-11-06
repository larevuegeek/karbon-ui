//! Glyphes placés d'une mise en page, pour le renderer Metal : même découpage en runs
//! et mêmes positions que le dessin Core Text (`draw.rs`), couleur de chaque portion.
use super::layout::Layout;
use crate::ui::{scene::PointF, theme::Color};
use objc2_core_foundation::{CFArray, CFRetained, CFType, CGPoint};
use objc2_core_graphics::CGGlyph;
use objc2_core_text::{CTFont, CTRun, kCTFontAttributeName};
use std::rc::Rc;

/// Glyphes consécutifs d'une même police et d'une même couleur.
pub(in crate::platform::macos) struct GlyphRun {
    /// Police réellement employée par Core Text (substitutions comprises).
    pub font: CFRetained<CTFont>,
    /// Couleur propre de la portion ; `None` : couleur du texte.
    pub color: Option<Color>,
    pub glyphs: Vec<CGGlyph>,
    /// Origine de chaque glyphe (repère logique, y vers le bas), depuis le coin haut
    /// gauche de la boîte du texte.
    pub origins: Vec<PointF>,
}
impl Layout {
    /// Runs de glyphes de toutes les lignes, extraits au premier appel.
    pub fn glyph_runs(&self) -> Rc<[GlyphRun]> {
        Rc::clone(self.glyphs.get_or_init(|| self.extract().into()))
    }
    fn extract(&self) -> Vec<GlyphRun> {
        let mut result = Vec::new();
        // SAFETY : clé constante de Core Text.
        let font_key = unsafe { kCTFontAttributeName };
        for line in &self.lines {
            // SAFETY : ligne valide ; tableau de CTRun d'après Core Text.
            let runs = unsafe { line.line.glyph_runs() };
            // SAFETY : idem.
            let runs: &CFArray<CTRun> = unsafe { runs.cast_unchecked() };
            for run in runs {
                // SAFETY : run valide.
                let count = unsafe { run.glyph_count() }.max(0) as usize;
                if count == 0 {
                    continue;
                }
                // SAFETY : dictionnaire d'attributs du run ; la police est un CTFont.
                let font = unsafe {
                    let attributes = run.attributes();
                    let key: *const objc2_core_foundation::CFString = font_key;
                    let value = attributes.value(key.cast());
                    if value.is_null() {
                        continue;
                    }
                    let font = &*(value as *const CTFont);
                    CFRetained::retain(std::ptr::NonNull::from(font))
                };
                let mut glyphs = vec![0 as CGGlyph; count];
                let mut positions = vec![CGPoint::new(0., 0.); count];
                let all = super::super::super::ffi::range(0, 0);
                // SAFETY : tampons de `count` éléments, remplis par Core Text.
                unsafe {
                    run.glyphs(all, std::ptr::NonNull::new_unchecked(glyphs.as_mut_ptr()));
                    run.positions(
                        all,
                        std::ptr::NonNull::new_unchecked(positions.as_mut_ptr()),
                    );
                }
                // SAFETY : run valide.
                let start = unsafe { run.string_range() }.location.max(0) as usize;
                let color = self
                    .bytes
                    .get(start)
                    .and_then(|byte| self.paragraph.style_at(*byte).color);
                let origins = positions
                    .iter()
                    .map(|p| PointF {
                        x: line.x + p.x as f32,
                        y: line.baseline - p.y as f32,
                    })
                    .collect();
                result.push(GlyphRun {
                    font,
                    color,
                    glyphs,
                    origins,
                });
            }
        }
        result
    }
}
/// `true` si `font` dessine des glyphes en couleur (émojis) : ils ne se teintent pas.
pub(in crate::platform::macos) fn colored(font: &CTFont) -> bool {
    // SAFETY : police valide.
    let traits = unsafe { font.symbolic_traits() };
    traits.contains(objc2_core_text::CTFontSymbolicTraits::TraitColorGlyphs)
}
/// Identité Core Foundation d'une police (égalité `CFEqual`).
pub(in crate::platform::macos) fn same_font(a: &CTFont, b: &CTFont) -> bool {
    let (a, b): (&CFType, &CFType) = (a.as_ref(), b.as_ref());
    std::ptr::eq(a, b) || a == b
}
