//! Conversions vers les structures du SDK Direct2D / DirectWrite (couleurs, rectangles,
//! matrices, coins arrondis, retour à la ligne).
use super::*;

pub(super) const PIXEL_FORMAT: D2D1_PIXEL_FORMAT = D2D1_PIXEL_FORMAT {
    format: DXGI_FORMAT_B8G8R8A8_UNORM,
    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
};
// Conversions vers les structures du SDK ; mêmes dispositions mémoire, copies explicites.
pub(super) fn color_f(c: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: c.red as f32 / 255.,
        g: c.green as f32 / 255.,
        b: c.blue as f32 / 255.,
        a: c.alpha as f32 / 255.,
    }
}
pub(super) fn rect_f(r: RectF) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}
pub(super) fn vector(p: PointF) -> Vector2 {
    Vector2 { X: p.x, Y: p.y }
}
pub(super) fn matrix(a: Affine) -> Matrix3x2 {
    Matrix3x2 {
        M11: a.m11,
        M12: a.m12,
        M21: a.m21,
        M22: a.m22,
        M31: a.dx,
        M32: a.dy,
    }
}
pub(super) fn affine(m: Matrix3x2) -> Affine {
    Affine {
        m11: m.M11,
        m12: m.M12,
        m21: m.M21,
        m22: m.M22,
        dx: m.M31,
        dy: m.M32,
    }
}
pub(super) fn rounded(rect: RectF, radius: f32) -> D2D1_ROUNDED_RECT {
    // Direct2D borne radiusX et radiusY chacun sur sa demi-dimension : un rayon trop grand
    // (`border-radius: 9999px`) donnerait une ellipse. CSS réduit les rayons d'un même
    // facteur : un rayon uniforme se borne à la moitié du plus petit côté (pilule).
    let half = (rect.right - rect.left)
        .abs()
        .min((rect.bottom - rect.top).abs())
        / 2.;
    let radius = radius.clamp(0., half);
    D2D1_ROUNDED_RECT {
        rect: rect_f(rect),
        radiusX: radius,
        radiusY: radius,
    }
}
pub(super) fn wrapping(wrap: bool) -> DWRITE_WORD_WRAPPING {
    if wrap {
        DWRITE_WORD_WRAPPING_WRAP
    } else {
        DWRITE_WORD_WRAPPING_NO_WRAP
    }
}
