//! Atlas de glyphes : chaque glyphe est rastérisé une fois par Core Text
//! (`CTFontDrawGlyphs`, anticrénelage en niveaux de gris sans lissage, comme le dessin
//! Core Graphics) à l'échelle d'affichage et à une position sous-pixel transmise
//! au douzième de pixel (Core Graphics la quantifie comme dans son propre chemin), puis
//! rangé par étagères dans des pages de 1024 × 1024 : masques R8 teintés par le shader,
//! ou BGRA pour les glyphes en couleur (émojis). Sous `background-clip: text`, un glyphe
//! en couleur devient un masque de luminance (`Kind::Luminance`) : il est dessiné dans un
//! bitmap en niveaux de gris, comme le masque de texte de Core Graphics.
use super::device::{Gpu, Texture, replace};
use crate::platform::macos::{bitmap::Bitmap, graphics::same_font};
use objc2_core_foundation::{CFRetained, CGAffineTransform, CGPoint, CGRect};
use objc2_core_graphics::{CGContext, CGGlyph};
use objc2_core_text::{CTFont, CTFontOrientation};
use objc2_metal::MTLPixelFormat;
use std::collections::HashMap;

const PAGE: usize = 1024;
/// Positions sous-pixel horizontales retenues par pixel pour `font` à `scale`.
///
/// Core Graphics quantifie lui-même la position d'un glyphe selon sa taille en pixels
/// (tiers de pixel jusqu'à 16 px environ, demi-pixel au-delà, pixel entier pour les
/// grandes tailles, position presque continue pour les petites tailles de certaines
/// polices) et pose la ligne de base sur un pixel entier. L'atlas lui transmet une
/// position au douzième (ou au sixième) de pixel, dont il fait la même quantification
/// que dans le chemin Core Graphics.
pub(super) fn steps(font: &CTFont, scale: f32) -> f32 {
    // SAFETY : police valide.
    let pixels = unsafe { font.size() } as f32 * scale;
    if pixels <= 16.5 {
        12.
    } else if pixels <= 48. {
        6.
    } else {
        2.
    }
}
/// Rastérisation d'un glyphe.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Kind {
    /// Couverture (R8), teintée par le shader.
    Mask,
    /// Glyphe en couleur (BGRA), non teinté.
    Color,
    /// Glyphe en couleur converti en niveaux de gris (R8) : masque d'un remplissage.
    Luminance,
}
/// Pages au-delà desquelles l'atlas est vidé au début de l'image suivante.
const MAX_PAGES: usize = 8;

