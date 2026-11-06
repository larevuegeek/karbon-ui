use super::*;
impl Slider {
    pub(super) fn geometry(&self, rect: RectF, value: f64) -> Geometry {
        let length = if self.axis == Axis::Horizontal {
            layout::width(rect)
        } else {
            layout::height(rect)
        };
        // Dessin CSS : le pouce reste entier dans la piste (moitié de sa largeur).
        let margin = match self.css_parts() {
            Some((thumb, _)) => (thumb.width.unwrap_or(16.) / 2.).min(length / 2.),
            None => 10f32.min(length / 2.),
        };
        let start = if self.axis == Axis::Horizontal {
            rect.left + margin
        } else {
            rect.top + margin
        };
        let length = (length - 2. * margin).max(0.);
        let ratio = ((value - self.min) / (self.max - self.min)).clamp(0., 1.) as f32;
        let center = start
            + length
                * if self.axis == Axis::Horizontal {
                    ratio
                } else {
                    1. - ratio
                };
        Geometry {
            start,
            length,
            center,
        }
    }

    pub(super) fn coordinate(&self, p: PointF) -> f32 {
        if self.axis == Axis::Horizontal {
            p.x
        } else {
            p.y
        }
    }

    pub(super) fn pointer_value(&self, rect: RectF, p: PointF, offset: f32) -> Option<f64> {
        let g = self.geometry(rect, self.value);
        if !p.x.is_finite() || !p.y.is_finite() || g.length <= 0. {
            return None;
        }
        let ratio = ((self.coordinate(p) - offset - g.start) / g.length).clamp(0., 1.) as f64;
        Some(self.snapped(
            self.min
                + (self.max - self.min)
                    * if self.axis == Axis::Horizontal {
                        ratio
                    } else {
                        1. - ratio
                    },
        ))
    }
}
