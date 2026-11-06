//! Cached cubic paths for independently rounded corners, in logical coordinates.
use super::*;
use std::collections::HashSet;
type Key = [u32; 8];
fn key(mut rect: RectF, mut radii: [f32; 4], stroke: Option<(f32, Color)>) -> Key {
    if let Some((width, _)) = stroke {
        let inset = width
            .min(rect.right - rect.left)
            .min(rect.bottom - rect.top)
            / 2.;
        rect.left += inset;
        rect.top += inset;
        rect.right -= inset;
        rect.bottom -= inset;
        for r in &mut radii {
            *r = (*r - inset).max(0.);
        }
    }
    let w = (rect.right - rect.left).max(0.);
    let h = (rect.bottom - rect.top).max(0.);
    let mut factor = 1f32;
    for (size, sum) in [
        (w, radii[0] + radii[1]),
        (w, radii[3] + radii[2]),
        (h, radii[0] + radii[3]),
        (h, radii[1] + radii[2]),
    ] {
        if sum > 0. {
            factor = factor.min(size / sum);
        }
    }
    for r in &mut radii {
        *r *= factor;
    }
    [
        rect.left.to_bits(),
        rect.top.to_bits(),
        rect.right.to_bits(),
        rect.bottom.to_bits(),
        radii[0].to_bits(),
        radii[1].to_bits(),
        radii[2].to_bits(),
        radii[3].to_bits(),
    ]
}
#[derive(Default)]
pub(super) struct Cache {
    paths: HashMap<Key, ID2D1PathGeometry>,
}
impl Cache {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.paths.len()
    }
    pub fn prepare(&mut self, factory: &ID2D1Factory, scene: &Scene) -> crate::Result<()> {
        let mut used = HashSet::new();
        for c in &scene.commands {
            let (rect, radii, stroke) = match c {
                DrawCommand::Shape {
                    rect,
                    radii,
                    stroke,
                    ..
                } => (*rect, *radii, *stroke),
                DrawCommand::PushShapeClip { rect, radii }
                | DrawCommand::Backdrop { rect, radii, .. } => (*rect, *radii, None),
                _ => continue,
            };
            let key = key(rect, radii, stroke);
            used.insert(key);
            if let std::collections::hash_map::Entry::Vacant(entry) = self.paths.entry(key) {
                entry.insert(create(factory, key)?);
            }
        }
        self.paths.retain(|k, _| used.contains(k));
        Ok(())
    }
    pub fn geometry(&self, rect: RectF, radii: [f32; 4]) -> &ID2D1PathGeometry {
        &self.paths[&key(rect, radii, None)]
    }
    pub fn draw(
        &self,
        target: &ID2D1RenderTarget,
        brush: &ID2D1Brush,
        rect: RectF,
        radii: [f32; 4],
        stroke: Option<(f32, Color)>,
    ) {
        let geometry = &self.paths[&key(rect, radii, stroke)];
        // SAFETY: passe de dessin ouverte ; géométrie du cache et pinceau vivants.
        unsafe {
            if let Some((width, _)) = stroke {
                target.DrawGeometry(
                    geometry,
                    brush,
                    width
                        .min(rect.right - rect.left)
                        .min(rect.bottom - rect.top),
                    None,
                );
            } else {
                target.FillGeometry(geometry, brush, None);
            }
        }
    }
}
fn create(factory: &ID2D1Factory, key: Key) -> crate::Result<ID2D1PathGeometry> {
    let [l, t, r, b, tl, tr, br, bl] = key.map(f32::from_bits);
    let k = 0.552_284_8;
    let (geometry, sink) = open_path(factory, "Create rounded shape", "Open rounded shape")?;
    let point = |x, y| Vector2 { X: x, Y: y };
    let corners = [
        (
            point(r - tr, t),
            D2D1_BEZIER_SEGMENT {
                point1: point(r - tr + k * tr, t),
                point2: point(r, t + tr - k * tr),
                point3: point(r, t + tr),
            },
        ),
        (
            point(r, b - br),
            D2D1_BEZIER_SEGMENT {
                point1: point(r, b - br + k * br),
                point2: point(r - br + k * br, b),
                point3: point(r - br, b),
            },
        ),
        (
            point(l + bl, b),
            D2D1_BEZIER_SEGMENT {
                point1: point(l + bl - k * bl, b),
                point2: point(l, b - bl + k * bl),
                point3: point(l, b - bl),
            },
        ),
        (
            point(l, t + tl),
            D2D1_BEZIER_SEGMENT {
                point1: point(l, t + tl - k * tl),
                point2: point(l + tl - k * tl, t),
                point3: point(l + tl, t),
            },
        ),
    ];
    // SAFETY: puits ouvert et non encore fermé ; segments copiés pendant chaque appel.
    unsafe {
        sink.BeginFigure(point(l + tl, t), D2D1_FIGURE_BEGIN_FILLED);
        for (p, c) in &corners {
            sink.AddLines(std::slice::from_ref(p));
            sink.AddBeziers(std::slice::from_ref(c));
        }
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    }
    // SAFETY: ferme le puits une seule fois.
    unsafe { sink.Close() }.check("Close rounded shape")?;
    Ok(geometry)
}
