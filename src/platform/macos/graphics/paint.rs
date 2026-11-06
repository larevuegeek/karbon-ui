//! Remplissages : couleur unie, dégradés linéaires et radiaux (extension des couleurs
//! extrêmes, comme Direct2D en mode CLAMP) et images étirées.
use super::super::ffi::{cg_color, components, point, rect as cg_rect, srgb};
use super::{canvas::Canvas, geometry::Path};
use crate::platform::shared::linear_points;
use crate::ui::{
    rendering::brush::{Background, GradientStop, LinearGradient, RadialGradient},
    scene::{Image, PointF, RectF},
    theme::Color,
};
use objc2_core_foundation::{CFRetained, CGAffineTransform, CGFloat};
use objc2_core_graphics::{CGColorSpace, CGContext, CGGradient, CGGradientDrawingOptions};
use std::sync::Arc;

/// Dégradé Core Graphics des arrêts ; `alpha_only` le construit en gris (masques), la
/// valeur de gris étant l'alpha de chaque arrêt.
pub(super) fn gradient(
    stops: &[GradientStop],
    alpha_only: bool,
) -> crate::Result<CFRetained<CGGradient>> {
    let (space, values): (CFRetained<CGColorSpace>, Vec<CGFloat>) = if alpha_only {
        let space = super::super::ffi::required(
            CGColorSpace::new_device_gray(),
            "CGColorSpaceCreateDeviceGray",
        )?;
        let values = stops
            .iter()
            .flat_map(|s| [CGFloat::from(s.color.alpha) / 255., 1.])
            .collect();
        (space, values)
    } else {
        (
            srgb()?,
            stops.iter().flat_map(|s| components(s.color)).collect(),
        )
    };
    let locations = stops
        .iter()
        .map(|s| CGFloat::from(s.position))
        .collect::<Vec<_>>();
    // SAFETY : `values` contient (composantes + alpha) × `stops.len()` réels et
    // `locations` un réel par arrêt, lus pendant l'appel.
    let gradient = unsafe {
        CGGradient::with_color_components(
            Some(&space),
            values.as_ptr(),
            locations.as_ptr(),
            stops.len(),
        )
    };
    super::super::ffi::required(gradient, "CGGradientCreateWithColorComponents")
}
const EXTEND: CGGradientDrawingOptions = CGGradientDrawingOptions(
    CGGradientDrawingOptions::DrawsBeforeStartLocation.0
        | CGGradientDrawingOptions::DrawsAfterEndLocation.0,
);
/// Dessine un dégradé linéaire étendu sur toute la zone de découpe courante.
pub(super) fn draw_linear(
    context: &CGContext,
    gradient: &LinearGradient,
    rect: RectF,
    alpha_only: bool,
) -> crate::Result<()> {
    let native = self::gradient(&gradient.stops, alpha_only)?;
    let (start, end) = linear_points(gradient, rect);
    CGContext::draw_linear_gradient(
        Some(context),
        Some(&native),
        point(start),
        point(end),
        EXTEND,
    );
    Ok(())
}
fn draw_radial(context: &CGContext, gradient: &RadialGradient, rect: RectF) -> crate::Result<()> {
    let native = self::gradient(&gradient.stops, false)?;
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    let [rx, ry] = gradient.radii(w, h);
    let c = Some(context);
    CGContext::save_g_state(c);
    CGContext::concat_ctm(
        c,
        CGAffineTransform {
            a: CGFloat::from(rx.max(0.001)),
            b: 0.,
            c: 0.,
            d: CGFloat::from(ry.max(0.001)),
            tx: CGFloat::from(rect.left + gradient.center[0] * w),
            ty: CGFloat::from(rect.top + gradient.center[1] * h),
        },
    );
    let origin = point(PointF { x: 0., y: 0. });
    CGContext::draw_radial_gradient(c, Some(&native), origin, 0., origin, 1., EXTEND);
    CGContext::restore_g_state(c);
    Ok(())
}
impl Canvas<'_> {
    pub(super) fn set_fill(&self, color: Color) {
        CGContext::set_fill_color_with_color(Some(self.context), Some(&cg_color(color)));
    }
    pub(super) fn set_stroke(&self, color: Color, width: f32) {
        let c = Some(self.context);
        CGContext::set_stroke_color_with_color(c, Some(&cg_color(color)));
        CGContext::set_line_width(c, CGFloat::from(width));
    }
    pub(super) fn fill_path(&self, path: &Path, even_odd: bool) {
        let c = Some(self.context);
        CGContext::add_path(c, Some(&path.0));
        if even_odd {
            CGContext::eo_fill_path(c);
        } else {
            CGContext::fill_path(c);
        }
    }
    pub(super) fn stroke_path(&self, path: &Path) {
        let c = Some(self.context);
        CGContext::add_path(c, Some(&path.0));
        CGContext::stroke_path(c);
    }
    /// Remplit `path` avec `fill`, posé sur `rect` pour les dégradés.
    pub(super) fn fill_background(
        &self,
        path: &Path,
        fill: &Background,
        rect: RectF,
    ) -> crate::Result<()> {
        match fill {
            Background::Solid(color) => {
                self.set_fill(*color);
                self.fill_path(path, false);
                Ok(())
            }
            Background::Linear(gradient) => {
                self.clipped(path, |c| draw_linear(c, gradient, rect, false))
            }
            Background::Radial(gradient) => self.clipped(path, |c| draw_radial(c, gradient, rect)),
        }
    }
    fn clipped(
        &self,
        path: &Path,
        draw: impl FnOnce(&CGContext) -> crate::Result<()>,
    ) -> crate::Result<()> {
        let c = Some(self.context);
        CGContext::save_g_state(c);
        CGContext::add_path(c, Some(&path.0));
        CGContext::clip(c);
        let result = draw(self.context);
        CGContext::restore_g_state(c);
        result
    }
    /// Image préparée par le renderer, étirée dans `rect` (réduite une fois à sa taille
    /// d'affichage quand le repère est sans rotation).
    pub(super) fn image(&mut self, rect: RectF, image: &Arc<Image>) -> crate::Result<()> {
        let t = self.transform;
        if t.m12 == 0. && t.m21 == 0. && t.m11 > 0. && t.m22 > 0. {
            // Sans rotation ni miroir : image à sa taille d'affichage exacte, posée sur les
            // pixels de la cible (simple copie, sans rééchantillonnage à chaque image).
            let device = t.bounds(rect);
            let (x, y) = (device.left.round(), device.top.round());
            let width = (device.right - device.left).round().max(0.);
            let height = (device.bottom - device.top).round().max(0.);
            if let Some(native) = self.caches.image(image, width as usize, height as usize)? {
                self.blit(&native, x, y, width, height);
            }
            return Ok(());
        }
        let Some(native) = self
            .caches
            .image(image, image.width as usize, image.height as usize)?
        else {
            return Ok(());
        };
        let c = Some(self.context);
        CGContext::save_g_state(c);
        // Repère y vers le bas : l'image est retournée autour du milieu de `rect`.
        CGContext::concat_ctm(
            c,
            CGAffineTransform {
                a: 1.,
                b: 0.,
                c: 0.,
                d: -1.,
                tx: 0.,
                ty: CGFloat::from(rect.top + rect.bottom),
            },
        );
        CGContext::draw_image(c, cg_rect(rect), Some(&native));
        CGContext::restore_g_state(c);
        Ok(())
    }
}
