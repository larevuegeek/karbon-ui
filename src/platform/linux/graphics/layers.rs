//! Couches hors de la cible : groupes filtrés (`filter`), `backdrop-filter` et texte
//! rempli par un fond (`background-clip: text`). Les masques en dégradé (`mask-image`)
//! sont des groupes Cairo ouverts par la canvas.
//!
//! Les groupes filtrés et les `backdrop-filter` sont rendus en pixels de la cible, filtrés
//! sur CPU (filtres communs avec macOS), puis recomposés dans le repère des pixels.
use super::super::bitmap::{Bitmap, Surface};
use super::{cache, canvas::Canvas, geometry, paint, text::Layout};
use crate::{
    platform::{
        linux::ffi::cairo::{CONTENT_ALPHA, FILTER_BILINEAR, FILTER_GOOD, Matrix, Pattern},
        shared::{apply_filters, normalized, reduction},
    },
    ui::{
        element::ElementId,
        rendering::{
            brush::Background,
            filter::{Filter, Filters},
            transform::Affine,
        },
        scene::{DrawCommand, RectF, signature::Signature},
        theme::Color,
    },
};
use std::rc::Rc;

/// Zone entière en pixels de la cible : gauche, haut, largeur, hauteur.
#[derive(Clone, Copy)]
struct Area {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
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
        let [x1, y1, x2, y2] = self.context.clip_extents().map(|v| v as f32);
        let visible = self.transform.bounds(RectF {
            left: x1,
            top: y1,
            right: x2,
            bottom: y2,
        });
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
    /// Dessine `image` étirée sur le rectangle de pixels (`x`, `y`, `width`, `height`) de
    /// la cible, sous la découpe courante, à l'opacité du groupe courant.
    fn draw_device(
        &self,
        image: &dyn Surface,
        [x, y, width, height]: [f64; 4],
        filter: std::ffi::c_int,
    ) -> crate::Result<()> {
        let (iw, ih) = image.size();
        if width <= 0. || height <= 0. {
            return Ok(());
        }
        let pattern = Pattern::surface(image)?;
        pattern.set_filter(filter);
        // Des pixels de la cible vers ceux de l'image.
        let (sx, sy) = (iw as f64 / width, ih as f64 / height);
        pattern.set_matrix(Matrix {
            xx: sx,
            yy: sy,
            x0: -x * sx,
            y0: -y * sy,
            ..Matrix::IDENTITY
        });
        let c = self.context;
        c.save();
        c.identity_matrix();
        c.rectangle(x, y, width, height);
        c.clip();
        c.set_source(&pattern);
        c.paint_with_alpha(self.alpha.into());
        c.restore();
        Ok(())
    }
    /// Image posée sur des pixels de la cible (étirée si elle est en cours d'animation,
    /// pas encore redimensionnée exactement).
    pub(super) fn blit(
        &self,
        image: &dyn Surface,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> crate::Result<()> {
        let rect = [x, y, width, height].map(f64::from);
        self.draw_device(image, rect, FILTER_GOOD)
    }
    /// Couche filtrée posée sur `area` (agrandie si elle a été calculée réduite : une
    /// interpolation bilinéaire suffit pour une image floutée).
    fn composite(&self, image: &Bitmap, area: Area) -> crate::Result<()> {
        let rect = [area.x, area.y, area.w, area.h].map(|v| v as f64);
        self.draw_device(image, rect, FILTER_BILINEAR)
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
            return self.composite(&image, area);
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
            let context = layer.context()?;
            let mut inner = Canvas::new(
                &context,
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
        layer.edit(|pixels| apply_filters(pixels, w, h, filters, scale / k as f32));
        let image = Rc::new(layer);
        self.composite(&image, area)?;
        self.caches
            .store_filtered(id, key, placement, image, commands);
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
        let row = surface.row_bytes();
        let pixels = surface.pixels();
        let mut copy = Vec::with_capacity(area.w * area.h * 4);
        for y in area.y..area.y + area.h {
            let start = y * row + area.x * 4;
            copy.extend_from_slice(&pixels[start..start + area.w * 4]);
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
                let image = Rc::new(Bitmap::from_pixels(area.w, area.h, copy)?);
                self.caches.store_backdrop(key, hash, Rc::clone(&image));
                image
            }
        };
        let c = self.context;
        c.save();
        let (shape, radii) = normalized(rect, radii, None);
        geometry::rounded(c, shape, radii);
        c.clip();
        let result = self.composite(&image, area);
        c.restore();
        result
    }
    /// `background-clip: text` : les glyphes forment un masque d'alpha, `fill` est peint
    /// au travers.
    pub(super) fn text_fill(
        &mut self,
        layout: &Layout,
        rect: RectF,
        fill: &Background,
        area: RectF,
    ) -> crate::Result<()> {
        let size = layout.paragraph.format.size;
        let ink = RectF {
            left: rect.left - size,
            top: rect.top - size,
            right: rect.left + layout.size.width.max(rect.right - rect.left) + size,
            bottom: rect.top + layout.size.height.max(rect.bottom - rect.top) + size,
        };
        let c = self.context;
        c.save();
        // Le groupe prend la taille de la découpe : elle est réduite à l'encre du texte.
        geometry::rectangle(c, ink);
        c.clip();
        c.push_group_with_content(CONTENT_ALPHA);
        let white = Color::rgba(255, 255, 255, 255);
        layout.draw(c, rect, white, Some(white), 1.);
        let mask = c.pop_group();
        let result = mask.and_then(|mask| {
            match fill {
                Background::Solid(color) => self.set_fill(*color),
                Background::Linear(g) => c.set_source(&paint::linear(g, area, self.alpha, false)?),
                Background::Radial(g) => c.set_source(&paint::radial(g, area, self.alpha)?),
            }
            c.mask(&mask);
            Ok(())
        });
        c.restore();
        result
    }
}
