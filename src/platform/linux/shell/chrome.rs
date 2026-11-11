//! Barre de titre et bords de la fenêtre, dessinés par le moteur quand le système ne
//! décore pas la fenêtre (GNOME sous Wayland, fenêtre sans cadre sous X11) : boutons de
//! légende (survol, appui, relâchement), déplacement, double clic, redimensionnement par
//! les bords et curseurs des bords. Mêmes zones que sous Windows
//! (`ui::title_bar::hit_test`), en unités logiques. Chaque backend traduit le bord et le
//! déplacement dans son protocole (`xdg_toplevel`, `_NET_WM_MOVERESIZE`).
use super::session::Session;
use crate::{
    ui::{
        geometry::{Point, Rect},
        scene::PointF,
        title_bar::{HitRegion, button_at, hit_test},
    },
    window::WindowAction,
};
use std::time::{Duration, Instant};

/// Délai et distance d'un double clic.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const DOUBLE_CLICK_DISTANCE: f32 = 4.;
/// Longueur des coins de la bordure extérieure, le long de chaque bord.
const CORNER: f32 = 20.;

/// Bord de redimensionnement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::platform::linux) enum Edge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
/// Suite à donner à un appui du bouton principal.
pub(in crate::platform::linux) enum Press {
    /// Le contenu reçoit l'appui.
    Content,
    /// Bouton de la barre de titre enfoncé : l'action suit le relâchement.
    Button,
    /// Déplacer la fenêtre.
    Move,
    /// Agrandir ou restaurer (double clic sur une zone de déplacement).
    Maximize,
    /// Redimensionner depuis ce bord.
    Resize(Edge),
}
/// Bord de redimensionnement d'une zone, et noms de son curseur dans le thème.
fn edge(region: &HitRegion) -> Option<(Edge, &'static [&'static str])> {
    Some(match region {
        HitRegion::Top => (Edge::Top, &["ns-resize", "n-resize", "top_side"]),
        HitRegion::Bottom => (Edge::Bottom, &["ns-resize", "s-resize", "bottom_side"]),
        HitRegion::Left => (Edge::Left, &["ew-resize", "w-resize", "left_side"]),
        HitRegion::TopLeft => (
            Edge::TopLeft,
            &["nwse-resize", "nw-resize", "top_left_corner"],
        ),
        HitRegion::BottomLeft => (
            Edge::BottomLeft,
            &["nesw-resize", "sw-resize", "bottom_left_corner"],
        ),
        HitRegion::Right => (Edge::Right, &["ew-resize", "e-resize", "right_side"]),
        HitRegion::TopRight => (
            Edge::TopRight,
            &["nesw-resize", "ne-resize", "top_right_corner"],
        ),
        HitRegion::BottomRight => (
            Edge::BottomRight,
            &["nwse-resize", "se-resize", "bottom_right_corner"],
        ),
        _ => return None,
    })
}
impl Session {
    fn area(&self) -> Rect {
        let (width, height) = self.logical_size();
        Rect {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        }
    }
    /// Zone sous `point` : bords (sauf fenêtre décorée par le compositeur, agrandie ou
    /// collée), barre de titre du moteur, contenu.
    fn region(&self, point: PointF) -> HitRegion {
        if self.server_decorated {
            return HitRegion::Client;
        }
        let at = Point {
            x: point.x.floor() as i32,
            y: point.y.floor() as i32,
        };
        // Bords tirés hors du contenu : le contenu garde ses pixels jusqu'au bord.
        let fixed =
            self.edges_outside || self.tiled || !self.facts.resizable || self.facts.fullscreen;
        match hit_test(&self.area(), &at, fixed) {
            // Sans barre du moteur, la bande du haut appartient à l'application.
            HitRegion::Caption if !self.chrome_visible() => HitRegion::Client,
            // Bande intérieure (X11, compositeur sans sous-surfaces) : une barre de
            // défilement collée au bord s'attrape ; les coins restent aux bords.
            HitRegion::Left | HitRegion::Right | HitRegion::Top | HitRegion::Bottom
                if self.runtime.scrollbar_at(point) =>
            {
                HitRegion::Client
            }
            region => region,
        }
    }
    fn button_at(&self, point: PointF) -> Option<crate::ui::title_bar::CaptionButton> {
        if !self.chrome_visible() {
            return None;
        }
        let at = Point {
            x: point.x.floor() as i32,
            y: point.y.floor() as i32,
        };
        button_at(&self.area(), &at)
    }
    /// Pointeur au-dessus de `point` : bouton survolé, et curseur d'un bord éventuel.
    pub fn hover(&mut self, point: PointF) -> Option<&'static [&'static str]> {
        let button = self.button_at(point);
        if self.hovered != button && self.pressed.is_none() {
            self.hovered = button;
            self.dirty = true;
        }
        edge(&self.region(point)).map(|(_, cursor)| cursor)
    }
    /// Bord sous `point`, hors du contenu (bordure de la fenêtre) ; les coins s'étendent
    /// sur [`CORNER`] le long de chaque bord.
    pub fn outer_edge(&self, point: PointF) -> Option<(Edge, &'static [&'static str])> {
        let (width, height) = self.logical_size();
        let side = |v: f32, end: i32| {
            if v < CORNER {
                Some(false)
            } else if v >= end as f32 - CORNER {
                Some(true)
            } else {
                None
            }
        };
        let region = match (side(point.x, width), side(point.y, height)) {
            (Some(false), Some(false)) => HitRegion::TopLeft,
            (Some(true), Some(false)) => HitRegion::TopRight,
            (Some(false), Some(true)) => HitRegion::BottomLeft,
            (Some(true), Some(true)) => HitRegion::BottomRight,
            (Some(false), None) => HitRegion::Left,
            (Some(true), None) => HitRegion::Right,
            (None, Some(false)) => HitRegion::Top,
            (None, Some(true)) => HitRegion::Bottom,
            (None, None) => return None,
        };
        edge(&region)
    }
    /// Pointeur sorti de la fenêtre.
    pub fn leave(&mut self) {
        if self.hovered.take().is_some() {
            self.dirty = true;
        }
    }
    /// Appui du bouton principal en `point`.
    pub fn press(&mut self, point: PointF, now: Instant) -> crate::Result<Press> {
        let region = self.region(point);
        if let Some((edge, _)) = edge(&region) {
            return Ok(Press::Resize(edge));
        }
        // Poignée de l'application (`data-resize-region`), même sur une fenêtre fixe.
        self.prepare()?;
        if let Some(edge) = self.resize_region_at(point) {
            return Ok(Press::Resize(from_resize_edge(edge)));
        }
        let double = self.last_press.is_some_and(|(at, previous)| {
            now.duration_since(at) <= DOUBLE_CLICK
                && (previous.x - point.x).abs() <= DOUBLE_CLICK_DISTANCE
                && (previous.y - point.y).abs() <= DOUBLE_CLICK_DISTANCE
        });
        self.last_press = if double { None } else { Some((now, point)) };
        if let Some(button) = self.button_at(point) {
            self.pressed = Some(button);
            self.hovered = Some(button);
            self.dirty = true;
            return Ok(Press::Button);
        }
        // Barre du moteur ou zone de déplacement de l'application (`data-drag-region`),
        // jugée sur l'arbre mis à jour ci-dessus.
        let drag = matches!(region, HitRegion::Caption) || self.runtime.drag_region_at(point);
        // Une fenêtre fixe ou en plein écran ne s'agrandit pas au double clic.
        let zoomable = self.facts.resizable && !self.facts.fullscreen;
        Ok(match (drag, double) {
            (true, true) if zoomable => Press::Maximize,
            (true, _) => Press::Move,
            (false, _) => Press::Content,
        })
    }
    /// Relâchement du bouton principal : action du bouton de légende enfoncé puis relâché
    /// au-dessus de lui. `None` si aucun bouton n'était enfoncé.
    pub fn release(&mut self, point: PointF) -> Option<Option<WindowAction>> {
        let pressed = self.pressed.take()?;
        let released = self.button_at(point);
        self.hovered = released;
        self.dirty = true;
        Some((released == Some(pressed)).then(|| pressed.action()))
    }
    /// `point` est sur la barre de titre du moteur (menu de la fenêtre au clic droit).
    pub fn caption_at(&self, point: PointF) -> bool {
        matches!(self.region(point), HitRegion::Caption)
    }
}

