//! Services du renderer Cairo prêtés au renderer Vulkan (`linux/vulkan/`) : mises en page
//! Pango (mêmes lignes, runs et positions que le dessin Cairo), rastérisation d'un glyphe
//! isolé pour l'atlas avec les options de police du chemin Cairo (niveaux de gris, sans
//! hinting), images préparées comme pour le dessin Cairo (réduites à leur taille
//! d'affichage, mêmes caches) et rastérisation d'une commande seule (icônes, SVG,
//! polygones). Une image traduite pour le GPU est encadrée par `begin_frame` et
//! `end_frame`, comme une peinture Cairo.
//!
//! Cairo (1.17.4 et suivants) pose chaque glyphe au quart de pixel, sur les deux axes :
//! pixel `floor(v + 1/8)`, phase `floor(4 (v + 1/8))` modulo 4 (`quarter`), et rastérise
//! le glyphe décalé de sa phase ; un glyphe en couleur est posé au pixel le plus proche. `rasterize_glyph` à une phase donne les pixels exacts du
//! dessin Cairo (test `graphics/tests/glyphs.rs`). La position `v` elle-même suit le
//! trajet de Cairo (`device_position`) : point de départ du run arrondi au 1/256 de pixel
//! (point fixe des chemins), ramené au repère logique, décalé du glyphe, transformé.
use super::{Renderer, canvas::Canvas, text};
use crate::{
    platform::linux::{
        bitmap::{Bitmap, Surface},
        ffi::{
            cairo::Matrix,
            glib::Object,
            pango::{
                GlyphInfo, Rectangle, SCALE, pango_cairo_show_glyph_string,
                pango_font_get_glyph_extents, pango_glyph_string_free, pango_glyph_string_new,
                pango_glyph_string_set_size,
            },
        },
    },
    ui::{
        rendering::transform::Affine,
        scene::{DrawCommand, Image, RectF, Scene},
        styling::text::RichParagraph,
        theme::Color,
    },
};
use std::{rc::Rc, sync::Arc};