/// Glyphe placé dans l'atlas.
#[derive(Clone, Copy)]
pub(super) struct Glyph {
    pub page: usize,
    /// Coin de l'image du glyphe depuis le pixel de son origine.
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub uv: [f32; 4],
    /// Glyphe en couleur (texture BGRA, non teintée).
    pub colored: bool,
}
struct Page {
    texture: Texture,
    colored: bool,
    /// Étagère courante : ordonnée, hauteur et abscisse libre.
    shelf: (usize, usize, usize),
}
#[derive(Hash, PartialEq, Eq)]
struct Key {
    font: u32,
    glyph: CGGlyph,
    scale: u32,
    x: u8,
    y: u8,
    kind: Kind,
}
#[derive(Default)]
pub(super) struct Atlas {
    pages: Vec<Page>,
    glyphs: HashMap<Key, Option<Glyph>>,
    fonts: Vec<CFRetained<CTFont>>,
}
impl Atlas {
    /// `true` si l'atlas déborde et doit être vidé (le GPU ne doit plus le lire).
    pub fn full(&self) -> bool {
        self.pages.len() > MAX_PAGES
    }
    /// Octets alloués par les pages.
    pub fn bytes(&self) -> usize {
        use objc2_metal::MTLResource;
        self.pages.iter().map(|p| p.texture.allocatedSize()).sum()
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn texture(&self, page: usize) -> &Texture {
        &self.pages[page].texture
    }
    /// Numéro stable de `font` dans l'atlas.
    pub fn font(&mut self, font: &CFRetained<CTFont>) -> u32 {
        if let Some(i) = self
            .fonts
            .iter()
            .position(|f| std::ptr::eq(&**f, &**font))
            .or_else(|| self.fonts.iter().position(|f| same_font(f, font)))
        {
            return i as u32;
        }
        self.fonts.push(font.clone());
        self.fonts.len() as u32 - 1
    }
    /// Glyphe `glyph` de la police numéro `id`, à l'échelle `scale` et décalé de
    /// (`x`, `y`) fractions `1 / steps(font, scale)` de pixel ; `None` pour un glyphe sans encre.
    #[allow(clippy::too_many_arguments)]
    pub fn glyph(
        &mut self,
        gpu: &Gpu,
        id: u32,
        font: &CTFont,
        glyph: CGGlyph,
        scale: f32,
        (x, y): (u8, u8),
        kind: Kind,
    ) -> crate::Result<Option<Glyph>> {
        let key = Key {
            font: id,
            glyph,
            scale: scale.to_bits(),
            x,
            y,
            kind,
        };
        if let Some(found) = self.glyphs.get(&key) {
            return Ok(*found);
        }
        let entry = self.rasterize(gpu, font, glyph, scale, (x, y), kind)?;
        self.glyphs.insert(key, entry);
        Ok(entry)
    }
    fn rasterize(
        &mut self,
        gpu: &Gpu,
        font: &CTFont,
        mut glyph: CGGlyph,
        scale: f32,
        (x, y): (u8, u8),
        kind: Kind,
    ) -> crate::Result<Option<Glyph>> {
        let colored = kind == Kind::Color;
        // SAFETY : un glyphe, sortie du rectangle global seulement.
        let bounds: CGRect = unsafe {
            font.bounding_rects_for_glyphs(
                CTFontOrientation::Default,
                std::ptr::NonNull::from(&mut glyph),
                std::ptr::null_mut(),
                1,
            )
        };
        if !(bounds.size.width > 0. && bounds.size.height > 0.) {
            return Ok(None);
        }
        let s = f64::from(scale);
        let sub = f64::from(steps(font, scale));
        let (fx, fy) = (f64::from(x) / sub, f64::from(y) / sub);
        let x0 = fx + bounds.origin.x * s;
        let x1 = fx + (bounds.origin.x + bounds.size.width) * s;
        let y0 = fy - (bounds.origin.y + bounds.size.height) * s;
        let y1 = fy - bounds.origin.y * s;
        let (left, top) = (x0.floor() as i32 - 1, y0.floor() as i32 - 1);
        let (right, bottom) = (x1.ceil() as i32 + 1, y1.ceil() as i32 + 1);
        let (width, height) = ((right - left) as usize, (bottom - top) as usize);
        if width > PAGE || height > PAGE {
            return Ok(None);
        }
        let bitmap = if colored {
            Bitmap::new(width, height)?
        } else {
            Bitmap::gray(width, height)?
        };
        let c = Some(&*bitmap.context);
        CGContext::set_allows_antialiasing(c, true);
        CGContext::set_should_antialias(c, true);
        // Réglages du canvas Core Graphics (positionnement et quantification sous-pixel
        // par défaut).
        CGContext::set_should_smooth_fonts(c, false);
        CGContext::set_gray_fill_color(c, 1., 1.);
        CGContext::set_text_matrix(
            c,
            CGAffineTransform {
                a: 1.,
                b: 0.,
                c: 0.,
                d: 1.,
                tx: 0.,
                ty: 0.,
            },
        );
        // Origine du glyphe dans l'image (repère natif, y vers le haut).
        CGContext::translate_ctm(
            c,
            fx - f64::from(left),
            height as f64 - (fy - f64::from(top)),
        );
        CGContext::scale_ctm(c, s, s);
        let mut position = CGPoint::new(0., 0.);
        // SAFETY : un glyphe et une position valides ; contexte bitmap vivant.
        unsafe {
            font.draw_glyphs(
                std::ptr::NonNull::from(&mut glyph),
                std::ptr::NonNull::from(&mut position),
                1,
                &bitmap.context,
            )
        };
        CGContext::flush(c);
        let (page, px, py) = self.place(gpu, width, height, colored)?;
        replace(
            &self.pages[page].texture,
            (px, py, width, height),
            bitmap.pixels(),
            bitmap.row_bytes(),
        );
        let n = PAGE as f32;
        Ok(Some(Glyph {
            page,
            left,
            top,
            width,
            height,
            uv: [
                px as f32 / n,
                py as f32 / n,
                (px + width) as f32 / n,
                (py + height) as f32 / n,
            ],
            colored,
        }))
    }
    /// Place un rectangle de `width` × `height` (1 pixel de marge) dans une page.
    fn place(
        &mut self,
        gpu: &Gpu,
        width: usize,
        height: usize,
        colored: bool,
    ) -> crate::Result<(usize, usize, usize)> {
        let (w, h) = (width + 1, height + 1);
        for (index, page) in self.pages.iter_mut().enumerate().rev() {
            if page.colored != colored {
                continue;
            }
            let (y, shelf, x) = page.shelf;
            if x + w <= PAGE && h <= shelf {
                page.shelf.2 += w;
                return Ok((index, x, y));
            }
            // Nouvelle étagère sous la courante.
            if y + shelf + h <= PAGE && w <= PAGE {
                page.shelf = (y + shelf, h, w);
                return Ok((index, 0, y + shelf));
            }
            break;
        }
        let format = if colored {
            MTLPixelFormat::BGRA8Unorm
        } else {
            MTLPixelFormat::R8Unorm
        };
        let texture = gpu.upload_texture(format, PAGE, PAGE, false)?;
        self.pages.push(Page {
            texture,
            colored,
            shelf: (0, h, w),
        });
        Ok((self.pages.len() - 1, 0, 0))
    }
}