/// Noms du thème pour un curseur du moteur, du nom CSS aux noms historiques X11.
pub(in crate::platform::linux) fn cursor_names(
    cursor: crate::ui::runtime::Cursor,
) -> &'static [&'static str] {
    match cursor {
        crate::ui::runtime::Cursor::Arrow => &["default", "left_ptr"],
        crate::ui::runtime::Cursor::Hand => &["pointer", "hand2", "hand1"],
        crate::ui::runtime::Cursor::ResizeHorizontal => {
            &["col-resize", "ew-resize", "sb_h_double_arrow"]
        }
        crate::ui::runtime::Cursor::ResizeVertical => {
            &["row-resize", "ns-resize", "sb_v_double_arrow"]
        }
        crate::ui::runtime::Cursor::Text => &["text", "xterm"],
        crate::ui::runtime::Cursor::NotAllowed => &["not-allowed", "crossed_circle"],
        crate::ui::runtime::Cursor::Wait => &["wait", "watch"],
        crate::ui::runtime::Cursor::Progress => &["progress", "left_ptr_watch"],
        crate::ui::runtime::Cursor::Help => &["help", "question_arrow"],
        crate::ui::runtime::Cursor::Move => &["move", "fleur"],
        crate::ui::runtime::Cursor::Crosshair => &["crosshair", "cross"],
        crate::ui::runtime::Cursor::ResizeDiagonal => &["nwse-resize", "bd_double_arrow"],
        crate::ui::runtime::Cursor::ResizeAntiDiagonal => &["nesw-resize", "fd_double_arrow"],
    }
}

/// Bord du moteur pour une poignée de l'application.
fn from_resize_edge(edge: crate::window::ResizeEdge) -> Edge {
    use crate::window::ResizeEdge;
    match edge {
        ResizeEdge::Left => Edge::Left,
        ResizeEdge::Right => Edge::Right,
        ResizeEdge::Top => Edge::Top,
        ResizeEdge::Bottom => Edge::Bottom,
        ResizeEdge::TopLeft => Edge::TopLeft,
        ResizeEdge::TopRight => Edge::TopRight,
        ResizeEdge::BottomLeft => Edge::BottomLeft,
        ResizeEdge::BottomRight => Edge::BottomRight,
    }
}
