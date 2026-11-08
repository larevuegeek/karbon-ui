//! Commandes d'une fenêtre ouverte (taille, position, état) et instantané de son état
//! remis à l'application. Géométrie commune aux backends : rectangles, placement dans la
//! zone de travail, proportions imposées pendant un redimensionnement, geste d'une
//! poignée de l'application (`data-resize-region`).
use super::ResizeEdge;

/// Change to apply to an open window, sent with
/// [`WindowCommand::Change`](crate::window::WindowCommand::Change).
///
/// Sizes are in logical pixels of the window's display, like
/// [`WindowOptions::width`](crate::window::WindowOptions::width); positions are screen
/// coordinates in the unit of [`WindowOptions::x`](crate::window::WindowOptions::x)
/// (logical pixels at the primary display's scale, `y` pointing down). A change the
/// platform cannot honour is ignored; see `docs/windows.md` for the table per platform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowChange {
    /// Resize the client area to `width` × `height`, keeping the top-left corner.
    /// Both must be positive.
    Size {
        /// New client width.
        width: i32,
        /// New client height.
        height: i32,
    },
    /// Smallest client size the user (and [`Size`](Self::Size)) can reach; `None`
    /// removes the limit. Both values must be positive.
    MinSize(Option<(i32, i32)>),
    /// Largest client size the user (and [`Size`](Self::Size)) can reach; `None`
    /// removes the limit. Both values must be positive.
    MaxSize(Option<(i32, i32)>),
    /// Keep the client area at `width` : `height` while the user resizes it; `None`
    /// lets both sides change freely. Both terms must be positive.
    AspectRatio(Option<(u32, u32)>),
    /// Let the user resize and maximize the window from its edges (`true`, the
    /// default), or not. [`resize_region`](crate::ui::element::Element::resize_region)
    /// elements keep resizing a fixed-size window.
    Resizable(bool),
    /// Keep the window above the windows that are not topmost, like
    /// [`WindowOptions::topmost`](crate::window::WindowOptions::topmost) at opening.
    Topmost(bool),
    /// Move the window's top-left corner (outer frame) to (`x`, `y`).
    Position {
        /// Horizontal screen coordinate.
        x: i32,
        /// Vertical screen coordinate.
        y: i32,
    },
    /// Centre the window in the work area of its display.
    Center,
    /// Move the window as little as needed to fit it entirely in the work area of its
    /// display (the screen without the taskbar or the dock).
    KeepOnScreen,
    /// Maximize (`true`) or restore (`false`) the window.
    Maximized(bool),
    /// Enter (`true`) or leave (`false`) full screen: the window covers its whole
    /// display, without the taskbar.
    Fullscreen(bool),
    /// Show (`true`) or hide (`false`) the window, without closing it.
    Visible(bool),
    /// Show the window, restore it if minimized and bring it to the foreground.
    Focus,
}

/// Geometry and state of an open window, passed to
/// [`Application::window_changed`](crate::application::Application::window_changed).
///
/// Units are those of [`WindowChange`]: `x` and `y` can be stored and given back to
/// [`WindowChange::Position`] or [`WindowOptions::x`](crate::window::WindowOptions::x),
/// `width` and `height` to [`WindowChange::Size`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WindowState {
    /// Horizontal screen position of the outer top-left corner (`0` when the platform
    /// does not tell, as under Wayland).
    pub x: i32,
    /// Vertical screen position of the outer top-left corner.
    pub y: i32,
    /// Client width, in logical pixels.
    pub width: i32,
    /// Client height, in logical pixels.
    pub height: i32,
    /// Physical pixels per logical pixel of the window's display (`1.5` at 150 %).
    pub scale: f32,
    /// The window is maximized.
    pub maximized: bool,
    /// The window is minimized (iconified).
    pub minimized: bool,
    /// The window covers its whole display.
    pub fullscreen: bool,
    /// The window is above the windows that are not topmost.
    pub topmost: bool,
    /// The user resizes the window from its edges.
    pub resizable: bool,
    /// The user is moving or resizing the window with the pointer; the end of the
    /// gesture sends a state where it is `false`.
    pub dragging: bool,
}

