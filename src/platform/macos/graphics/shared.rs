//! Services du renderer Core Graphics prêtés au renderer Metal (`platform/macos/metal/`) :
//! mises en page Core Text et rastérisation d'une commande isolée (icônes, SVG, polygones),
//! pour que les deux chemins partagent la même géométrie.
use super::{Renderer, canvas::Canvas, text};
use crate::ui::{
    rendering::transform::Affine,
    scene::{DrawCommand, RectF},
    styling::text::RichParagraph,
    theme::Color,
};
use std::rc::Rc;

/// Texte prêt à dessiner : glyphes placés (relatifs à la boîte) et décorations.
pub(in crate::platform::macos) struct TextGlyphs {
    pub runs: Rc<[text::GlyphRun]>,
    /// Rectangles des décorations (repère logique absolu) et leur couleur.
    pub decorations: Vec<(RectF, Color)>,
    /// Le texte est coupé à sa boîte (`TextFormat::clip`).
    pub clip: bool,
}
impl Renderer {
    /// Glyphes et décorations de `paragraph` posé dans `rect`, mis en page comme le dessin
    /// Core Graphics (même cache de mises en page).
    pub(in crate::platform::macos) fn text_glyphs(
        &mut self,
        paragraph: &RichParagraph,
        rect: RectF,
        color: Color,
        wrap: bool,
    ) -> crate::Result<TextGlyphs> {
        let layout = self
            .text
            .layout(&mut self.fonts, paragraph, rect.right - rect.left, wrap)?;
        Ok(TextGlyphs {
            runs: layout.glyph_runs(),
            decorations: layout.decoration_rects(rect, color, None),
            clip: layout.paragraph.format.clip,
        })
    }
    /// Pixels BGRA prémultipliés (`width` × `height`, transparents au départ) de `command`
    /// dessinée par Core Graphics avec `transform` (repère logique → pixels de l'image).
    pub(in crate::platform::macos) fn rasterize(
        &mut self,
        command: &DrawCommand,
        width: usize,
        height: usize,
        transform: Affine,
    ) -> crate::Result<Vec<u8>> {
        let target = super::Bitmap::new(width, height)?;
        let result = {
            let mut canvas = Canvas::new(
                &target.context,
                width,
                height,
                None,
                transform,
                &mut self.fonts,
                &mut self.text,
                &mut self.caches,
            );
            let result = canvas.run(std::slice::from_ref(command));
            canvas.finish();
            result
        };
        objc2_core_graphics::CGContext::flush(Some(&target.context));
        result?;
        Ok(target.into_pixels())
    }
    /// Oublie l'image précédente : la prochaine image Core Graphics est dessinée en entier
    /// (la surface a été présentée par un autre chemin entre-temps).
    pub(in crate::platform::macos) fn forget(&mut self) {
        self.previous = None;
        self.examined = None;
        self.fresh = false;
    }
    /// Libère les caches propres au dessin Core Graphics (images converties et
    /// redimensionnées, groupes filtrés, `backdrop-filter`, inventaires des zones) quand
    /// un autre chemin présente les images ; les mises en page du texte, partagées avec
    /// Metal, restent. La prochaine image Core Graphics est dessinée en entier.
    pub(in crate::platform::macos) fn release(&mut self) {
        self.forget();
        self.last_region = None;
        self.caches = Default::default();
    }
}
