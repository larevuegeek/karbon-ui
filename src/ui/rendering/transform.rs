//! 2D affine matrix in logical coordinates, compatible with Direct2D.
use crate::ui::core::geometry::{PointF, RectF};
/// 2D affine transform using Direct2D's row-vector convention:
/// `x' = x·m11 + y·m21 + dx`, `y' = x·m12 + y·m22 + dy`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// X scale / rotation term applied to `x`.
    pub m11: f32,
    /// Y shear / rotation term applied to `x`.
    pub m12: f32,
    /// X shear / rotation term applied to `y`.
    pub m21: f32,
    /// Y scale / rotation term applied to `y`.
    pub m22: f32,
    /// Horizontal translation.
    pub dx: f32,
    /// Vertical translation.
    pub dy: f32,
}
impl Affine {
    /// Transform that leaves every point unchanged.
    pub const IDENTITY: Self = Self {
        m11: 1.,
        m12: 0.,
        m21: 0.,
        m22: 1.,
        dx: 0.,
        dy: 0.,
    };
    /// Pure translation by (`x`, `y`).
    pub const fn translation(x: f32, y: f32) -> Self {
        Self {
            dx: x,
            dy: y,
            ..Self::IDENTITY
        }
    }
    /// Transform a point.
    pub fn point(self, p: PointF) -> PointF {
        PointF {
            x: p.x * self.m11 + p.y * self.m21 + self.dx,
            y: p.x * self.m12 + p.y * self.m22 + self.dy,
        }
    }
    /// Apply `self`, then `next` (Direct2D row-vector convention).
    pub fn then(self, next: Self) -> Self {
        Self {
            m11: self.m11 * next.m11 + self.m12 * next.m21,
            m12: self.m11 * next.m12 + self.m12 * next.m22,
            m21: self.m21 * next.m11 + self.m22 * next.m21,
            m22: self.m21 * next.m12 + self.m22 * next.m22,
            dx: self.dx * next.m11 + self.dy * next.m21 + next.dx,
            dy: self.dx * next.m12 + self.dy * next.m22 + next.dy,
        }
    }
    /// Inverse transform, or `None` if the matrix is singular or not finite.
    pub fn inverse(self) -> Option<Self> {
        let d = self.m11 * self.m22 - self.m12 * self.m21;
        if !d.is_finite() || d.abs() < 1e-8 {
            return None;
        }
        let m = Self {
            m11: self.m22 / d,
            m12: -self.m12 / d,
            m21: -self.m21 / d,
            m22: self.m11 / d,
            dx: 0.,
            dy: 0.,
        };
        let p = m.point(PointF {
            x: -self.dx,
            y: -self.dy,
        });
        Some(Self {
            dx: p.x,
            dy: p.y,
            ..m
        })
    }
    /// Axis-aligned bounding box of `r` after transformation.
    pub fn bounds(self, r: RectF) -> RectF {
        let points = [
            self.point(PointF {
                x: r.left,
                y: r.top,
            }),
            self.point(PointF {
                x: r.right,
                y: r.top,
            }),
            self.point(PointF {
                x: r.left,
                y: r.bottom,
            }),
            self.point(PointF {
                x: r.right,
                y: r.bottom,
            }),
        ];
        RectF {
            left: points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
            top: points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
            right: points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
            bottom: points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
        }
    }
    /// `true` if all six coefficients are finite.
    pub fn is_finite(self) -> bool {
        [self.m11, self.m12, self.m21, self.m22, self.dx, self.dy]
            .iter()
            .all(|v| v.is_finite())
    }
}
