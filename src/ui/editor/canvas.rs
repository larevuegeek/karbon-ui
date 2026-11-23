//! Base des toiles 2D des éditeurs (KUI-036) et tracés épais (KUI-037) : le passage des
//! coordonnées du monde (secondes et valeurs d'une courbe, place des nœuds d'un graphe) à
//! celles de l'écran, le zoom autour du pointeur, le glissement de la vue, le cadrage ; et
//! des courbes de Bézier tracées avec une épaisseur, découpées en segments assez courts
//! pour rester lisses à l'écran.
use crate::ui::{
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};

/// How a 2D canvas shows its world: the world point at the top left corner of the
/// canvas and the pixels per world unit, horizontally and vertically (a curve editor
/// zooms its time and its values apart). `y` grows downwards on the screen and upwards
/// in the world, as on a graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasView {
    /// World point at the top left corner of the canvas.
    pub origin: PointF,
    /// Pixels per world unit, horizontally and vertically (both positive).
    pub zoom: (f32, f32),
}

/// Bornes du zoom, en pixels par unité.
const ZOOM_RANGE: (f32, f32) = (0.01, 100_000.);

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            origin: PointF { x: 0., y: 1. },
            zoom: (100., 100.),
        }
    }
}

impl CanvasView {
    /// The screen point (from the top left corner of the canvas) of `world`.
    pub fn to_screen(&self, world: PointF) -> PointF {
        PointF {
            x: (world.x - self.origin.x) * self.zoom.0,
            y: (self.origin.y - world.y) * self.zoom.1,
        }
    }

    /// The world point under `screen`.
    pub fn to_world(&self, screen: PointF) -> PointF {
        PointF {
            x: self.origin.x + screen.x / self.zoom.0,
            y: self.origin.y - screen.y / self.zoom.1,
        }
    }

    /// Zoom by `factor` (horizontally and vertically) around the screen point `at`, which
    /// stays over the same world point. The zoom stays between 0.01 and 100 000 pixels
    /// per unit.
    pub fn zoom_at(&mut self, at: PointF, factor: (f32, f32)) {
        let fixed = self.to_world(at);
        let clamp = |z: f32| z.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        self.zoom = (clamp(self.zoom.0 * factor.0), clamp(self.zoom.1 * factor.1));
        self.origin = PointF {
            x: fixed.x - at.x / self.zoom.0,
            y: fixed.y + at.y / self.zoom.1,
        };
    }

    /// Slide the view by `by` screen pixels (the content follows the pointer).
    pub fn pan(&mut self, by: (f32, f32)) {
        self.origin.x -= by.0 / self.zoom.0;
        self.origin.y += by.1 / self.zoom.1;
    }

    /// Show the world rectangle `world` (`left`, `bottom` .. `right`, `top` in world
    /// units, `top` above `bottom`) in a canvas of `size` pixels, with `margin` pixels
    /// around. A flat side is given one unit.
    pub fn frame(&mut self, world: RectF, size: (f32, f32), margin: f32) {
        let width = (world.right - world.left).abs().max(1e-3);
        let height = (world.top - world.bottom).abs().max(1e-3);
        let inner = (
            (size.0 - 2. * margin).max(1.),
            (size.1 - 2. * margin).max(1.),
        );
        let clamp = |z: f32| z.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        self.zoom = (clamp(inner.0 / width), clamp(inner.1 / height));
        self.origin = PointF {
            x: world.left.min(world.right) - margin / self.zoom.0,
            y: world.top.max(world.bottom) + margin / self.zoom.1,
        };
    }
}

/// The points of the cubic Bézier curve from `start` to `end` with the control points
/// `c1` and `c2`, close enough that no segment is longer than `step` (in the units of the
/// points; at least two points, at most 512).
pub fn bezier_points(start: PointF, c1: PointF, c2: PointF, end: PointF, step: f32) -> Vec<PointF> {
    let length = |a: PointF, b: PointF| (b.x - a.x).hypot(b.y - a.y);
    let hull = length(start, c1) + length(c1, c2) + length(c2, end);
    let count = ((hull / step.max(0.1)).ceil() as usize).clamp(1, 511);
    (0..=count)
        .map(|i| {
            let t = i as f32 / count as f32;
            let u = 1. - t;
            let (a, b, c, d) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
            PointF {
                x: a * start.x + b * c1.x + c * c2.x + d * end.x,
                y: a * start.y + b * c1.y + c * c2.y + d * end.y,
            }
        })
        .collect()
}