/// Rectangle en pixels de l'écran (physiques ou logiques selon l'appelant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Coin haut-gauche qui fait tenir `window` dans `area` en le déplaçant le moins possible ;
/// une fenêtre plus grande que la zone s'aligne sur son coin haut-gauche.
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(dead_code)
)]
pub(crate) fn keep_on_screen(window: ScreenRect, area: ScreenRect) -> (i32, i32) {
    let fit = |position: i32, size: i32, start: i32, length: i32| {
        position.min(start + length - size).max(start)
    };
    (
        fit(window.x, window.width, area.x, area.width),
        fit(window.y, window.height, area.y, area.height),
    )
}

/// Coin haut-gauche d'une fenêtre de `width` × `height` centrée dans `area`.
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(dead_code)
)]
pub(crate) fn centered(width: i32, height: i32, area: ScreenRect) -> (i32, i32) {
    (
        area.x + (area.width - width) / 2,
        area.y + (area.height - height).max(0) / 2,
    )
}

/// Bords saisis pendant un redimensionnement à la souris.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) struct Edges {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
impl Edges {
    /// Bords saisis par une poignée de l'application (`data-resize-region`).
    pub(crate) fn of(edge: ResizeEdge) -> Self {
        use ResizeEdge::*;
        Self {
            left: matches!(edge, Left | TopLeft | BottomLeft),
            top: matches!(edge, Top | TopLeft | TopRight),
            right: matches!(edge, Right | TopRight | BottomRight),
            bottom: matches!(edge, Bottom | BottomLeft | BottomRight),
        }
    }
}

/// Taille de `width` × `height` ramenée aux proportions `ratio` pendant un geste sur
/// `edges` : un bord seul impose sa dimension, un coin garde la plus grande des deux
/// tailles proposées. Partant d'une taille aux bonnes proportions, le coin suit donc le
/// plus grand des deux déplacements comptés vers l'extérieur : le carré du micro-lecteur
/// de RustMusic grandit de `max(dx, dy)` (`resizeMicroPlayer`).
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) fn constrain_aspect(
    (width, height): (i32, i32),
    ratio: (u32, u32),
    edges: Edges,
) -> (i32, i32) {
    let ratio = f64::from(ratio.0) / f64::from(ratio.1.max(1));
    let from_width = (width, (f64::from(width) / ratio).round() as i32);
    let from_height = ((f64::from(height) * ratio).round() as i32, height);
    let horizontal = edges.left || edges.right;
    let vertical = edges.top || edges.bottom;
    match (horizontal, vertical) {
        (true, false) => from_width,
        (false, true) => from_height,
        _ if f64::from(width) / ratio >= f64::from(height) => from_width,
        _ => from_height,
    }
}

/// `size` ramenée entre `min` et `max`. Avec `keep_ratio`, les deux côtés changent du
/// même facteur pour garder les proportions ; si les bornes se contredisent, le maximum
/// l'emporte.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) fn bound_size(
    (width, height): (i32, i32),
    min: (i32, i32),
    max: (i32, i32),
    keep_ratio: bool,
) -> (i32, i32) {
    if !keep_ratio {
        return (width.min(max.0).max(min.0), height.min(max.1).max(min.1));
    }
    let (width, height) = (f64::from(width.max(1)), f64::from(height.max(1)));
    let shrink = (f64::from(max.0) / width)
        .min(f64::from(max.1) / height)
        .min(1.);
    let grow = (f64::from(min.0) / width)
        .max(f64::from(min.1) / height)
        .max(1.);
    let factor = if shrink < 1. { shrink } else { grow };
    (
        (width * factor).round() as i32,
        (height * factor).round() as i32,
    )
}

