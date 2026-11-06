//! Géométrie commune des renderers : rectangles arrondis normalisés comme sous Direct2D
//! et extrémités des dégradés linéaires.
use crate::ui::{
    rendering::brush::LinearGradient,
    scene::{PointF, RectF},
};

/// Rectangle et rayons normalisés comme le backend Windows : un trait est intérieur
/// (rectangle rentré de la moitié de son épaisseur), les rayons trop grands sont réduits
/// proportionnellement (CSS Backgrounds §5.5).
pub(in crate::platform) fn normalized(
    mut rect: RectF,
    mut radii: [f32; 4],
    stroke: Option<f32>,
) -> (RectF, [f32; 4]) {
    if let Some(width) = stroke {
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
    (rect, radii.map(|r| r * factor))
}
/// Extrémités d'un dégradé linéaire sur `rect`, calculées comme le pinceau Direct2D.
pub(in crate::platform) fn linear_points(
    gradient: &LinearGradient,
    rect: RectF,
) -> (PointF, PointF) {
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    let (s, c) = gradient
        .angle_in(w, h)
        .rem_euclid(360.)
        .to_radians()
        .sin_cos();
    let extent = (w * c.abs() + h * s.abs()) * 0.5;
    let (x, y) = (
        (rect.left + rect.right) * 0.5,
        (rect.top + rect.bottom) * 0.5,
    );
    (
        PointF {
            x: x - c * extent,
            y: y - s * extent,
        },
        PointF {
            x: x + c * extent,
            y: y + s * extent,
        },
    )
}
