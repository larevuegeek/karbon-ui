//! Images et commandes rastérisées une fois par Cairo : icônes vectorielles, SVG et
//! polygones, à leur taille d'affichage. Copie de `macos/metal/encode/raster.rs`.
//!
//! Une image sans rotation ni miroir reprend la décision du dessin Cairo : même rectangle
//! arrondi au pixel, mêmes pixels réduits à la taille d'affichage (`display_image`),
//! posés pixel pour pixel ; étirée (taille animée, trop grande) ou tournée, l'image
//! d'origine est échantillonnée avec ses mipmaps. Une rastérisation sous un repère qui
//! garde les axes est posée pixel pour pixel (même couverture que le dessin Cairo) ; sous
//! une rotation, elle est faite dans le repère local et échantillonnée.
use super::{Bind, Builder};
use crate::{
    platform::{
        linux::vulkan::textures::RasterKey,
        shared::gpu::{Instance, PAINT_TEXTURE, SHAPE_NONE, axis_aligned, pixel},
    },
    ui::{
        rendering::transform::Affine,
        scene::{DrawCommand, Image, PointF, RectF},
    },
};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

/// Plus grand côté d'une rastérisation, en pixels.
const MAX_RASTER: usize = 8192;

fn bounds_of(points: impl Iterator<Item = PointF>) -> Option<RectF> {
    let mut r: Option<RectF> = None;
    for p in points {
        let b = r.get_or_insert(RectF {
            left: p.x,
            top: p.y,
            right: p.x,
            bottom: p.y,
        });
        b.left = b.left.min(p.x);
        b.top = b.top.min(p.y);
        b.right = b.right.max(p.x);
        b.bottom = b.bottom.max(p.y);
    }
    r.filter(|r| {
        [r.left, r.top, r.right, r.bottom]
            .iter()
            .all(|n| n.is_finite())
    })
}
/// Bornes locales de la commande, marge des traits comprise, et empreinte de son
/// contenu relative au coin de ces bornes.
fn footprint(command: &DrawCommand) -> Option<(RectF, f32, u64)> {
    let mut h = DefaultHasher::new();
    let color = |h: &mut DefaultHasher, c: crate::ui::theme::Color| {
        [c.red, c.green, c.blue, c.alpha].hash(h)
    };
    match command {
        DrawCommand::Vector {
            rect,
            icon,
            color: c,
            outline,
        } => {
            1u8.hash(&mut h);
            (Arc::as_ptr(icon) as usize).hash(&mut h);
            color(&mut h, *c);
            [
                outline,
                &(rect.right - rect.left),
                &(rect.bottom - rect.top),
            ]
            .map(|n| n.to_bits())
            .hash(&mut h);
            let pad = outline.max(0.) * (rect.right - rect.left) / icon.width.max(1e-3);
            Some((*rect, pad, h.finish()))
        }
        DrawCommand::Svg {
            rect,
            image,
            color: c,
        } => {
            2u8.hash(&mut h);
            (*image as *const _ as usize).hash(&mut h);
            color(&mut h, *c);
            [rect.right - rect.left, rect.bottom - rect.top]
                .map(f32::to_bits)
                .hash(&mut h);
            let place = image.placement(*rect);
            let scale = pixel(place).recip();
            let mut pad = 0f32;
            let points = image.shapes.iter().flat_map(|shape| {
                if shape.stroke_width > 0. {
                    pad = pad.max(shape.stroke_width * shape.miter_limit.max(1.) / 2. * scale);
                }
                shape.ops.iter().flat_map(|op| {
                    use crate::ui::rendering::svg::PathOp;
                    let pts: Vec<(f32, f32)> = match *op {
                        PathOp::MoveTo(x, y) | PathOp::LineTo(x, y) => vec![(x, y)],
                        PathOp::CubicTo(a, b, c, d, e, f) => vec![(a, b), (c, d), (e, f)],
                        PathOp::Close => Vec::new(),
                    };
                    pts.into_iter().map(|(x, y)| place.point(PointF { x, y }))
                })
            });
            let bounds = bounds_of(points.collect::<Vec<_>>().into_iter())?;
            Some((bounds, pad, h.finish()))
        }
        DrawCommand::Path {
            points,
            closed,
            color: c,
        } => {
            let bounds = bounds_of(points.iter().copied())?;
            3u8.hash(&mut h);
            closed.hash(&mut h);
            color(&mut h, *c);
            for p in points {
                [p.x - bounds.left, p.y - bounds.top]
                    .map(f32::to_bits)
                    .hash(&mut h);
            }
            // Polyligne ouverte de 2 px, jonctions en onglet (limite 10).
            Some((bounds, if *closed { 0. } else { 10. }, h.finish()))
        }
        _ => None,
    }
}
impl Builder<'_> {
    pub(super) fn image(&mut self, rect: RectF, image: &Arc<Image>) -> crate::Result<()> {
        let t = self.transform();
        let (quad, m, surface, mipmaps) = if t.m12 == 0. && t.m21 == 0. && t.m11 > 0. && t.m22 > 0.
        {
            // Posée sur les pixels de la cible, comme le dessin Cairo (`paint.rs`).
            let device = t.bounds(rect);
            let (x, y) = (device.left.round(), device.top.round());
            let w = (device.right - device.left).round().max(0.);
            let h = (device.bottom - device.top).round().max(0.);
            let Some(surface) = self.cairo.display_image(image, w as usize, h as usize)? else {
                return Ok(());
            };
            if w == 0. || h == 0. {
                return Ok(());
            }
            let stretched = surface.size() != (w as usize, h as usize);
            (RectF::new(x, y, w, h), Affine::IDENTITY, surface, stretched)
        } else {
            let (w, h) = (image.width as usize, image.height as usize);
            let Some(surface) = self.cairo.display_image(image, w, h)? else {
                return Ok(());
            };
            (rect, t, surface, true)
        };
        let resources = &mut *self.resources;
        let view = resources
            .textures
            .image(&mut resources.staging, image, &*surface, mipmaps)?;
        self.emit(
            Instance::new(quad, m, SHAPE_NONE, PAINT_TEXTURE),
            Bind::Texture(view),
        );
        Ok(())
    }
    pub(super) fn raster(&mut self, command: &DrawCommand) -> crate::Result<()> {
        if let DrawCommand::Vector { icon, .. } = command
            && !(icon.width.is_finite()
                && icon.width > 0.
                && icon.height.is_finite()
                && icon.height > 0.)
        {
            return Err(crate::Error::invalid_input("invalid icon dimensions"));
        }
        let Some((bounds, pad, content)) = footprint(command) else {
            return Ok(());
        };
        let t = self.transform();
        let pad = pad + 2. * pixel(t);
        let local = RectF {
            left: bounds.left - pad,
            top: bounds.top - pad,
            right: bounds.right + pad,
            bottom: bounds.bottom + pad,
        };
        let (key, size, raster, quad, m) = if axis_aligned(t) {
            let d = t.bounds(local);
            let (x, y) = (d.left.floor(), d.top.floor());
            let (w, h) = (
                (d.right.ceil() - x) as usize,
                (d.bottom.ceil() - y) as usize,
            );
            let o = t.point(PointF {
                x: bounds.left,
                y: bounds.top,
            });
            // Position sous-pixel quantifiée au quart, comme les glyphes : au plus seize
            // rastérisations par contenu, réutilisées pendant un défilement fractionnaire
            // (au lieu d'une par image), pour un écart d'au plus 1/8 de pixel.
            let quarter = |v: f32| (v * 4.).round() / 4.;
            let offset = (quarter(o.x - x), quarter(o.y - y));
            let key = RasterKey {
                content,
                scale: [t.m11.to_bits(), t.m22.to_bits()],
                offset: [offset.0.to_bits(), offset.1.to_bits()],
            };
            let raster = t.then(Affine::translation(
                -x + offset.0 - (o.x - x),
                -y + offset.1 - (o.y - y),
            ));
            let quad = RectF {
                left: x,
                top: y,
                right: x + w as f32,
                bottom: y + h as f32,
            };
            (key, (w, h), raster, quad, Affine::IDENTITY)
        } else {
            // Échelle quantifiée au huitième : une animation d'échelle réutilise ses images.
            let s = ((pixel(t).recip() * 8.).round() / 8.).max(0.125);
            let (w, h) = (
                ((local.right - local.left) * s).ceil() as usize,
                ((local.bottom - local.top) * s).ceil() as usize,
            );
            let key = RasterKey {
                content,
                scale: [s.to_bits(); 2],
                offset: [u32::MAX; 2],
            };
            let raster = Affine {
                m11: s,
                m22: s,
                dx: -local.left * s,
                dy: -local.top * s,
                ..Affine::IDENTITY
            };
            let quad = RectF {
                left: local.left,
                top: local.top,
                right: local.left + w as f32 / s,
                bottom: local.top + h as f32 / s,
            };
            (key, (w, h), raster, quad, t)
        };
        if size.0 == 0 || size.1 == 0 || size.0 > MAX_RASTER || size.1 > MAX_RASTER {
            return Ok(());
        }
        let (cairo, resources) = (&mut *self.cairo, &mut *self.resources);
        let icon = match command {
            DrawCommand::Vector { icon, .. } => Some(icon),
            _ => None,
        };
        let view = resources
            .textures
            .raster(&mut resources.staging, key, size, icon, || {
                cairo.rasterize(command, size.0, size.1, raster)
            })?;
        self.emit(
            Instance::new(quad, m, SHAPE_NONE, PAINT_TEXTURE),
            Bind::Texture(view),
        );
        Ok(())
    }
}
