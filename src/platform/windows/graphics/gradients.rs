//! Direct2D brushes, retained while their portable gradient is present in the scene.
use super::*;
use crate::ui::rendering::brush::LinearGradient;
use std::collections::HashSet;
#[derive(Default)]
pub(super) struct Cache {
    radials: super::radials::Cache,
    brushes: HashMap<usize, (Arc<LinearGradient>, ID2D1LinearGradientBrush)>,
    used: HashSet<usize>,
}
impl Cache {
    #[cfg(test)]
    pub fn radial_len(&self) -> usize {
        self.radials.len()
    }
    pub fn prepare(&mut self, target: &ID2D1RenderTarget, scene: &Scene) -> crate::Result<()> {
        self.radials.prepare(target, scene)?;
        self.used.clear();
        for command in &scene.commands {
            let gradient = match command {
                DrawCommand::Gradient { gradient, .. } => Some(gradient),
                DrawCommand::Shape {
                    fill: Some(crate::ui::rendering::brush::Background::Linear(gradient)),
                    ..
                }
                | DrawCommand::PushTextFill {
                    fill: crate::ui::rendering::brush::Background::Linear(gradient),
                    ..
                }
                | DrawCommand::PushMask { gradient, .. } => Some(gradient),
                _ => None,
            };
            if let Some(gradient) = gradient {
                let key = Arc::as_ptr(gradient) as usize;
                self.used.insert(key);
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
                    target.CreateGradientStopCollection(
                        &stops,
                        D2D1_GAMMA_2_2,
                        D2D1_EXTEND_MODE_CLAMP,
                    )
                }
                .check("CreateGradientStopCollection")?;
                let props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                    startPoint: Vector2 { X: 0., Y: 0. },
                    endPoint: Vector2 { X: 1., Y: 0. },
                };
                // SAFETY: cible et collection valides ; propriétés lues pendant l'appel.
                let brush = unsafe { target.CreateLinearGradientBrush(&props, None, &collection) }
                    .check("CreateLinearGradientBrush")?;
                self.brushes.insert(key, (Arc::clone(gradient), brush));
            }
        }
        self.brushes.retain(|key, _| self.used.contains(key));
        Ok(())
    }
    pub fn brush(&self, gradient: &Arc<LinearGradient>, rect: RectF) -> &ID2D1LinearGradientBrush {
        let brush = &self.brushes[&(Arc::as_ptr(gradient) as usize)].1;
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
        let start = Vector2 {
            X: x - c * extent,
            Y: y - s * extent,
        };
        let end = Vector2 {
            X: x + c * extent,
            Y: y + s * extent,
        };
        // SAFETY: pinceau valide, possédé par le cache.
        unsafe {
            brush.SetStartPoint(start);
            brush.SetEndPoint(end);
        }
        brush
    }
    pub fn radial(
        &self,
        gradient: &Arc<crate::ui::rendering::brush::RadialGradient>,
        rect: RectF,
    ) -> &ID2D1RadialGradientBrush {
        self.radials.brush(gradient, rect)
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.brushes.len()
    }
}