/// Taille pendant le geste d'une poignée `edge`, dans l'unité de l'appelant : `start` à
/// l'appui, pointeur déplacé depuis de `delta` en coordonnées de l'écran (y vers le
/// bas), proportions `ratio` (règle de [`constrain_aspect`]) puis bornes `min`..`max`.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) fn grip_size(
    edge: ResizeEdge,
    start: (i32, i32),
    (dx, dy): (i32, i32),
    ratio: Option<(u32, u32)>,
    min: (i32, i32),
    max: (i32, i32),
) -> (i32, i32) {
    let edges = Edges::of(edge);
    // Déplacement compté vers l'extérieur du bord saisi.
    let outward = |delta: i32, before: bool, after: bool| {
        if after {
            delta
        } else if before {
            -delta
        } else {
            0
        }
    };
    let size = (
        (start.0 + outward(dx, edges.left, edges.right)).max(1),
        (start.1 + outward(dy, edges.top, edges.bottom)).max(1),
    );
    let size = ratio.map_or(size, |ratio| constrain_aspect(size, ratio, edges));
    bound_size(size, min, max, ratio.is_some())
}

/// Rectangle de `width` × `height` qui garde en place les bords de `start` opposés à
/// la poignée `edge` (y vers le bas).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn anchored(
    edge: ResizeEdge,
    start: ScreenRect,
    (width, height): (i32, i32),
) -> ScreenRect {
    let edges = Edges::of(edge);
    ScreenRect {
        x: if edges.left {
            start.x + start.width - width
        } else {
            start.x
        },
        y: if edges.top {
            start.y + start.height - height
        } else {
            start.y
        },
        width,
        height,
    }
}

