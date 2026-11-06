//! Couches hors de la cible : groupes filtrés (`filter`), masques en dégradé
//! (`mask-image`), `backdrop-filter` et texte rempli par un fond (`background-clip: text`).
//!
//! Les couches sont rendues en pixels de la cible (repère y vers le bas), puis
//! recomposées dans le repère natif de Core Graphics (pixels, y vers le haut).
use super::super::{
    bitmap::{self, Bitmap},
    ffi::affine,
};
use super::{cache, canvas::Canvas, canvas::flip, geometry, paint, text::Layout};
use crate::platform::shared::{apply_filters, reduction};
use crate::ui::{
    element::ElementId,
    rendering::{
        brush::{Background, LinearGradient},
        filter::{Filter, Filters},
        transform::Affine,
    },
    scene::{DrawCommand, RectF, signature::Signature},
    theme::Color,
};
use objc2_core_foundation::{CGAffineTransform, CGFloat, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGContext, CGImage, CGInterpolationQuality};

/// Zone entière en pixels de la cible : gauche, haut, largeur, hauteur.
#[derive(Clone, Copy)]
struct Area {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}
fn invert(t: CGAffineTransform) -> Option<CGAffineTransform> {
    let d = t.a * t.d - t.b * t.c;
    if !d.is_finite() || d.abs() < 1e-12 {
        return None;
    }
    Some(CGAffineTransform {
        a: t.d / d,
        b: -t.b / d,
        c: -t.c / d,
        d: t.a / d,
        tx: (t.c * t.ty - t.d * t.tx) / d,
        ty: (t.b * t.tx - t.a * t.ty) / d,
    })
}
impl Canvas<'_> {
    /// Facteur d'échelle moyen du repère courant (pixels par unité logique).
    fn scale(&self) -> f32 {
        let t = self.transform;
        (t.m11 * t.m22 - t.m12 * t.m21).abs().sqrt()
    }
    /// Pixels couverts par `rect` (repère logique), bornés à la cible et à la découpe
    /// courante élargie de `margin` pixels.
    fn area(&self, rect: RectF, margin: f32) -> Option<Area> {
        let device = self.transform.bounds(rect);
        let clip = CGContext::clip_bounding_box(Some(self.context));
        let clip = RectF::new(
            clip.origin.x as f32,
            clip.origin.y as f32,
            clip.size.width as f32,
            clip.size.height as f32,
        );
        let visible = self.transform.bounds(clip);
        let left = device.left.max(visible.left - margin).max(0.).floor();
        let top = device.top.max(visible.top - margin).max(0.).floor();
        let right = device
            .right
            .min(visible.right + margin)
            .min(self.width as f32)
            .ceil();
        let bottom = device
            .bottom
            .min(visible.bottom + margin)
            .min(self.height as f32)
            .ceil();
        if !(right > left && bottom > top) {
            return None;
        }
        Some(Area {
            x: left as usize,
            y: top as usize,
            w: (right - left) as usize,
            h: (bottom - top) as usize,
        })
    }
    /// Rectangle natif (y vers le haut) de `area`.
    fn native(&self, area: Area) -> CGRect {
        CGRect::new(
            CGPoint::new(
                area.x as CGFloat,
                (self.height - area.y - area.h) as CGFloat,
            ),
            CGSize::new(area.w as CGFloat, area.h as CGFloat),
        )
    }
    /// Exécute `f` dans le repère natif des pixels de la cible ; sans `save`/`restore`
    /// pour qu'une découpe posée par `f` persiste.
    fn in_device(&self, f: impl FnOnce(&CGContext)) {
        let c = Some(self.context);
        let ctm = CGContext::ctm(c);
        let Some(inverse) = invert(ctm) else {
            return;
        };
        CGContext::concat_ctm(c, inverse);
        f(self.context);
        CGContext::concat_ctm(c, ctm);
    }
    /// Dessine `image` sur le rectangle de pixels (`x`, `y`, `width`, `height`, y vers le
    /// bas) de la cible, sous la découpe courante.
    pub(super) fn blit(&self, image: &CGImage, x: f32, y: f32, width: f32, height: f32) {
        let rect = CGRect::new(
            CGPoint::new(
                CGFloat::from(x),
                self.height as CGFloat - CGFloat::from(y + height),
            ),
            CGSize::new(CGFloat::from(width), CGFloat::from(height)),
        );
        let scaled = CGImage::width(Some(image)) as f32 != width
            || CGImage::height(Some(image)) as f32 != height;
        self.in_device(|c| {
            CGContext::save_g_state(Some(c));
            if scaled {
                // Image en cours d'animation, pas encore redimensionnée exactement.
                CGContext::set_interpolation_quality(Some(c), CGInterpolationQuality::Medium);
            }
            CGContext::draw_image(Some(c), rect, Some(image));
            CGContext::restore_g_state(Some(c));
        });
    }
    /// Dessine `image` sur `area`, sous la découpe courante (agrandie si elle a été
    /// calculée réduite).
    fn composite(&self, image: &CGImage, area: Area) {
        let rect = self.native(area);
        self.in_device(|c| {
            CGContext::save_g_state(Some(c));
            // Image floutée agrandie : une interpolation bilinéaire suffit.
            CGContext::set_interpolation_quality(Some(c), CGInterpolationQuality::Low);
            CGContext::draw_image(Some(c), rect, Some(image));
            CGContext::restore_g_state(Some(c));
        });
    }
    /// Repère d'une couche couvrant `area` : celui de la cible, décalé.
    fn layer_transform(&self, area: Area) -> Affine {
        self.transform
            .then(Affine::translation(-(area.x as f32), -(area.y as f32)))
    }
    /// Groupe filtré : commandes rendues dans une couche, filtrées sur CPU, recomposées.
    pub(super) fn filter(
        &mut self,
        id: ElementId,
        commands: &[DrawCommand],
        bounds: RectF,
        filters: Filters,
    ) -> crate::Result<()> {
        let scale = self.scale();
        let margin = filters.padding() * scale;
        let output = filters.bounds(bounds);
        // Zone entière tant qu'elle est raisonnable : elle ne dépend alors pas de la
        // découpe courante et le résultat reste en cache pendant un défilement.
        let whole = self.area(output, f32::INFINITY);
        let area = match whole {
            Some(a) if a.w * a.h <= 4_194_304 => Some(a),
            _ => self.area(output, margin),
        };
        let Some(area) = area else {
            return Ok(());
        };
        let key = Signature::new(commands, bounds, filters, scale);
        let placement = cache::placement(self.layer_transform(area), area.w, area.h);
        if let Some(image) = self.caches.filtered(id, &key, placement) {
            self.composite(&image, area);
            return Ok(());
        }
        if area.w * area.h > 16_777_216 {
            return Err(crate::Error::invalid_input(
                "filter group exceeds its 16M physical pixel limit",
            ));
        }
        // Un grand flou efface les détails : le contenu est dessiné réduit d'un facteur
        // `k`, flouté, puis agrandi à la recomposition (coût divisé par k²).
        let k = reduction(filters, scale);
        let (w, h) = (area.w.div_ceil(k), area.h.div_ceil(k));
        let shrink = Affine {
            m11: 1. / k as f32,
            m22: 1. / k as f32,
            ..Affine::IDENTITY
        };
        let mut layer = Bitmap::new(w, h)?;
        {
            let mut inner = Canvas::new(
                &layer.context,
                w,
                h,
                None,
                self.layer_transform(area).then(shrink),
                self.fonts,
                self.text,
                self.caches,
            );
            inner.text_fills = self.text_fills.clone();
            let result = inner.run(commands);
            inner.finish();
            result?;
        }
        CGContext::flush(Some(&layer.context));
        apply_filters(layer.pixels_mut(), w, h, filters, scale / k as f32);
        let image = layer.image()?;
        self.composite(&image, area);
        self.caches
            .store_filtered(id, key, placement, image, commands);
        Ok(())
    }
    /// `mask-image` : découpe par l'alpha d'un dégradé posé sur `bounds`, étendu au-delà.
    pub(super) fn mask(&mut self, bounds: RectF, gradient: &LinearGradient) -> crate::Result<()> {
        let everything = RectF::new(-1e6, -1e6, 2e6, 2e6);
        let Some(area) = self.area(everything, 0.) else {
            return Ok(());
        };
        let mask = Bitmap::gray(area.w, area.h)?;
        let c = Some(&*mask.context);
        CGContext::concat_ctm(c, flip(area.h));
        CGContext::concat_ctm(c, affine(self.layer_transform(area)));
        paint::draw_linear(&mask.context, gradient, bounds, true)?;
        let image = mask.image()?;
        let rect = self.native(area);
        self.in_device(|c| CGContext::clip_to_mask(Some(c), rect, Some(&image)));
        Ok(())
    }
    /// `backdrop-filter` : filtre les pixels déjà peints sous la forme et les repeint
    /// dedans. Sans surface lisible (couche de filtre), rien n'est fait.
    pub(super) fn backdrop(
        &mut self,
        rect: RectF,
        radii: [f32; 4],
        filters: Filters,
    ) -> crate::Result<()> {
        let Some(surface) = self.surface else {
            return Ok(());
        };
        let blur = filters
            .values
            .iter()
            .map(|f| match f {
                Filter::Blur(n) => *n,
                _ => 0.,
            })
            .sum::<f32>()
            * filters.scale;
        let margin = (blur * 3.).ceil();
        let grown = RectF {
            left: rect.left - margin,
            top: rect.top - margin,
            right: rect.right + margin,
            bottom: rect.bottom + margin,
        };
        let Some(area) = self.area(grown, margin * self.scale()) else {
            return Ok(());
        };
        CGContext::flush(Some(self.context));
        let row = surface.row_bytes();
        let mut copy = Vec::with_capacity(area.w * area.h * 4);
        for y in area.y..area.y + area.h {
            let start = y * row + area.x * 4;
            copy.extend_from_slice(&surface.pixels()[start..start + area.w * 4]);
        }
        // Mêmes pixels dessous qu'à l'image précédente : le résultat filtré est réutilisé.
        let bits = |r: RectF| [r.left, r.top, r.right, r.bottom].map(|v| u64::from(v.to_bits()));
        let [l, t, r, b] = bits(rect);
        let key = [
            l,
            t,
            r,
            b,
            radii
                .iter()
                .fold(0u64, |h, v| h.rotate_left(16) ^ u64::from(v.to_bits())),
            filters.values.as_ptr() as u64 ^ u64::from(filters.scale.to_bits()),
            (area.x as u64) << 32 | area.y as u64,
            (area.w as u64) << 32 | area.h as u64,
        ];
        let hash = cache::hash(&copy);
        let image = match self.caches.backdrop(key, hash) {
            Some(image) => image,
            None => {
                apply_filters(&mut copy, area.w, area.h, filters, self.scale());
                let image = bitmap::image_from_bgra(area.w, area.h, &copy)?;
                self.caches.store_backdrop(key, hash, image.clone());
                image
            }
        };
        let c = Some(self.context);
        CGContext::save_g_state(c);
        let (shape, radii) = geometry::normalized(rect, radii, None);
        CGContext::add_path(c, Some(&geometry::rounded(shape, radii).0));
        CGContext::clip(c);
        self.composite(&image, area);
        CGContext::restore_g_state(c);
        Ok(())
    }
    /// `background-clip: text` : les glyphes forment un masque, `fill` est peint dessous.
    pub(super) fn text_fill(
        &mut self,
        layout: &Layout,
        rect: RectF,
        fill: &Background,
        area_rect: RectF,
    ) -> crate::Result<()> {
        let size = layout.paragraph.format.size;
        let ink = RectF {
            left: rect.left - size,
            top: rect.top - size,
            right: rect.left + layout.size.width.max(rect.right - rect.left) + size,
            bottom: rect.top + layout.size.height.max(rect.bottom - rect.top) + size,
        };
        let Some(area) = self.area(ink, 0.) else {
            return Ok(());
        };
        let mask = Bitmap::gray(area.w, area.h)?;
        let c = Some(&*mask.context);
        CGContext::concat_ctm(c, flip(area.h));
        CGContext::concat_ctm(c, affine(self.layer_transform(area)));
        CGContext::set_should_smooth_fonts(c, false);
        layout.draw(
            &mask.context,
            rect,
            Color::rgba(255, 255, 255, 255),
            Some(Color::rgba(255, 255, 255, 255)),
        );
        let image = mask.image()?;
        let native = self.native(area);
        let context = Some(self.context);
        CGContext::save_g_state(context);
        self.in_device(|c| CGContext::clip_to_mask(Some(c), native, Some(&image)));
        let whole = geometry::round_rect(area_rect, 0.);
        let result = self.fill_background(&whole, fill, area_rect);
        CGContext::restore_g_state(context);
        result
    }
}