/// Draw the polyline through `points` with a thickness of `width` pixels in `color`
/// (wires of a graph, curves of an editor). With `dash` (`Some((on, off))` in pixels),
/// only the dashes are drawn. Segments shorter than half a pixel are merged.
pub fn stroke(
    scene: &mut Scene,
    points: &[PointF],
    width: f32,
    color: Color,
    dash: Option<(f32, f32)>,
) {
    let Some((&first, rest)) = points.split_first() else {
        return;
    };
    let mut from = first;
    // Où l'on en est dans le motif des tirets : la distance parcourue, et si c'est un trait.
    let mut walked = 0.;
    for &to in rest {
        let length = (to.x - from.x).hypot(to.y - from.y);
        if length < 0.5 && to != *points.last().unwrap_or(&to) {
            continue;
        }
        match dash {
            None => scene.commands.push(DrawCommand::Line {
                start: from,
                end: to,
                width,
                color,
            }),
            Some((on, off)) => dashes(scene, (from, to), (on, off, &mut walked), width, color),
        }
        from = to;
    }
}

/// Les tirets du segment `from`–`to`, le motif repris où le segment d'avant l'a laissé.
fn dashes(
    scene: &mut Scene,
    (from, to): (PointF, PointF),
    (on, off, walked): (f32, f32, &mut f32),
    width: f32,
    color: Color,
) {
    let length = (to.x - from.x).hypot(to.y - from.y);
    let period = (on + off).max(0.5);
    let at = |d: f32| PointF {
        x: from.x + (to.x - from.x) * d / length,
        y: from.y + (to.y - from.y) * d / length,
    };
    let mut done = 0.;
    while done < length {
        let phase = (*walked + done) % period;
        let (drawing, left) = if phase < on {
            (true, on - phase)
        } else {
            (false, period - phase)
        };
        let next = (done + left).min(length);
        if drawing {
            scene.commands.push(DrawCommand::Line {
                start: at(done),
                end: at(next),
                width,
                color,
            });
        }
        done = next;
    }
    *walked += length;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_comes_back_from_the_screen_and_stays_put_while_zooming_around_it() {
        let mut view = CanvasView::default();
        let world = PointF { x: 2.5, y: -0.25 };
        let screen = view.to_screen(world);
        let back = view.to_world(screen);
        assert!((back.x - world.x).abs() < 1e-5 && (back.y - world.y).abs() < 1e-5);
        // Zoom de 1 % à 1 000 % autour du pointeur : le point sous lui ne bouge pas.
        for factor in [0.01, 3., 10., 0.5] {
            view.zoom_at(screen, (factor, factor));
            let under = view.to_world(screen);
            assert!((under.x - world.x).abs() < 1e-3, "{factor}: {under:?}");
            assert!((under.y - world.y).abs() < 1e-3, "{factor}: {under:?}");
        }
        // Glisser de 50 pixels : le contenu suit le pointeur.
        let before = view.to_screen(world);
        view.pan((50., -20.));
        let after = view.to_screen(world);
        assert!((after.x - before.x - 50.).abs() < 1e-3 && (after.y - before.y + 20.).abs() < 1e-3);
    }

    #[test]
    fn framing_shows_the_whole_rectangle_inside_the_margin() {
        let mut view = CanvasView::default();
        let world = RectF {
            left: 0.,
            right: 4.,
            bottom: -1.,
            top: 1.,
        };
        view.frame(world, (420., 220.), 10.);
        let top_left = view.to_screen(PointF { x: 0., y: 1. });
        let bottom_right = view.to_screen(PointF { x: 4., y: -1. });
        assert!((top_left.x - 10.).abs() < 1e-3 && (top_left.y - 10.).abs() < 1e-3);
        assert!((bottom_right.x - 410.).abs() < 1e-3 && (bottom_right.y - 210.).abs() < 1e-3);
    }

    #[test]
    fn a_bezier_is_cut_in_short_segments_and_dashes_skip_the_gaps() {
        let p = |x: f32, y: f32| PointF { x, y };
        let points = bezier_points(p(0., 0.), p(100., 0.), p(100., 100.), p(200., 100.), 4.);
        assert_eq!(points.first(), Some(&p(0., 0.)));
        assert_eq!(points.last(), Some(&p(200., 100.)));
        assert!(
            points
                .windows(2)
                .all(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y) <= 4.01)
        );
        let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
        stroke(&mut scene, &points, 2., Color::hex(0xFFFFFF), None);
        let full = scene.commands.len();
        assert_eq!(full, points.len() - 1);
        // Tirets de 6 sur 6 : à peu près la moitié de la longueur, en plus de morceaux.
        let mut dashed = Scene::new(Color::rgba(0, 0, 0, 0));
        let line = [p(0., 0.), p(120., 0.)];
        stroke(&mut dashed, &line, 2., Color::hex(0xFFFFFF), Some((6., 6.)));
        let drawn: f32 = dashed
            .commands
            .iter()
            .map(|c| match c {
                DrawCommand::Line { start, end, .. } => end.x - start.x,
                _ => 0.,
            })
            .sum();
        assert_eq!(dashed.commands.len(), 10);
        assert!((drawn - 60.).abs() < 1e-3, "{drawn}");
    }
}