/// Première incohérence d'un changement, en anglais pour l'erreur publique.
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(dead_code)
)]
pub(crate) fn invalid_change(change: &WindowChange) -> Option<&'static str> {
    match *change {
        WindowChange::Size { width, height } if width <= 0 || height <= 0 => {
            Some("WindowChange::Size needs a positive width and height")
        }
        WindowChange::MinSize(Some((width, height)))
        | WindowChange::MaxSize(Some((width, height)))
            if width <= 0 || height <= 0 =>
        {
            Some("WindowChange::MinSize and MaxSize need a positive width and height")
        }
        WindowChange::AspectRatio(Some((width, height))) if width == 0 || height == 0 => {
            Some("WindowChange::AspectRatio needs positive terms")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const AREA: ScreenRect = ScreenRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    };
    fn window(x: i32, y: i32, width: i32, height: i32) -> ScreenRect {
        ScreenRect {
            x,
            y,
            width,
            height,
        }
    }
    #[test]
    fn keep_on_screen_moves_the_least_needed() {
        // Déjà dans l'écran : rien ne bouge.
        assert_eq!(keep_on_screen(window(100, 200, 380, 246), AREA), (100, 200));
        // Collée en bas : le panneau qui s'ouvre la fait remonter juste ce qu'il faut.
        assert_eq!(keep_on_screen(window(100, 900, 380, 546), AREA), (100, 494));
        // Sortie à gauche et en haut, puis à droite.
        assert_eq!(keep_on_screen(window(-50, -10, 380, 246), AREA), (0, 0));
        assert_eq!(keep_on_screen(window(1800, 10, 220, 220), AREA), (1700, 10));
        // Plus grande que la zone : alignée sur son coin haut-gauche.
        assert_eq!(keep_on_screen(window(30, 30, 2000, 1200), AREA), (0, 0));
        // Second écran à gauche du principal (coordonnées négatives).
        let left = window(-1280, 0, 1280, 1000);
        assert_eq!(keep_on_screen(window(-100, 10, 380, 246), left), (-380, 10));
    }
    #[test]
    fn centered_uses_the_work_area_origin() {
        assert_eq!(centered(1280, 900, AREA), (320, 70));
        assert_eq!(centered(400, 300, window(1920, 40, 1280, 984)), (2360, 382));
    }
    #[test]
    fn aspect_ratio_follows_the_dragged_edges() {
        let corner = Edges::of(ResizeEdge::BottomRight);
        // Coin : la plus grande des deux tailles proposées, le carré est gardé.
        assert_eq!(constrain_aspect((300, 260), (1, 1), corner), (300, 300));
        assert_eq!(constrain_aspect((230, 400), (1, 1), corner), (400, 400));
        // Le plus grand déplacement signé, pas le plus grand en valeur absolue : vers la
        // droite de 50 et vers le haut de 80, le carré grandit de 50 (`max(dx, dy)`).
        assert_eq!(constrain_aspect((270, 140), (1, 1), corner), (270, 270));
        let right = Edges::of(ResizeEdge::Right);
        assert_eq!(constrain_aspect((320, 100), (16, 9), right), (320, 180));
        let bottom = Edges::of(ResizeEdge::Bottom);
        assert_eq!(constrain_aspect((100, 180), (16, 9), bottom), (320, 180));
        // Coin haut-gauche : les tailles proposées comptent déjà le geste vers l'extérieur.
        let top_left = Edges::of(ResizeEdge::TopLeft);
        assert_eq!(constrain_aspect((180, 250), (1, 1), top_left), (250, 250));
    }
    #[test]
    fn bounds_keep_the_ratio_when_asked() {
        // Sans proportions : chaque côté est borné seul.
        assert_eq!(
            bound_size((700, 100), (220, 220), (660, 660), false),
            (660, 220)
        );
        // Avec proportions : les deux côtés changent du même facteur.
        assert_eq!(
            bound_size((800, 400), (100, 100), (400, 400), true),
            (400, 200)
        );
        assert_eq!(
            bound_size((50, 100), (200, 200), (900, 900), true),
            (200, 400)
        );
        assert_eq!(
            bound_size((300, 300), (220, 220), (660, 660), true),
            (300, 300)
        );
    }
    #[test]
    fn a_grip_grows_the_micro_player_square_like_rustmusic() {
        // `resizeMicroPlayer` : côté = départ + max(dx, dy), borné 220-660.
        let micro = |delta| {
            grip_size(
                ResizeEdge::BottomRight,
                (220, 220),
                delta,
                Some((1, 1)),
                (220, 220),
                (660, 660),
            )
        };
        assert_eq!(micro((50, 10)), (270, 270));
        assert_eq!(micro((10, 50)), (270, 270));
        assert_eq!(micro((50, -80)), (270, 270));
        assert_eq!(micro((-30, -10)), (220, 220));
        assert_eq!(micro((900, 0)), (660, 660));
        // Sans proportions, chaque côté suit le pointeur ; un bord seul ne change que sa
        // dimension, tiré vers l'extérieur (vers la gauche pour le bord gauche).
        let free = |edge, delta| grip_size(edge, (400, 300), delta, None, (1, 1), (800, 800));
        assert_eq!(free(ResizeEdge::BottomRight, (30, -20)), (430, 280));
        assert_eq!(free(ResizeEdge::Left, (-50, 90)), (450, 300));
        assert_eq!(free(ResizeEdge::TopLeft, (20, 40)), (380, 260));
    }
    #[test]
    fn a_grip_keeps_the_opposite_edges_in_place() {
        let start = window(100, 200, 220, 220);
        assert_eq!(
            anchored(ResizeEdge::BottomRight, start, (300, 300)),
            window(100, 200, 300, 300)
        );
        assert_eq!(
            anchored(ResizeEdge::TopLeft, start, (300, 300)),
            window(20, 120, 300, 300)
        );
        assert_eq!(
            anchored(ResizeEdge::Top, start, (220, 250)),
            window(100, 170, 220, 250)
        );
    }
    #[test]
    fn invalid_changes_are_reported() {
        assert!(
            invalid_change(&WindowChange::Size {
                width: 0,
                height: 10
            })
            .is_some()
        );
        assert!(invalid_change(&WindowChange::MinSize(Some((10, -1)))).is_some());
        assert!(invalid_change(&WindowChange::MaxSize(Some((0, 10)))).is_some());
        assert!(invalid_change(&WindowChange::AspectRatio(Some((1, 0)))).is_some());
        assert!(invalid_change(&WindowChange::MinSize(None)).is_none());
        assert!(
            invalid_change(&WindowChange::Size {
                width: 380,
                height: 246
            })
            .is_none()
        );
        assert!(invalid_change(&WindowChange::Position { x: -200, y: -5 }).is_none());
    }
}