/// Texte prêt à dessiner : glyphes placés (relatifs à la boîte) et décorations.
pub(in crate::platform::linux) struct TextGlyphs {
    pub runs: Rc<[text::GlyphRun]>,
    /// Rectangles des décorations (repère logique absolu) et leur couleur.
    pub decorations: Vec<(RectF, Color)>,
    /// Le texte est coupé à sa boîte (`TextFormat::clip`).
    pub clip: bool,
}
/// Image d'un glyphe : coin depuis le pixel de son origine, pixels (alpha seul, ou BGRA
/// prémultiplié pour un glyphe en couleur) et octets par ligne.
#[derive(Debug)]
pub(in crate::platform::linux) struct GlyphImage {
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub stride: usize,
}
impl Renderer {
    /// Début d'une image traduite pour le GPU : surfaces des images de `scene` (celles
    /// qui n'y sont plus sont libérées), caches marqués inutilisés.
    pub(in crate::platform::linux) fn begin_frame(&mut self, scene: &Scene) -> crate::Result<()> {
        self.prepare_images(scene)?;
        self.caches.begin();
        Ok(())
    }
    /// Fin de cette image : les entrées des caches qu'elle n'a pas employées sont libérées.
    pub(in crate::platform::linux) fn end_frame(&mut self) {
        self.caches.end();
    }
    /// Oublie l'image précédente : la prochaine image Cairo est dessinée en entier (la
    /// surface a été présentée par Vulkan entre-temps).
    pub(in crate::platform::linux) fn forget(&mut self) {
        self.previous = None;
        self.examined = None;
        self.fresh = false;
    }
    /// Surface que le dessin Cairo poserait pour `image` affichée sur `width` × `height`
    /// pixels : réduite à cette taille, ou d'origine (même taille, trop grande, ou taille
    /// animée : étirée) ; `None` pour une image absente de la scène.
    pub(in crate::platform::linux) fn display_image(
        &mut self,
        image: &Arc<Image>,
        width: usize,
        height: usize,
    ) -> crate::Result<Option<Rc<dyn Surface>>> {
        self.caches.image(image, width, height)
    }
    /// Rastérise `command` seule dans une surface transparente de `width` × `height`
    /// pixels, sous `transform` (repère logique → pixels de la surface) ; pixels BGRA
    /// prémultipliés.
    pub(in crate::platform::linux) fn rasterize(
        &mut self,
        command: &DrawCommand,
        width: usize,
        height: usize,
        transform: Affine,
    ) -> crate::Result<Vec<u8>> {
        let target = Bitmap::new(width, height)?;
        {
            let context = target.context()?;
            let mut canvas = Canvas::new(
                &context,
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
            result?;
        }
        Ok(target.into_pixels())
    }
    /// Glyphes et décorations de `paragraph` posé dans `rect`, mis en page comme le dessin
    /// Cairo (même cache de mises en page).
    pub(in crate::platform::linux) fn text_glyphs(
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
}
/// Position `v` (pixels de la cible) coupée comme Cairo : pixel entier et phase en
/// quarts de pixel. Un glyphe en couleur (image d'émoji) n'a pas de phase : Cairo le pose
/// au pixel le plus proche.
pub(in crate::platform::linux) fn quarter(v: f64, colored: bool) -> (i32, u8) {
    if colored {
        return ((v + 0.5).floor() as i32, 0);
    }
    let n = (4. * (v + 0.125)).floor();
    let whole = (n / 4.).floor();
    (whole as i32, (n - whole * 4.) as u8)
}
/// Position en pixels d'un glyphe dessiné par Cairo, sur un axe : `start` est le point
/// de départ du run (repère logique, `move_to`), `offset` le décalage du glyphe, `scale`
/// et `translate` la transformation (droite) du contexte.
pub(in crate::platform::linux) fn device_position(
    start: f64,
    offset: f64,
    scale: f64,
    translate: f64,
) -> f64 {
    let fixed = ((scale * start + translate) * 256.).round_ties_even() / 256.;
    // Inverse d'une matrice d'échelle et de translation, comme `cairo_matrix_invert`.
    let inverse = 1. / scale;
    let user = inverse * fixed + -translate * inverse;
    scale * (user + offset) + translate
}
/// Glyphe `glyph` de `font` rastérisé à l'échelle `scale`, son origine décalée de
/// (`fx`, `fy`) pixel (quarts de pixel pour suivre Cairo) ; `None` pour un glyphe sans
/// encre ou plus grand qu'une page d'atlas.
pub(in crate::platform::linux) fn rasterize_glyph(
    font: &Object,
    glyph: u32,
    scale: f32,
    (fx, fy): (f32, f32),
    colored: bool,
) -> crate::Result<Option<GlyphImage>> {
    let mut ink = Rectangle::default();
    // SAFETY : police valide ; sortie locale (étendue logique non demandée).
    unsafe { pango_font_get_glyph_extents(font.as_ptr(), glyph, &mut ink, std::ptr::null_mut()) };
    if ink.width <= 0 || ink.height <= 0 {
        return Ok(None);
    }
    let s = f64::from(scale);
    let px = |v: i32| f64::from(v) / SCALE * s;
    let (fx, fy) = (f64::from(fx), f64::from(fy));
    let left = (fx + px(ink.x)).floor() as i32 - 1;
    let top = (fy + px(ink.y)).floor() as i32 - 1;
    let right = (fx + px(ink.x + ink.width)).ceil() as i32 + 1;
    let bottom = (fy + px(ink.y + ink.height)).ceil() as i32 + 1;
    let (width, height) = ((right - left) as usize, (bottom - top) as usize);
    if width > 1024 || height > 1024 {
        return Ok(None);
    }
    let surface = if colored {
        Bitmap::new(width, height)?
    } else {
        Bitmap::alpha(width, height)?
    };
    {
        let context = surface.context()?;
        context.transform(Matrix {
            x0: fx - f64::from(left),
            y0: fy - f64::from(top),
            ..Matrix::scaling(s, s)
        });
        context.set_source_rgba(1., 1., 1., 1.);
        context.move_to(0., 0.);
        show_glyph(&context, font, glyph);
    }
    let stride = surface.row_bytes();
    Ok(Some(GlyphImage {
        left,
        top,
        width,
        height,
        pixels: surface.pixels().to_vec(),
        stride,
    }))
}
/// Dessine le glyphe `glyph` de `font` au point courant de `context`.
pub(super) fn show_glyph(
    context: &crate::platform::linux::ffi::cairo::Context,
    font: &Object,
    glyph: u32,
) {
    // SAFETY : chaîne d'un glyphe, créée et libérée ici ; police et contexte valides.
    unsafe {
        let string = pango_glyph_string_new();
        pango_glyph_string_set_size(string, 1);
        let info: &mut GlyphInfo = &mut *(*string).glyphs;
        info.glyph = glyph;
        info.geometry.width = 0;
        info.geometry.x_offset = 0;
        info.geometry.y_offset = 0;
        info.attr = 0;
        pango_cairo_show_glyph_string(context.raw(), font.as_ptr(), string);
        pango_glyph_string_free(string);
    }
}
