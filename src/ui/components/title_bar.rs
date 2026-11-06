use crate::ui::geometry::{Point, Rect};
use crate::window::WindowAction;
/// Height of the custom title bar strip, in pixels.
pub const TITLE_BAR_HEIGHT: i32 = 40;
/// Width of each caption button, in pixels.
pub const BUTTON_WIDTH: i32 = 46;
/// Thickness of the resize band along the window edges, in pixels.
pub const RESIZE_BORDER: i32 = 8;

/// Caption button drawn at the right of the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionButton {
    /// Minimise the window.
    Minimize,
    /// Maximise or restore the window.
    Maximize,
    /// Close the window.
    Close,
}

impl CaptionButton {
    /// Window action triggered by this button.
    pub fn action(self) -> WindowAction {
        match self {
            Self::Minimize => WindowAction::Minimize,
            Self::Maximize => WindowAction::ToggleMaximize,
            Self::Close => WindowAction::Close,
        }
    }
}
/// Rectangle of `button` in a window whose client area is `area`: buttons sit in
/// the title bar, right-aligned inside the resize border, in the order minimise,
/// maximise, close.
pub fn button_rect(area: &Rect, button: CaptionButton) -> Rect {
    let index = match button {
        CaptionButton::Minimize => 3,
        CaptionButton::Maximize => 2,
        CaptionButton::Close => 1,
    };
    let left = area.right - RESIZE_BORDER - index * BUTTON_WIDTH;
    Rect {
        left,
        top: 0,
        right: left + BUTTON_WIDTH,
        bottom: TITLE_BAR_HEIGHT,
    }
}

/// Caption button under `point`, if any.
pub fn button_at(area: &Rect, point: &Point) -> Option<CaptionButton> {
    for button in [
        CaptionButton::Minimize,
        CaptionButton::Maximize,
        CaptionButton::Close,
    ] {
        let rectangle = button_rect(area, button);
        if point.x >= rectangle.left
            && point.x < rectangle.right
            && point.y >= rectangle.top
            && point.y < rectangle.bottom
        {
            return Some(button);
        }
    }
    None
}

/// Window region under a point, as reported to the OS for a frameless window.
#[derive(Debug, PartialEq, Eq)]
pub enum HitRegion {
    /// Outside the window area.
    Outside,
    /// Application content, including the caption buttons.
    Client,
    /// Title bar area that drags the window.
    Caption,
    /// Left resize edge.
    Left,
    /// Right resize edge.
    Right,
    /// Top resize edge.
    Top,
    /// Bottom resize edge.
    Bottom,
    /// Top-left resize corner.
    TopLeft,
    /// Top-right resize corner.
    TopRight,
    /// Bottom-left resize corner.
    BottomLeft,
    /// Bottom-right resize corner.
    BottomRight,
}

// Ici, aucune constante ni aucun appel propre à Windows.
/// Classify `point` within `area` (right and bottom edges excluded). Resize
/// edges are disabled while `maximized`; corners take precedence over edges and
/// caption buttons count as client area so clicking them does not drag.
pub fn hit_test(area: &Rect, point: &Point, maximized: bool) -> HitRegion {
    // Le bord droit et le bord inférieur sont des bornes exclues.
    if point.x < area.left || point.x >= area.right || point.y < area.top || point.y >= area.bottom
    {
        return HitRegion::Outside;
    }

    // Une fenêtre agrandie ne se redimensionne pas en tirant ses bords.
    let resizable = !maximized;
    let left_edge = resizable && point.x < area.left + RESIZE_BORDER;
    let right_edge = resizable && point.x >= area.right - RESIZE_BORDER;
    let top_edge = resizable && point.y < area.top + RESIZE_BORDER;
    let bottom_edge = resizable && point.y >= area.bottom - RESIZE_BORDER;

    // Un coin appartient à deux bords : le vérifier en premier.
    if top_edge && left_edge {
        HitRegion::TopLeft
    } else if top_edge && right_edge {
        HitRegion::TopRight
    } else if bottom_edge && left_edge {
        HitRegion::BottomLeft
    } else if bottom_edge && right_edge {
        HitRegion::BottomRight
    } else if left_edge {
        HitRegion::Left
    } else if right_edge {
        HitRegion::Right
    } else if top_edge {
        HitRegion::Top
    } else if bottom_edge {
        HitRegion::Bottom
    } else if button_at(area, point).is_some() {
        HitRegion::Client // Un clic sur un bouton ne doit pas déplacer la fenêtre.
    } else if point.y < TITLE_BAR_HEIGHT {
        HitRegion::Caption // Le reste de la bande supérieure déplace la fenêtre.
    } else {
        HitRegion::Client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: i32) -> Rect {
        Rect {
            left: 0,
            top: 0,
            right: width,
            bottom: 300,
        }
    }

    #[test]
    fn resize_edges_take_priority_over_title_bar_and_buttons() {
        let area = area(400);
        for (x, y, expected) in [
            (0, 0, HitRegion::TopLeft),
            (399, 0, HitRegion::TopRight),
            (0, 299, HitRegion::BottomLeft),
            (399, 299, HitRegion::BottomRight),
            (0, 150, HitRegion::Left),
            (399, 150, HitRegion::Right),
            (200, 0, HitRegion::Top),
            (200, 299, HitRegion::Bottom),
        ] {
            assert_eq!(hit_test(&area, &Point { x, y }, false), expected);
        }
    }

    #[test]
    fn buttons_follow_right_edge_and_remain_client_content() {
        for width in [240, 400, 800] {
            let area = area(width);
            for button in [
                CaptionButton::Minimize,
                CaptionButton::Maximize,
                CaptionButton::Close,
            ] {
                let rect = button_rect(&area, button);
                let center = Point {
                    x: (rect.left + rect.right) / 2,
                    y: TITLE_BAR_HEIGHT / 2,
                };
                assert_eq!(button_at(&area, &center), Some(button));
                assert_eq!(hit_test(&area, &center, false), HitRegion::Client);
            }
            let close = button_rect(&area, CaptionButton::Close);
            assert_eq!(close.right, width - RESIZE_BORDER);
            assert_eq!(
                button_at(
                    &area,
                    &Point {
                        x: close.right,
                        y: 20
                    }
                ),
                None
            );
        }
    }

    #[test]
    fn title_content_and_outside_regions_are_distinct() {
        let area = area(400);
        for (x, y, expected) in [
            (20, 20, HitRegion::Caption),
            (200, 40, HitRegion::Client),
            (-1, 20, HitRegion::Outside),
            (400, 20, HitRegion::Outside),
            (20, -1, HitRegion::Outside),
            (20, 300, HitRegion::Outside),
        ] {
            assert_eq!(hit_test(&area, &Point { x, y }, false), expected);
        }
    }

    #[test]
    fn maximized_window_has_no_resize_edges() {
        let area = area(400);
        assert_eq!(
            hit_test(&area, &Point { x: 0, y: 0 }, true),
            HitRegion::Caption
        );
        assert_eq!(
            hit_test(&area, &Point { x: 399, y: 299 }, true),
            HitRegion::Client
        );
        let close = button_rect(&area, CaptionButton::Close);
        assert_eq!(
            hit_test(
                &area,
                &Point {
                    x: close.left,
                    y: 0
                },
                true
            ),
            HitRegion::Client
        );
    }
}
