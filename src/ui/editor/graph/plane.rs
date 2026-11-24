//! Le plan d'une toile de graphe à l'écran : la vue (le point du plan en haut à gauche,
//! un zoom pareil dans les deux sens, `y` vers le bas comme à l'écran), le tracé d'un
//! fil, et la mini-carte posée dans le coin en bas à droite.
use super::model::Graph;
use crate::ui::{editor::canvas::bezier_points, scene::PointF, scene::RectF};

/// Bornes du zoom.
const ZOOM_RANGE: (f32, f32) = (0.1, 4.);
/// Taille de la mini-carte et sa marge au bord, en pixels ; taille de toile en deçà de
/// laquelle elle n'est pas montrée.
const MINIMAP: (f32, f32) = (180., 120.);
const MINIMAP_MARGIN: f32 = 10.;
const MINIMAP_ROOM: (f32, f32) = (360., 260.);

/// How a graph canvas shows its plane: the point of the plane at the top left corner and
/// the pixels per graph unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphView {
    /// The point of the plane at the top left corner of the canvas.
    pub origin: PointF,
    /// Pixels per graph unit (0.1 to 4).
    pub zoom: f32,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            origin: PointF { x: -40., y: -40. },
            zoom: 1.,
        }
    }
}

impl GraphView {
    /// The screen point (from the top left corner of the canvas) of `world`.
    pub fn to_screen(&self, world: PointF) -> PointF {
        PointF {
            x: (world.x - self.origin.x) * self.zoom,
            y: (world.y - self.origin.y) * self.zoom,
        }
    }

    /// The point of the plane under `screen`.
    pub fn to_world(&self, screen: PointF) -> PointF {
        PointF {
            x: self.origin.x + screen.x / self.zoom,
            y: self.origin.y + screen.y / self.zoom,
        }
    }

    /// The rectangle of the plane `world` on the screen.
    pub fn rect_to_screen(&self, world: RectF) -> RectF {
        let a = self.to_screen(PointF {
            x: world.left,
            y: world.top,
        });
        let b = self.to_screen(PointF {
            x: world.right,
            y: world.bottom,
        });
        RectF {
            left: a.x,
            top: a.y,
            right: b.x,
            bottom: b.y,
        }
    }

    /// Zoom by `factor` around the screen point `at`, which stays over the same point of
    /// the plane.
    pub fn zoom_at(&mut self, at: PointF, factor: f32) {
        let fixed = self.to_world(at);
        self.zoom = (self.zoom * factor).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        self.origin = PointF {
            x: fixed.x - at.x / self.zoom,
            y: fixed.y - at.y / self.zoom,
        };
    }

    /// Slide the view by `by` screen pixels (the plane follows the pointer).
    pub fn pan(&mut self, by: (f32, f32)) {
        self.origin.x -= by.0 / self.zoom;
        self.origin.y -= by.1 / self.zoom;
    }

    /// Show the rectangle of the plane `world` in a canvas of `size` pixels, with
    /// `margin` pixels around, at most at zoom 1.
    pub fn frame(&mut self, world: RectF, size: (f32, f32), margin: f32) {
        let width = (world.right - world.left).max(1.);
        let height = (world.bottom - world.top).max(1.);
        let room = (
            (size.0 - 2. * margin).max(1.),
            (size.1 - 2. * margin).max(1.),
        );
        self.zoom = (room.0 / width)
            .min(room.1 / height)
            .clamp(ZOOM_RANGE.0, 1.);
        self.origin = PointF {
            x: world.left - (size.0 / self.zoom - width) / 2.,
            y: world.top - (size.1 / self.zoom - height) / 2.,
        };
    }

    /// Center the view on the point of the plane `world`, in a canvas of `size` pixels.
    pub fn center_on(&mut self, world: PointF, size: (f32, f32)) {
        self.origin = PointF {
            x: world.x - size.0 / 2. / self.zoom,
            y: world.y - size.1 / 2. / self.zoom,
        };
    }
}

/// Les points du fil d'une sortie en `from` à une entrée en `to` (à l'écran) : une
/// courbe qui part à droite et arrive par la gauche.
pub(super) fn wire_points(from: PointF, to: PointF) -> Vec<PointF> {
    let pull = ((to.x - from.x).abs() / 2.).max(40.);
    let c1 = PointF {
        x: from.x + pull,
        y: from.y,
    };
    let c2 = PointF {
        x: to.x - pull,
        y: to.y,
    };
    bezier_points(from, c1, c2, to, 14.)
}

/// La mini-carte : sa place à l'écran, le rectangle du plan qu'elle montre, son échelle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Minimap {
    pub rect: RectF,
    pub world: RectF,
    pub scale: f32,
}

impl Minimap {
    /// La mini-carte d'une toile de `size` pixels montrant `graph` par `view`, si la toile
    /// est assez grande et le graphe n'est pas vide : le graphe et ce que montre la vue.
    pub fn of(graph: &Graph, view: &GraphView, size: (f32, f32)) -> Option<Self> {
        if size.0 < MINIMAP_ROOM.0 || size.1 < MINIMAP_ROOM.1 {
            return None;
        }
        let bounds = graph.bounds()?;
        let seen = RectF {
            left: view.origin.x,
            top: view.origin.y,
            right: view.origin.x + size.0 / view.zoom,
            bottom: view.origin.y + size.1 / view.zoom,
        };
        let world = RectF {
            left: bounds.left.min(seen.left),
            top: bounds.top.min(seen.top),
            right: bounds.right.max(seen.right),
            bottom: bounds.bottom.max(seen.bottom),
        };
        let scale = (MINIMAP.0 / (world.right - world.left).max(1.))
            .min(MINIMAP.1 / (world.bottom - world.top).max(1.));
        let rect = RectF::new(
            size.0 - MINIMAP.0 - MINIMAP_MARGIN,
            size.1 - MINIMAP.1 - MINIMAP_MARGIN,
            MINIMAP.0,
            MINIMAP.1,
        );
        Some(Self { rect, world, scale })
    }

    /// Le point de la mini-carte (à l'écran) d'un point du plan.
    pub fn to_screen(self, world: PointF) -> PointF {
        PointF {
            x: self.rect.left + (world.x - self.world.left) * self.scale,
            y: self.rect.top + (world.y - self.world.top) * self.scale,
        }
    }

    /// Le point du plan sous un point de la mini-carte.
    pub fn to_world(self, screen: PointF) -> PointF {
        PointF {
            x: self.world.left + (screen.x - self.rect.left) / self.scale,
            y: self.world.top + (screen.y - self.rect.top) / self.scale,
        }
    }

    /// Le point `screen` est-il sur la mini-carte ?
    pub fn contains(self, screen: PointF) -> bool {
        let r = self.rect;
        screen.x >= r.left && screen.x <= r.right && screen.y >= r.top && screen.y <= r.bottom
    }
}
