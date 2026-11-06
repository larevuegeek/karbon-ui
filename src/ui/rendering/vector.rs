//! Vector silhouettes made of several contours (holes stay transparent).
use crate::ui::scene::PointF;
/// One segment of a [`VectorContour`], continuing from the previous point.
pub enum VectorSegment {
    /// Straight line to the given point.
    Line(PointF),
    /// Cubic Bézier curve.
    Curve {
        /// First control point.
        control1: PointF,
        /// Second control point.
        control2: PointF,
        /// End point of the curve.
        end: PointF,
    },
}
/// Closed contour: a start point followed by segments.
pub struct VectorContour {
    /// First point of the contour.
    pub start: PointF,
    /// Segments drawn in order from `start`; the contour is closed automatically
    /// and skipped when it has fewer than two segments.
    pub segments: Vec<VectorSegment>,
}
/// Vector icon in its own coordinate space, scaled to the destination rectangle when drawn.
///
/// Overlapping contours create holes, which stay transparent.
pub struct VectorIcon {
    /// Width of the icon's coordinate space; must be finite and positive.
    pub width: f32,
    /// Height of the icon's coordinate space; must be finite and positive.
    pub height: f32,
    /// Contours forming the silhouette.
    pub contours: Vec<VectorContour>,
}
