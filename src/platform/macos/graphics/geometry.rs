//! Chemins Core Graphics : rectangles arrondis à coins indépendants, polylignes, icônes
//! vectorielles et formes SVG compilées, en coordonnées logiques.
use crate::ui::{
    rendering::{
        svg::{PathOp, SvgShape},
        vector::{VectorIcon, VectorSegment},
    },
    scene::{PointF, RectF},
};
use objc2_core_foundation::{CFRetained, CGFloat};
use objc2_core_graphics::CGMutablePath;

pub(super) use crate::platform::shared::normalized;

/// Constructeur de chemin (les appels Core Graphics reçoivent une matrice nulle).
pub(super) struct Path(pub CFRetained<CGMutablePath>);
impl Path {
    pub fn new() -> Self {
        Self(CGMutablePath::new())
    }
    pub fn move_to(&self, p: PointF) {
        // SAFETY : chemin valide ; matrice nulle (identité).
        unsafe {
            CGMutablePath::move_to_point(
                Some(&self.0),
                std::ptr::null(),
                CGFloat::from(p.x),
                CGFloat::from(p.y),
            )
        };
    }
    pub fn line_to(&self, p: PointF) {
        // SAFETY : idem.
        unsafe {
            CGMutablePath::add_line_to_point(
                Some(&self.0),
                std::ptr::null(),
                CGFloat::from(p.x),
                CGFloat::from(p.y),
            )
        };
    }
    pub fn curve_to(&self, c1: PointF, c2: PointF, end: PointF) {
        // SAFETY : idem.
        unsafe {
            CGMutablePath::add_curve_to_point(
                Some(&self.0),
                std::ptr::null(),
                CGFloat::from(c1.x),
                CGFloat::from(c1.y),
                CGFloat::from(c2.x),
                CGFloat::from(c2.y),
                CGFloat::from(end.x),
                CGFloat::from(end.y),
            )
        };
    }
    pub fn close(&self) {
        CGMutablePath::close_subpath(Some(&self.0));
    }
}
const fn p(x: f32, y: f32) -> PointF {
    PointF { x, y }
}
/// Rectangle arrondi (haut-gauche, haut-droit, bas-droit, bas-gauche) en courbes cubiques.
pub(super) fn rounded(rect: RectF, radii: [f32; 4]) -> Path {
    let (l, t, r, b) = (rect.left, rect.top, rect.right, rect.bottom);
    let [tl, tr, br, bl] = radii;
    let k = 0.552_284_8;
    let path = Path::new();
    path.move_to(p(l + tl, t));
    path.line_to(p(r - tr, t));
    path.curve_to(p(r - tr + k * tr, t), p(r, t + tr - k * tr), p(r, t + tr));
    path.line_to(p(r, b - br));
    path.curve_to(p(r, b - br + k * br), p(r - br + k * br, b), p(r - br, b));
    path.line_to(p(l + bl, b));
    path.curve_to(p(l + bl - k * bl, b), p(l, b - bl + k * bl), p(l, b - bl));
    path.line_to(p(l, t + tl));
    path.curve_to(p(l, t + tl - k * tl), p(l + tl - k * tl, t), p(l + tl, t));
    path.close();
    path
}
/// Rectangle à rayon unique, réduit à la moitié du plus petit côté (comme Direct2D).
pub(super) fn round_rect(rect: RectF, radius: f32) -> Path {
    let w = (rect.right - rect.left).max(0.);
    let h = (rect.bottom - rect.top).max(0.);
    let r = radius.max(0.).min(w / 2.).min(h / 2.);
    rounded(rect, [r; 4])
}
/// Polyligne ouverte ou polygone fermé.
pub(super) fn polyline(points: &[PointF], closed: bool) -> Path {
    let path = Path::new();
    path.move_to(points[0]);
    for point in &points[1..] {
        path.line_to(*point);
    }
    if closed {
        path.close();
    }
    path
}
/// Contours d'une icône mis à l'échelle de `rect`.
pub(super) fn icon(rect: RectF, icon: &VectorIcon) -> Path {
    let place = |q: PointF| {
        p(
            rect.left + q.x / icon.width * (rect.right - rect.left),
            rect.top + q.y / icon.height * (rect.bottom - rect.top),
        )
    };
    let path = Path::new();
    for contour in icon.contours.iter().filter(|c| c.segments.len() >= 2) {
        path.move_to(place(contour.start));
        for segment in &contour.segments {
            match segment {
                VectorSegment::Line(end) => path.line_to(place(*end)),
                VectorSegment::Curve {
                    control1,
                    control2,
                    end,
                } => path.curve_to(place(*control1), place(*control2), place(*end)),
            }
        }
        path.close();
    }
    path
}
/// Contour d'une forme SVG en unités de la viewBox (la canvas applique le placement,
/// pour que l'épaisseur des traits suive l'échelle comme sous Direct2D).
pub(super) fn svg(shape: &SvgShape) -> Path {
    let path = Path::new();
    let at = p;
    for op in shape.ops {
        match *op {
            PathOp::MoveTo(x, y) => path.move_to(at(x, y)),
            PathOp::LineTo(x, y) => path.line_to(at(x, y)),
            PathOp::CubicTo(a, b, c, d, e, f) => path.curve_to(at(a, b), at(c, d), at(e, f)),
            PathOp::Close => path.close(),
        }
    }
    path
}
