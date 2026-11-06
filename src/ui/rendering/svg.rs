//! Inline SVG compiled at build time.
//!
//! The template compiler turns `<svg>` markup into a static [`SvgImage`]: every shape
//! (`path`, `circle`, `rect`…) becomes a list of absolute [`PathOp`]s in viewBox
//! units, with `g` transforms, arcs and quadratic curves already converted to cubic
//! Béziers, and presentation attributes (`fill`, `stroke`, `stroke-width`…) resolved
//! through inheritance. Nothing is parsed at runtime.
use crate::ui::{
    rendering::transform::Affine,
    scene::{PointF, RectF},
    theme::Color,
};

/// Paint of a fill or stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgPaint {
    /// Nothing is painted (`none`).
    None,
    /// The element's CSS `color` (`currentColor`), resolved when painting.
    CurrentColor,
    /// A constant colour.
    Color(Color),
}
impl SvgPaint {
    /// Colour to paint with, given the current colour and an opacity factor; `None` when
    /// nothing is visible.
    pub fn resolve(self, current: Color, opacity: f32) -> Option<Color> {
        let color = match self {
            Self::None => return None,
            Self::CurrentColor => current,
            Self::Color(color) => color,
        };
        let alpha = (f32::from(color.alpha) * opacity.clamp(0., 1.)).round() as u8;
        (alpha > 0).then_some(Color { alpha, ..color })
    }
}
/// Shape of the ends of open sub-paths (`stroke-linecap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineCap {
    /// Ends exactly at the end point.
    Butt,
    /// Half-disc around the end point.
    Round,
    /// Half-square around the end point.
    Square,
}
/// Shape of the corners between segments (`stroke-linejoin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineJoin {
    /// Sharp corner, bevelled beyond the miter limit.
    Miter,
    /// Rounded corner.
    Round,
    /// Cut corner.
    Bevel,
}
/// Rule deciding which regions of overlapping contours are inside (`fill-rule`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Inside when the winding number is not zero (SVG default).
    NonZero,
    /// Inside when an odd number of contours surround the point.
    EvenOdd,
}
/// One command of a compiled path, in absolute viewBox coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathOp {
    /// Start a new sub-path at (x, y).
    MoveTo(f32, f32),
    /// Straight line to (x, y).
    LineTo(f32, f32),
    /// Cubic Bézier: first control point, second control point, end point.
    CubicTo(f32, f32, f32, f32, f32, f32),
    /// Close the current sub-path with a straight line to its start.
    Close,
}
/// One painted shape of an [`SvgImage`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgShape {
    /// Outline of the shape; every sub-path starts with [`PathOp::MoveTo`].
    pub ops: &'static [PathOp],
    /// Interior paint.
    pub fill: SvgPaint,
    /// Outline paint.
    pub stroke: SvgPaint,
    /// Outline width in viewBox units.
    pub stroke_width: f32,
    /// Ends of open sub-paths.
    pub line_cap: LineCap,
    /// Corners between segments.
    pub line_join: LineJoin,
    /// Limit of the miter length, as a multiple of the stroke width (at least 1).
    pub miter_limit: f32,
    /// Inside test of the fill.
    pub fill_rule: FillRule,
    /// Opacity of the fill (`fill-opacity` × `opacity`), from 0 to 1.
    pub fill_opacity: f32,
    /// Opacity of the stroke (`stroke-opacity` × `opacity`), from 0 to 1.
    pub stroke_opacity: f32,
}
/// A compiled SVG drawing: its viewBox and shapes, painted in order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgImage {
    /// `min-x`, `min-y`, `width` and `height` of the viewBox.
    pub view_box: [f32; 4],
    /// Scale the drawing uniformly and centre it (`xMidYMid meet`, the default); `false`
    /// stretches it to the box (`preserveAspectRatio="none"`).
    pub preserve_aspect_ratio: bool,
    /// Shapes in painting order.
    pub shapes: &'static [SvgShape],
}
impl SvgImage {
    /// Check the numeric invariants of the drawing.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) for a non-finite or empty
    /// viewBox, a non-finite coordinate, a negative stroke width, a miter limit below 1,
    /// an opacity outside 0..=1 or a sub-path that does not start with `MoveTo`.
    pub fn validate(&self) -> crate::Result<()> {
        let [x, y, w, h] = self.view_box;
        if ![x, y, w, h].iter().all(|n| n.is_finite()) || w <= 0. || h <= 0. {
            return Err(crate::Error::invalid_input("invalid SVG viewBox"));
        }
        for shape in self.shapes {
            if !shape.stroke_width.is_finite()
                || shape.stroke_width < 0.
                || !shape.miter_limit.is_finite()
                || shape.miter_limit < 1.
                || !(0. ..=1.).contains(&shape.fill_opacity)
                || !(0. ..=1.).contains(&shape.stroke_opacity)
            {
                return Err(crate::Error::invalid_input("invalid SVG paint parameters"));
            }
            let mut open = false;
            for op in shape.ops {
                let finite = match *op {
                    PathOp::MoveTo(x, y) => {
                        open = true;
                        x.is_finite() && y.is_finite()
                    }
                    PathOp::LineTo(x, y) => open && x.is_finite() && y.is_finite(),
                    PathOp::CubicTo(a, b, c, d, e, f) => {
                        open && [a, b, c, d, e, f].iter().all(|n| n.is_finite())
                    }
                    PathOp::Close => open,
                };
                if !finite {
                    return Err(crate::Error::invalid_input("invalid SVG path"));
                }
            }
        }
        Ok(())
    }
    /// Transform from viewBox units to `rect`, honouring `preserveAspectRatio`.
    pub fn placement(&self, rect: RectF) -> Affine {
        let [x, y, w, h] = self.view_box;
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        let (sx, sy) = (width / w, height / h);
        let (sx, sy, ox, oy) = if self.preserve_aspect_ratio {
            let s = sx.min(sy);
            (s, s, (width - w * s) / 2., (height - h * s) / 2.)
        } else {
            (sx, sy, 0., 0.)
        };
        Affine {
            m11: sx,
            m12: 0.,
            m21: 0.,
            m22: sy,
            dx: rect.left + ox - x * sx,
            dy: rect.top + oy - y * sy,
        }
    }
}
/// A sub-path flattened into straight segments.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    /// Vertices in drawing order (the closing segment is implied when `closed`).
    pub points: Vec<PointF>,
    /// The sub-path ends with [`PathOp::Close`].
    pub closed: bool,
}
/// Flatten `ops` into polylines after applying `transform`; curves are split into
/// segments shorter than about `tolerance` units of the destination space.
pub fn flatten(ops: &[PathOp], transform: Affine, tolerance: f32) -> Vec<Polyline> {
    let tolerance = tolerance.max(0.01);
    let mut out: Vec<Polyline> = Vec::new();
    let mut current: Option<Polyline> = None;
    let mut last = PointF { x: 0., y: 0. };
    let mut start = last;
    let finish = |current: &mut Option<Polyline>, out: &mut Vec<Polyline>| {
        if let Some(line) = current.take()
            && !line.points.is_empty()
        {
            out.push(line);
        }
    };
    for op in ops {
        match *op {
            PathOp::MoveTo(x, y) => {
                finish(&mut current, &mut out);
                last = PointF { x, y };
                start = last;
                current = Some(Polyline {
                    points: vec![transform.point(last)],
                    closed: false,
                });
            }
            PathOp::LineTo(x, y) => {
                last = PointF { x, y };
                if let Some(line) = &mut current {
                    line.points.push(transform.point(last));
                }
            }
            PathOp::CubicTo(x1, y1, x2, y2, x, y) => {
                let p0 = transform.point(last);
                let p1 = transform.point(PointF { x: x1, y: y1 });
                let p2 = transform.point(PointF { x: x2, y: y2 });
                last = PointF { x, y };
                let p3 = transform.point(last);
                let hull = (p1.x - p0.x).hypot(p1.y - p0.y)
                    + (p2.x - p1.x).hypot(p2.y - p1.y)
                    + (p3.x - p2.x).hypot(p3.y - p2.y);
                let steps = ((hull / tolerance).sqrt() * 2.).ceil().clamp(1., 256.) as u32;
                if let Some(line) = &mut current {
                    for i in 1..=steps {
                        let t = i as f32 / steps as f32;
                        let u = 1. - t;
                        let (a, b, c, d) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
                        line.points.push(PointF {
                            x: a * p0.x + b * p1.x + c * p2.x + d * p3.x,
                            y: a * p0.y + b * p1.y + c * p2.y + d * p3.y,
                        });
                    }
                }
            }
            PathOp::Close => {
                if let Some(mut line) = current.take() {
                    line.closed = true;
                    out.push(line);
                }
                // Un tracé qui continue après Z repart du début du sous-chemin.
                last = start;
                current = Some(Polyline {
                    points: vec![transform.point(start)],
                    closed: false,
                });
            }
        }
    }
    finish(&mut current, &mut out);
    // Un sous-chemin réduit à un point (après Z) n'est ni rempli ni tracé.
    out.retain(|line| line.points.len() > 1);
    out
}
/// Signed winding number of `p` against the closed polylines (for the fill).
pub fn winding(lines: &[Polyline], p: PointF) -> i32 {
    let mut winding = 0;
    for line in lines {
        let n = line.points.len();
        for i in 0..n {
            let a = line.points[i];
            let b = line.points[(i + 1) % n];
            if a.y <= p.y {
                if b.y > p.y && (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y) > 0. {
                    winding += 1;
                }
            } else if b.y <= p.y && (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y) < 0. {
                winding -= 1;
            }
        }
    }
    winding
}
/// Whether `p` lies on the stroke of `lines` with half-width `half`. Joins are rounded
/// (miter and bevel joins are approximated by round ones); caps follow `cap`.
pub fn on_stroke(lines: &[Polyline], p: PointF, half: f32, cap: LineCap) -> bool {
    for line in lines {
        let points = &line.points;
        let n = points.len();
        let segments = if line.closed { n } else { n - 1 };
        for i in 0..segments {
            let a = points[i];
            let b = points[(i + 1) % n];
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let length2 = dx * dx + dy * dy;
            let first = !line.closed && i == 0;
            let last = !line.closed && i + 1 == segments;
            if length2 <= f32::EPSILON {
                // Segment nul : seuls les bouts ronds ou carrés le rendent visible.
                let near = match cap {
                    LineCap::Butt => false,
                    LineCap::Round => (p.x - a.x).hypot(p.y - a.y) <= half,
                    LineCap::Square => (p.x - a.x).abs() <= half && (p.y - a.y).abs() <= half,
                };
                if near && n <= 2 {
                    return true;
                }
                continue;
            }
            let length = length2.sqrt();
            let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / length2;
            let distance = ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / length;
            let extend = if cap == LineCap::Square {
                half / length
            } else {
                0.
            };
            let low = if first { -extend } else { 0. };
            let high = if last { 1. + extend } else { 1. };
            if (low..=high).contains(&t) && distance <= half {
                return true;
            }
            // Jonctions rondes entre segments, bouts ronds aux extrémités ouvertes.
            let round_start = !first || cap == LineCap::Round;
            let round_end = !last || cap == LineCap::Round;
            if (round_start && (p.x - a.x).hypot(p.y - a.y) <= half)
                || (round_end && (p.x - b.x).hypot(p.y - b.y) <= half)
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    const SQUARE: [PathOp; 5] = [
        PathOp::MoveTo(0., 0.),
        PathOp::LineTo(10., 0.),
        PathOp::LineTo(10., 10.),
        PathOp::LineTo(0., 10.),
        PathOp::Close,
    ];
    #[test]
    fn placement_centres_and_scales_the_view_box_uniformly() {
        let image = SvgImage {
            view_box: [0., 0., 24., 12.],
            preserve_aspect_ratio: true,
            shapes: &[],
        };
        let a = image.placement(RectF::new(10., 10., 48., 48.));
        let p = a.point(PointF { x: 0., y: 0. });
        assert_eq!((p.x, p.y), (10., 22.));
        let q = a.point(PointF { x: 24., y: 12. });
        assert_eq!((q.x, q.y), (58., 46.));
        let stretched = SvgImage {
            preserve_aspect_ratio: false,
            ..image
        }
        .placement(RectF::new(0., 0., 48., 48.));
        assert_eq!(stretched.point(PointF { x: 24., y: 12. }).y, 48.);
    }
    #[test]
    fn flattening_keeps_open_and_closed_sub_paths() {
        let lines = flatten(
            &[
                PathOp::MoveTo(0., 0.),
                PathOp::CubicTo(0., 10., 10., 10., 10., 0.),
                PathOp::MoveTo(20., 0.),
                PathOp::LineTo(30., 0.),
                PathOp::LineTo(30., 10.),
                PathOp::Close,
            ],
            Affine::IDENTITY,
            0.25,
        );
        assert_eq!(lines.len(), 2);
        assert!(!lines[0].closed && lines[0].points.len() > 4);
        let end = lines[0].points.last().unwrap();
        assert!((end.x - 10.).abs() < 1e-4 && end.y.abs() < 1e-4);
        assert!(lines[1].closed);
    }
    #[test]
    fn winding_and_stroke_tests_follow_svg_rules() {
        let lines = flatten(&SQUARE, Affine::IDENTITY, 0.25);
        assert_eq!(winding(&lines, PointF { x: 5., y: 5. }).abs(), 1);
        assert_eq!(winding(&lines, PointF { x: 15., y: 5. }), 0);
        assert!(on_stroke(
            &lines,
            PointF { x: 5., y: 0.5 },
            1.,
            LineCap::Butt
        ));
        assert!(!on_stroke(
            &lines,
            PointF { x: 5., y: 5. },
            1.,
            LineCap::Butt
        ));
        let open = flatten(
            &[PathOp::MoveTo(0., 0.), PathOp::LineTo(10., 0.)],
            Affine::IDENTITY,
            0.25,
        );
        let beyond = PointF { x: 10.5, y: 0. };
        assert!(!on_stroke(&open, beyond, 1., LineCap::Butt));
        assert!(on_stroke(&open, beyond, 1., LineCap::Round));
        assert!(on_stroke(&open, beyond, 1., LineCap::Square));
    }
    #[test]
    fn validation_rejects_broken_drawings() {
        static BAD: [SvgShape; 1] = [SvgShape {
            ops: &[PathOp::LineTo(1., 1.)],
            fill: SvgPaint::CurrentColor,
            stroke: SvgPaint::None,
            stroke_width: 1.,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 4.,
            fill_rule: FillRule::NonZero,
            fill_opacity: 1.,
            stroke_opacity: 1.,
        }];
        let image = SvgImage {
            view_box: [0., 0., 24., 24.],
            preserve_aspect_ratio: true,
            shapes: &BAD,
        };
        assert!(image.validate().is_err());
        assert!(
            SvgImage {
                view_box: [0., 0., 0., 24.],
                shapes: &[],
                ..image
            }
            .validate()
            .is_err()
        );
        assert_eq!(
            SvgPaint::CurrentColor.resolve(Color::rgb(10, 20, 30), 0.5),
            Some(Color::rgba(10, 20, 30, 128))
        );
    }
}
