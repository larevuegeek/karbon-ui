//! Dessins SVG compilés : remplissage et tracé des contours aplatis.
use super::{Canvas, paint::premultiplied};
use crate::ui::{scene::RectF, theme::Color};

impl Canvas<'_> {
    /// Dessin SVG compilé : remplissage pair-impair ou non nul des contours aplatis,
    /// puis tracé (jonctions arrondies ; bouts plats, ronds ou carrés).
    pub(super) fn svg(
        &mut self,
        rect: RectF,
        image: &crate::ui::rendering::svg::SvgImage,
        current: Color,
    ) {
        use crate::ui::rendering::svg::{FillRule, flatten, on_stroke, winding};
        let placement = image.placement(rect);
        // Tolérance d'aplatissement : un quart de pixel physique.
        let tolerance =
            0.25 / (self.scale * self.transform.m11.hypot(self.transform.m12)).max(0.01);
        let unit = placement.m11.abs().min(placement.m22.abs());
        for shape in image.shapes {
            let lines = flatten(shape.ops, placement, tolerance);
            if lines.is_empty() {
                continue;
            }
            let mut bounds = RectF {
                left: f32::INFINITY,
                top: f32::INFINITY,
                right: f32::NEG_INFINITY,
                bottom: f32::NEG_INFINITY,
            };
            for p in lines.iter().flat_map(|l| &l.points) {
                bounds.left = bounds.left.min(p.x);
                bounds.top = bounds.top.min(p.y);
                bounds.right = bounds.right.max(p.x);
                bounds.bottom = bounds.bottom.max(p.y);
            }
            if let Some(color) = shape.fill.resolve(current, shape.fill_opacity) {
                let source = premultiplied(color, self.opacity);
                let evenodd = shape.fill_rule == FillRule::EvenOdd;
                self.cover(bounds, |p| {
                    let w = winding(&lines, p);
                    let inside = if evenodd { w % 2 != 0 } else { w != 0 };
                    inside.then_some(source)
                });
            }
            if let Some(color) = shape.stroke.resolve(current, shape.stroke_opacity) {
                let half = shape.stroke_width * unit / 2.;
                if half <= 0. {
                    continue;
                }
                let source = premultiplied(color, self.opacity);
                let reach = half
                    * if shape.line_cap == crate::ui::rendering::svg::LineCap::Square {
                        1.5
                    } else {
                        1.
                    };
                let area = RectF {
                    left: bounds.left - reach,
                    top: bounds.top - reach,
                    right: bounds.right + reach,
                    bottom: bounds.bottom + reach,
                };
                self.cover(area, |p| {
                    on_stroke(&lines, p, half, shape.line_cap).then_some(source)
                });
            }
        }
    }
}
