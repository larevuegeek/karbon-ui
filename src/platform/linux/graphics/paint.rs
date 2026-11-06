//! Remplissages : couleur unie, dégradés linéaires et radiaux (couleurs extrêmes
//! étendues, comme Direct2D en mode CLAMP) et images étirées.
use super::{canvas::Canvas, text::rgba};
use crate::{
    platform::{
        linux::ffi::cairo::{
            Context, EXTEND_PAD, FILL_EVEN_ODD, FILL_WINDING, FILTER_GOOD, Matrix, Pattern,
        },
        shared::linear_points,
    },
    ui::{
        rendering::brush::{Background, GradientStop, LinearGradient, RadialGradient},
        scene::{Image, RectF},
        theme::Color,
    },
};
use std::sync::Arc;

fn stops<'a>(
    pattern: Pattern<'a>,
    stops: &[GradientStop],
    alpha: f32,
    alpha_only: bool,
) -> Pattern<'a> {
    for stop in stops {
        let color = if alpha_only {
            [
                0.,
                0.,
                0.,
                f64::from(stop.color.alpha) / 255. * f64::from(alpha),
            ]
        } else {
            rgba(stop.color, alpha)
        };
        pattern.add_stop(stop.position.into(), color);
    }
    pattern.set_extend(EXTEND_PAD);
    pattern
}
/// Dégradé linéaire posé sur `rect` ; `alpha_only` ne garde que l'alpha des arrêts
/// (masques).
pub(super) fn linear(
    gradient: &LinearGradient,
    rect: RectF,
    alpha: f32,
    alpha_only: bool,
) -> crate::Result<Pattern<'static>> {
    let (start, end) = linear_points(gradient, rect);
    let pattern = Pattern::linear(start.x.into(), start.y.into(), end.x.into(), end.y.into())?;
    Ok(stops(pattern, &gradient.stops, alpha, alpha_only))
}
/// Dégradé radial elliptique posé sur `rect`.
pub(super) fn radial(
    gradient: &RadialGradient,
    rect: RectF,
    alpha: f32,
) -> crate::Result<Pattern<'static>> {
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    let [rx, ry] = gradient.radii(w, h).map(|r| f64::from(r.max(0.001)));
    let (cx, cy) = (
        f64::from(rect.left + gradient.center[0] * w),
        f64::from(rect.top + gradient.center[1] * h),
    );
    let pattern = Pattern::radial(0., 0., 1.)?;
    // Du repère utilisateur vers le cercle unité du motif.
    pattern.set_matrix(Matrix {
        xx: 1. / rx,
        yy: 1. / ry,
        x0: -cx / rx,
        y0: -cy / ry,
        ..Matrix::IDENTITY
    });
    Ok(stops(pattern, &gradient.stops, alpha, false))
}
impl Canvas<'_> {
    pub(super) fn set_fill(&self, color: Color) {
        let [r, g, b, a] = rgba(color, self.alpha);
        self.context.set_source_rgba(r, g, b, a);
    }
    pub(super) fn set_stroke(&self, color: Color, width: f32) {
        self.set_fill(color);
        self.context.set_line_width(width.into());
    }
    /// Remplit le chemin courant.
    pub(super) fn fill_path(&self, even_odd: bool) {
        self.context.set_fill_rule(if even_odd {
            FILL_EVEN_ODD
        } else {
            FILL_WINDING
        });
        self.context.fill();
    }
    /// Remplit le chemin tracé par `path` avec `fill`, posé sur `rect` pour les dégradés.
    pub(super) fn fill_background(
        &self,
        path: impl FnOnce(&Context),
        fill: &Background,
        rect: RectF,
    ) -> crate::Result<()> {
        match fill {
            Background::Solid(color) => self.set_fill(*color),
            Background::Linear(gradient) => {
                self.context
                    .set_source(&linear(gradient, rect, self.alpha, false)?);
            }
            Background::Radial(gradient) => {
                self.context
                    .set_source(&radial(gradient, rect, self.alpha)?);
            }
        }
        path(self.context);
        self.fill_path(false);
        Ok(())
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
                self.blit(&*native, x, y, width, height)?;
            }
            return Ok(());
        }
        let (iw, ih) = (image.width as usize, image.height as usize);
        let Some(native) = self.caches.image(image, iw, ih)? else {
            return Ok(());
        };
        let (rw, rh) = (
            f64::from((rect.right - rect.left).max(1e-3)),
            f64::from((rect.bottom - rect.top).max(1e-3)),
        );
        let pattern = Pattern::surface(&*native)?;
        pattern.set_filter(FILTER_GOOD);
        // Du repère logique vers les pixels de l'image.
        let (sx, sy) = (iw as f64 / rw, ih as f64 / rh);
        pattern.set_matrix(Matrix {
            xx: sx,
            yy: sy,
            x0: -f64::from(rect.left) * sx,
            y0: -f64::from(rect.top) * sy,
            ..Matrix::IDENTITY
        });
        let c = self.context;
        c.save();
        super::geometry::rectangle(c, rect);
        c.clip();
        c.set_source(&pattern);
        c.paint_with_alpha(self.alpha.into());
        c.restore();
        Ok(())
    }
}
