//! Chemins Cairo : rectangles arrondis à coins indépendants, polylignes, icônes
//! vectorielles et formes SVG compilées, ajoutés au chemin courant (coordonnées logiques).
use crate::{
    platform::linux::ffi::cairo::Context,
    ui::{
        rendering::{
            svg::{PathOp, SvgShape},
            vector::{VectorIcon, VectorSegment},
        },
        scene::{PointF, RectF},
    },
};

const fn p(x: f32, y: f32) -> PointF {
    PointF { x, y }
}
fn move_to(c: &Context, at: PointF) {
    c.move_to(at.x.into(), at.y.into());
}
fn line_to(c: &Context, at: PointF) {
    c.line_to(at.x.into(), at.y.into());
}
fn curve_to(c: &Context, c1: PointF, c2: PointF, end: PointF) {
    c.curve_to(
        c1.x.into(),
        c1.y.into(),
        c2.x.into(),
        c2.y.into(),
        end.x.into(),
        end.y.into(),
    );
}
/// Rectangle sans arrondi.
pub(super) fn rectangle(c: &Context, r: RectF) {
    c.rectangle(
        r.left.into(),
        r.top.into(),
        (r.right - r.left).into(),
        (r.bottom - r.top).into(),
    );
}
/// Rectangle arrondi (haut-gauche, haut-droit, bas-droit, bas-gauche) en courbes cubiques.
pub(super) fn rounded(c: &Context, rect: RectF, radii: [f32; 4]) {
    let (l, t, r, b) = (rect.left, rect.top, rect.right, rect.bottom);
    let [tl, tr, br, bl] = radii;
    let k = 0.552_284_8;
    move_to(c, p(l + tl, t));
    line_to(c, p(r - tr, t));
    curve_to(
        c,
        p(r - tr + k * tr, t),
        p(r, t + tr - k * tr),
        p(r, t + tr),
    );
    line_to(c, p(r, b - br));
    curve_to(
        c,
        p(r, b - br + k * br),
        p(r - br + k * br, b),
        p(r - br, b),
    );
    line_to(c, p(l + bl, b));
    curve_to(
        c,
        p(l + bl - k * bl, b),
        p(l, b - bl + k * bl),
        p(l, b - bl),
    );
    line_to(c, p(l, t + tl));
    curve_to(
        c,
        p(l, t + tl - k * tl),
        p(l + tl - k * tl, t),
        p(l + tl, t),
    );
    c.close_path();
}
/// Rectangle à rayon unique, réduit à la moitié du plus petit côté (comme Direct2D).
pub(super) fn round_rect(c: &Context, rect: RectF, radius: f32) {
    let w = (rect.right - rect.left).max(0.);
    let h = (rect.bottom - rect.top).max(0.);
    let r = radius.max(0.).min(w / 2.).min(h / 2.);
    if r > 0. {
        rounded(c, rect, [r; 4]);
    } else {
        rectangle(c, rect);
    }
}
/// Polyligne ouverte ou polygone fermé.
pub(super) fn polyline(c: &Context, points: &[PointF], closed: bool) {
    move_to(c, points[0]);
    for point in &points[1..] {
        line_to(c, *point);
    }
    if closed {
        c.close_path();
    }
}
/// Contours d'une icône mis à l'échelle de `rect`.
pub(super) fn icon(c: &Context, rect: RectF, icon: &VectorIcon) {
    let place = |q: PointF| {
        p(
            rect.left + q.x / icon.width * (rect.right - rect.left),
            rect.top + q.y / icon.height * (rect.bottom - rect.top),
        )
    };
    for contour in icon.contours.iter().filter(|c| c.segments.len() >= 2) {
        move_to(c, place(contour.start));
        for segment in &contour.segments {
            match segment {
                VectorSegment::Line(end) => line_to(c, place(*end)),
                VectorSegment::Curve {
                    control1,
                    control2,
                    end,
                } => curve_to(c, place(*control1), place(*control2), place(*end)),
            }
        }
        c.close_path();
    }
}
/// Contour d'une forme SVG en unités de la viewBox (la canvas applique le placement,
/// pour que l'épaisseur des traits suive l'échelle comme sous Direct2D).
pub(super) fn svg(c: &Context, shape: &SvgShape) {
    for op in shape.ops {
        match *op {
            PathOp::MoveTo(x, y) => move_to(c, p(x, y)),
            PathOp::LineTo(x, y) => line_to(c, p(x, y)),
            PathOp::CubicTo(a, b, cx, d, e, f) => curve_to(c, p(a, b), p(cx, d), p(e, f)),
            PathOp::Close => c.close_path(),
        }
    }
}
