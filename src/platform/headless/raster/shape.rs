//! Géométrie de couverture : rectangles arrondis, anneaux, segments épais et polygones
//! testés point par point dans leur repère local.
use crate::ui::scene::{PointF, RectF};

/// Rectangle arrondi (rayons haut-gauche, haut-droit, bas-droit, bas-gauche).
#[derive(Clone, Copy)]
pub(super) struct Rounded {
    pub(super) rect: RectF,
    pub(super) radii: [f32; 4],
}
impl Rounded {
    pub(super) fn new(rect: RectF, radii: [f32; 4]) -> Self {
        let limit = ((rect.right - rect.left).min(rect.bottom - rect.top) / 2.).max(0.);
        Self {
            rect,
            radii: radii.map(|r| r.clamp(0., limit)),
        }
    }
    pub(super) fn inset(self, amount: f32) -> Self {
        Self::new(
            RectF {
                left: self.rect.left + amount,
                top: self.rect.top + amount,
                right: self.rect.right - amount,
                bottom: self.rect.bottom - amount,
            },
            self.radii.map(|r| r - amount),
        )
    }
    pub(super) fn contains(&self, p: PointF) -> bool {
        let r = self.rect;
        if p.x < r.left || p.x >= r.right || p.y < r.top || p.y >= r.bottom {
            return false;
        }
        let corners = [
            (r.left + self.radii[0], r.top + self.radii[0], self.radii[0]),
            (
                r.right - self.radii[1],
                r.top + self.radii[1],
                self.radii[1],
            ),
            (
                r.right - self.radii[2],
                r.bottom - self.radii[2],
                self.radii[2],
            ),
            (
                r.left + self.radii[3],
                r.bottom - self.radii[3],
                self.radii[3],
            ),
        ];
        corners
            .iter()
            .enumerate()
            .all(|(index, &(cx, cy, radius))| {
                let outside_x = if index == 0 || index == 3 {
                    p.x < cx
                } else {
                    p.x > cx
                };
                let outside_y = if index < 2 { p.y < cy } else { p.y > cy };
                !(outside_x && outside_y) || (p.x - cx).hypot(p.y - cy) <= radius
            })
    }
}

pub(super) enum Shape<'a> {
    Rounded(Rounded),
    Ring {
        outer: Rounded,
        inner: Rounded,
    },
    Segment {
        start: PointF,
        end: PointF,
        half: f32,
    },
    Polygon(&'a [PointF]),
}
impl Shape<'_> {
    pub(super) fn contains(&self, p: PointF) -> bool {
        match self {
            Self::Rounded(shape) => shape.contains(p),
            Self::Ring { outer, inner } => outer.contains(p) && !inner.contains(p),
            Self::Segment { start, end, half } => {
                let (dx, dy) = (end.x - start.x, end.y - start.y);
                let length = dx * dx + dy * dy;
                let t = if length > 0. {
                    (((p.x - start.x) * dx + (p.y - start.y) * dy) / length).clamp(0., 1.)
                } else {
                    0.
                };
                (p.x - start.x - t * dx).hypot(p.y - start.y - t * dy) <= *half
            }
            Self::Polygon(points) => {
                // Règle pair-impair.
                let mut inside = false;
                let mut previous = points[points.len() - 1];
                for &point in points.iter() {
                    if (point.y > p.y) != (previous.y > p.y)
                        && p.x
                            < (previous.x - point.x) * (p.y - point.y) / (previous.y - point.y)
                                + point.x
                    {
                        inside = !inside;
                    }
                    previous = point;
                }
                inside
            }
        }
    }
    pub(super) fn bounds(&self) -> RectF {
        match self {
            Self::Rounded(shape) | Self::Ring { outer: shape, .. } => shape.rect,
            Self::Segment { start, end, half } => RectF {
                left: start.x.min(end.x) - half,
                top: start.y.min(end.y) - half,
                right: start.x.max(end.x) + half,
                bottom: start.y.max(end.y) + half,
            },
            Self::Polygon(points) => RectF {
                left: points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
                top: points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
                right: points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
                bottom: points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
            },
        }
    }
}

pub(super) fn intersect(a: RectF, b: RectF) -> RectF {
    RectF {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    }
}
