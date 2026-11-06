//! Formes en distance signée : rectangles à coins indépendants (remplis d'une couleur
//! ou d'un dégradé), traits (anneau entre deux boîtes) et segments, avec la géométrie de
//! `graphics/` (rayons normalisés, trait intérieur des formes, rayon unique réduit à la
//! moitié du plus petit côté).
use super::{Bind, Builder};
use crate::platform::macos::graphics::{linear_points, normalized};
use crate::platform::macos::metal::instance::{
    Instance, PAINT_LINEAR, PAINT_RADIAL, SHAPE_RING, SHAPE_SEGMENT, color, pixel, rect as array,
};
use crate::ui::{
    rendering::brush::{Background, LinearGradient, RadialGradient},
    scene::{DrawCommand, PointF, RectF},
    theme::Color,
};
use std::sync::Arc;

/// Peinture d'une boîte et rectangle sur lequel un dégradé est posé.
pub(super) enum Fill<'a> {
    Solid(Color),
    Linear(&'a LinearGradient, RectF),
    Radial(&'a RadialGradient, RectF),
}
/// Rayon unique réduit à la moitié du plus petit côté (`geometry::round_rect`).
fn single_radius(rect: RectF, radius: f32) -> [f32; 4] {
    let w = (rect.right - rect.left).max(0.);
    let h = (rect.bottom - rect.top).max(0.);
    [radius.max(0.).min(w / 2.).min(h / 2.); 4]
}
fn grown(r: RectF, by: f32) -> RectF {
    RectF {
        left: r.left - by,
        top: r.top - by,
        right: r.right + by,
        bottom: r.bottom + by,
    }
}
impl Builder<'_> {
    pub(super) fn paint(&mut self, command: &DrawCommand) -> crate::Result<()> {
        match command {
            DrawCommand::Fill {
                rect,
                radius,
                color,
            } => self.fill_box(*rect, single_radius(*rect, *radius), Fill::Solid(*color)),
            DrawCommand::Stroke {
                rect,
                radius,
                width,
                color,
            } => self.ring(*rect, single_radius(*rect, *radius), *width, *color),
            DrawCommand::Shape {
                rect,
                radii,
                fill,
                stroke,
            } => match (fill, stroke) {
                (_, Some((w, c))) => {
                    let w2 = w.min(rect.right - rect.left).min(rect.bottom - rect.top);
                    let (path, radii) = normalized(*rect, *radii, Some(*w));
                    self.ring(path, radii, w2, *c);
                }
                (Some(fill), None) => {
                    let (path, radii) = normalized(*rect, *radii, None);
                    let fill = match fill {
                        Background::Solid(c) => Fill::Solid(*c),
                        Background::Linear(g) => Fill::Linear(g, *rect),
                        Background::Radial(g) => Fill::Radial(g, *rect),
                    };
                    self.fill_box(path, radii, fill);
                }
                (None, None) => {}
            },
            DrawCommand::Gradient {
                rect,
                radius,
                gradient,
            } => self.fill_box(
                *rect,
                single_radius(*rect, *radius),
                Fill::Linear(gradient, *rect),
            ),
            DrawCommand::RadialGradient {
                rect,
                radius,
                gradient,
            } => self.fill_box(
                *rect,
                single_radius(*rect, *radius),
                Fill::Radial(gradient, *rect),
            ),
            DrawCommand::Line {
                start,
                end,
                width,
                color,
            } => self.line(*start, *end, *width, *color),
            DrawCommand::Image { rect, image } => self.image(*rect, image)?,
            DrawCommand::Path { points, .. } if points.len() < 2 => {}
            DrawCommand::Vector { .. } | DrawCommand::Svg { .. } | DrawCommand::Path { .. } => {
                self.raster(command)?
            }
            DrawCommand::Text {
                rect,
                text,
                size,
                bold,
                color,
                wrap,
            } => {
                let format = crate::ui::styling::text::TextFormat::new(
                    Default::default(),
                    crate::ui::font::Font::system(*bold),
                    *size,
                );
                self.text(&plain(text, format), *rect, *color, *wrap)?;
            }
            DrawCommand::FontText {
                rect,
                text,
                size,
                font,
                color,
                wrap,
            } => {
                let format =
                    crate::ui::styling::text::TextFormat::new(Default::default(), *font, *size);
                self.text(&plain(text, format), *rect, *color, *wrap)?;
            }
            DrawCommand::FormattedText {
                rect,
                text,
                format,
                color,
                wrap,
            } => self.text(&plain(text, *format), *rect, *color, *wrap)?,
            DrawCommand::RichText {
                rect,
                paragraph,
                color,
                wrap,
            } => self.text(paragraph, *rect, *color, *wrap)?,
            _ => {}
        }
        Ok(())
    }
    /// Boîte arrondie remplie.
    pub(super) fn fill_box(&mut self, shape: RectF, radii: [f32; 4], fill: Fill<'_>) {
        if !(shape.right > shape.left && shape.bottom > shape.top) {
            return;
        }
        let mut i = Instance::rounded(shape, radii, self.transform());
        match fill {
            Fill::Solid(c) => i.color = color(c),
            Fill::Linear(gradient, area) => {
                let (a, b) = linear_points(gradient, area);
                i.kind[1] = PAINT_LINEAR;
                i.paint = [a.x, a.y, b.x, b.y];
                [i.kind[2], i.kind[3]] = self.stops(&gradient.stops);
            }
            Fill::Radial(gradient, area) => {
                let (w, h) = (area.right - area.left, area.bottom - area.top);
                let [rx, ry] = gradient.radii(w, h);
                i.kind[1] = PAINT_RADIAL;
                i.paint = [
                    area.left + gradient.center[0] * w,
                    area.top + gradient.center[1] * h,
                    rx.max(0.001),
                    ry.max(0.001),
                ];
                [i.kind[2], i.kind[3]] = self.stops(&gradient.stops);
            }
        }
        self.emit(i, Bind::None);
    }
    /// Trait de `width` centré sur le contour de la boîte `path` (jonctions en onglet :
    /// coins vifs à l'extérieur d'un coin sans rayon).
    fn ring(&mut self, path: RectF, radii: [f32; 4], width: f32, c: Color) {
        if width.is_nan() || width <= 0. || c.alpha == 0 {
            return;
        }
        let half = width / 2.;
        let outer = grown(path, half);
        let outer_radii = radii.map(|r| if r > 0. { r + half } else { 0. });
        let inner = grown(path, -half);
        if !(inner.right > inner.left && inner.bottom > inner.top) {
            return self.fill_box(outer, outer_radii, Fill::Solid(c));
        }
        let mut i = Instance::rounded(outer, outer_radii, self.transform());
        i.kind[0] = SHAPE_RING;
        i.inner = array(inner);
        i.iradii = radii.map(|r| (r - half).max(0.));
        i.color = color(c);
        self.emit(i, Bind::None);
    }
    /// Segment à bouts plats ; horizontal ou vertical sous un repère droit, c'est une boîte.
    fn line(&mut self, start: PointF, end: PointF, width: f32, c: Color) {
        let half = width / 2.;
        if half.is_nan() || half <= 0. || (start.x == end.x && start.y == end.y) {
            return;
        }
        let t = self.transform();
        if t.m12 == 0. && t.m21 == 0. && (start.x == end.x || start.y == end.y) {
            let r = if start.y == end.y {
                RectF {
                    left: start.x.min(end.x),
                    top: start.y - half,
                    right: start.x.max(end.x),
                    bottom: start.y + half,
                }
            } else {
                RectF {
                    left: start.x - half,
                    top: start.y.min(end.y),
                    right: start.x + half,
                    bottom: start.y.max(end.y),
                }
            };
            return self.fill_box(r, [0.; 4], Fill::Solid(c));
        }
        let p = pixel(t);
        let quad = RectF {
            left: start.x.min(end.x) - half - p,
            top: start.y.min(end.y) - half - p,
            right: start.x.max(end.x) + half + p,
            bottom: start.y.max(end.y) + half + p,
        };
        let mut i = Instance::new(quad, t, SHAPE_SEGMENT, 0);
        i.shape = [start.x, start.y, end.x, end.y];
        i.radii = [half, 0., 0., 0.];
        i.color = color(c);
        self.emit(i, Bind::None);
    }
}
fn plain(
    text: &Arc<str>,
    format: crate::ui::styling::text::TextFormat,
) -> crate::ui::styling::text::RichParagraph {
    crate::ui::styling::text::RichParagraph {
        text: Arc::clone(text),
        format,
        spans: Vec::new(),
    }
}
