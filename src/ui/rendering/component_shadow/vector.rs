//! Ombres sans flou dessinées en vectoriel : anneaux et bandes, sans image.
use super::{BoxShadow, Frame, Paint, normalized};
use crate::ui::{layout, scene::RectF};
/// Shadow without blur as vector frames, in logical units (`s` already scaled), or
/// `None` when it needs an image: a ring (`0 0 0 n`) on any shape, any offset and
/// spread on a square-cornered box.
pub(super) fn vector(w: f32, h: f32, radii: [f32; 4], s: BoxShadow) -> Option<Vec<Frame>> {
    if s.blur != 0. || s.color.alpha == 0 {
        return (s.blur == 0.).then(Vec::new);
    }
    let frame = |paint, x, y, width, height| Frame {
        layer: 0,
        paint,
        x,
        y,
        width,
        height,
        inset: s.inset,
    };
    let radii = normalized(w, h, radii);
    // Anneau : le trait d'une forme est tracé à l'intérieur de son rectangle (comme une
    // bordure CSS). Intérieur : sur la boîte elle-même ; extérieur : sur la boîte agrandie
    // de `spread`, rayons agrandis d'autant.
    if s.x == 0. && s.y == 0. && s.spread > 0. {
        if s.inset {
            if w <= 2. * s.spread || h <= 2. * s.spread {
                return None;
            }
            return Some(vec![frame(
                Paint::Ring {
                    radii,
                    width: s.spread,
                    color: s.color,
                },
                0.,
                0.,
                w,
                h,
            )]);
        }
        let outer = normalized(
            w + 2. * s.spread,
            h + 2. * s.spread,
            radii.map(|r| if r > 0. { r + s.spread } else { 0. }),
        );
        return Some(vec![frame(
            Paint::Ring {
                radii: outer,
                width: s.spread,
                color: s.color,
            },
            -s.spread,
            -s.spread,
            w + 2. * s.spread,
            h + 2. * s.spread,
        )]);
    }
    if radii.iter().any(|r| *r > 0.) {
        return None;
    }
    // Square corners: the shadow is the rectangle `a` minus the rectangle `b`.
    let (a, b) = if s.inset {
        (
            RectF::new(0., 0., w, h),
            RectF::new(
                s.x + s.spread,
                s.y + s.spread,
                (w - 2. * s.spread).max(0.),
                (h - 2. * s.spread).max(0.),
            ),
        )
    } else {
        (
            RectF::new(
                s.x - s.spread,
                s.y - s.spread,
                (w + 2. * s.spread).max(0.),
                (h + 2. * s.spread).max(0.),
            ),
            RectF::new(0., 0., w, h),
        )
    };
    let b = layout::intersect(a, b);
    let mut bands = Vec::new();
    let mut band = |left: f32, top: f32, right: f32, bottom: f32| {
        if right > left && bottom > top {
            bands.push(frame(
                Paint::Fill(s.color),
                left,
                top,
                right - left,
                bottom - top,
            ));
        }
    };
    if layout::width(b) <= 0. || layout::height(b) <= 0. {
        band(a.left, a.top, a.right, a.bottom);
    } else {
        band(a.left, a.top, a.right, b.top);
        band(a.left, b.bottom, a.right, a.bottom);
        band(a.left, b.top, b.left, b.bottom);
        band(b.right, b.top, a.right, b.bottom);
    }
    Some(bands)
}
