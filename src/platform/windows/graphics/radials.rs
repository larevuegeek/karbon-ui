//! ID2D1RadialGradientBrush, partagé par le matériel et WARP.
use super::*;
use crate::ui::rendering::brush::RadialGradient;
use std::collections::HashSet;
#[derive(Default)]
pub(super) struct Cache {
    brushes: HashMap<usize, (Arc<RadialGradient>, ID2D1RadialGradientBrush)>,
}
impl Cache {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.brushes.len()
    }
    pub fn prepare(&mut self, target: &ID2D1RenderTarget, scene: &Scene) -> crate::Result<()> {
        let mut used = HashSet::new();
        for command in &scene.commands {
            let gradient = match command {
                DrawCommand::RadialGradient { gradient, .. } => gradient,
                DrawCommand::Shape {
                    fill: Some(crate::ui::rendering::brush::Background::Radial(gradient)),
                    ..
                }
                | DrawCommand::PushTextFill {
                    fill: crate::ui::rendering::brush::Background::Radial(gradient),
                    ..
                } => gradient,
                _ => continue,
            };
            let key = Arc::as_ptr(gradient) as usize;
            used.insert(key);
            if self.brushes.contains_key(&key) {
                continue;
            }
            gradient.validate()?;
            let stops = gradient
                .stops
                .iter()
                .map(|s| D2D1_GRADIENT_STOP {
                    position: s.position,
                    color: color_f(s.color),
                })
                .collect::<Vec<_>>();
            // SAFETY: cible valide ; arrêts lus pendant l'appel (gamma 2.2, extension clamp).
            let collection = unsafe {
                target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP)
            }
            .check("Create radial gradient stops")?;
            let props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                center: Vector2 { X: 0., Y: 0. },
                gradientOriginOffset: Vector2 { X: 0., Y: 0. },
                radiusX: 1.,
                radiusY: 1.,
            };
            // SAFETY: cible et collection valides ; propriétés lues pendant l'appel.
            let brush = unsafe { target.CreateRadialGradientBrush(&props, None, &collection) }
                .check("CreateRadialGradientBrush")?;
            self.brushes.insert(key, (Arc::clone(gradient), brush));
        }
        self.brushes.retain(|key, _| used.contains(key));
        Ok(())
    }
    pub fn brush(&self, gradient: &Arc<RadialGradient>, rect: RectF) -> &ID2D1RadialGradientBrush {
        let brush = &self.brushes[&(Arc::as_ptr(gradient) as usize)].1;
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let center = Vector2 {
            X: rect.left + gradient.center[0] * w,
            Y: rect.top + gradient.center[1] * h,
        };
        let radii = gradient.radii(w, h);
        // SAFETY: pinceau valide, possédé par le cache.
        unsafe {
            brush.SetCenter(center);
            brush.SetRadiusX(radii[0].max(0.001));
            brush.SetRadiusY(radii[1].max(0.001));
        }
        brush
    }
}
