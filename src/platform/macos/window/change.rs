//! Commandes d'une fenêtre ouverte (`WindowChange`) sous AppKit, état remis à
//! l'application (`Application::window_changed`) et poignée de redimensionnement de
//! l'application (`data-resize-region`), suivie à la main : AppKit n'offre pas de geste
//! de redimensionnement à démarrer, ni sur une fenêtre sans bord redimensionnable.
//!
//! Repère : positions en points, mesurées depuis le coin haut-gauche de la zone utile de
//! l'écran principal, y vers le bas, comme `WindowOptions::x` (`session::place`).
use super::session::Session;
use crate::{
    ui::scene::PointF,
    window::{
        ResizeEdge, WindowChange, WindowId, WindowState,
        change::{Edges, ScreenRect, centered, grip_size, keep_on_screen},
    },
};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSEvent, NSFloatingWindowLevel, NSNormalWindowLevel, NSScreen, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::cell::Cell;

/// Geste en cours sur une poignée : fenêtre, bord, souris et cadre au départ.
#[derive(Clone, Copy)]
struct Grip {
    id: WindowId,
    edge: ResizeEdge,
    mouse: NSPoint,
    frame: NSRect,
}
thread_local! { static GRIP: Cell<Option<Grip>> = const { Cell::new(None) }; }

/// Zone utile de l'écran principal : origine du repère des positions.
fn primary_area(mtm: MainThreadMarker) -> Option<NSRect> {
    NSScreen::mainScreen(mtm).map(|screen| screen.visibleFrame())
}
/// Zone utile de l'écran de la fenêtre.
fn window_area(window: &NSWindow, mtm: MainThreadMarker) -> Option<NSRect> {
    window
        .screen()
        .or_else(|| NSScreen::mainScreen(mtm))
        .map(|screen| screen.visibleFrame())
}
/// Rectangle y vers le bas, relatif au haut de `area`.
fn downward(rect: NSRect, area: NSRect) -> ScreenRect {
    ScreenRect {
        x: rect.origin.x.round() as i32,
        y: (area.origin.y + area.size.height - (rect.origin.y + rect.size.height)).round() as i32,
        width: rect.size.width.round() as i32,
        height: rect.size.height.round() as i32,
    }
}
/// Place le coin haut-gauche de `window` en (`x`, `y`), y vers le bas depuis le haut
/// de `area`.
fn move_top_left(window: &NSWindow, area: NSRect, (x, y): (i32, i32)) {
    window.setFrameTopLeftPoint(NSPoint::new(
        f64::from(x),
        area.origin.y + area.size.height - f64::from(y),
    ));
}
/// Contenu de `width` × `height` points, le coin haut-gauche gardé.
fn set_content_size(window: &NSWindow, width: f64, height: f64) {
    let frame = window.frame();
    let content = window.contentRectForFrameRect(frame);
    let mut target =
        window.frameRectForContentRect(NSRect::new(content.origin, NSSize::new(width, height)));
    target.origin.y = frame.origin.y + frame.size.height - target.size.height;
    window.setFrame_display(target, true);
}
fn bounded(window: &NSWindow, width: f64, height: f64) -> (f64, f64) {
    let (min, max) = (window.contentMinSize(), window.contentMaxSize());
    (
        width.min(max.width).max(min.width),
        height.min(max.height).max(min.height),
    )
}

/// Applique `change` à `window`. À appeler hors de tout emprunt des sessions : AppKit
/// envoie des notifications synchrones (fenêtre devenue clé, taille) que la boucle doit
/// pouvoir traiter (M2 de la revue).
pub(super) fn apply(window: &NSWindow, change: WindowChange, mtm: MainThreadMarker) {
    {
        match change {
            WindowChange::Size { width, height } => {
                let (width, height) = bounded(window, f64::from(width), f64::from(height));
                set_content_size(window, width, height);
            }
            WindowChange::MinSize(size) => {
                let (width, height) = size.unwrap_or((1, 1));
                window.setContentMinSize(NSSize::new(f64::from(width), f64::from(height)));
                enforce_bounds(window);
            }
            WindowChange::MaxSize(size) => {
                let (width, height) =
                    size.map_or((f64::MAX, f64::MAX), |(w, h)| (f64::from(w), f64::from(h)));
                window.setContentMaxSize(NSSize::new(width, height));
                enforce_bounds(window);
            }
            WindowChange::AspectRatio(Some((width, height))) => {
                window.setContentAspectRatio(NSSize::new(f64::from(width), f64::from(height)));
            }
            // Les incréments remplacent les proportions : un point par pas, sans contrainte.
            WindowChange::AspectRatio(None) => {
                window.setContentResizeIncrements(NSSize::new(1., 1.));
            }
            WindowChange::Resizable(resizable) => {
                let mut style = window.styleMask();
                style.set(NSWindowStyleMask::Resizable, resizable);
                window.setStyleMask(style);
            }
            WindowChange::Topmost(topmost) => {
                window.setLevel(if topmost {
                    NSFloatingWindowLevel
                } else {
                    NSNormalWindowLevel
                });
            }
            WindowChange::Position { x, y } => {
                if let Some(area) = primary_area(mtm) {
                    move_top_left(window, area, (x + area.origin.x.round() as i32, y));
                }
            }
            WindowChange::Center | WindowChange::KeepOnScreen => {
                let Some(area) = window_area(window, mtm) else {
                    return;
                };
                let rect = downward(window.frame(), area);
                let bounds = ScreenRect {
                    x: area.origin.x.round() as i32,
                    y: 0,
                    width: area.size.width.round() as i32,
                    height: area.size.height.round() as i32,
                };
                let target = if change == WindowChange::Center {
                    centered(rect.width, rect.height, bounds)
                } else {
                    keep_on_screen(rect, bounds)
                };
                if target != (rect.x, rect.y) {
                    move_top_left(window, area, target);
                }
            }
            WindowChange::Maximized(maximized) => {
                if window.isZoomed() != maximized {
                    window.zoom(None);
                }
            }
            WindowChange::Fullscreen(fullscreen) => {
                if window.styleMask().contains(NSWindowStyleMask::FullScreen) != fullscreen {
                    // Une fenêtre fixe n'entre en plein écran qu'avec ce comportement.
                    window.setCollectionBehavior(
                        window.collectionBehavior() | NSWindowCollectionBehavior::FullScreenPrimary,
                    );
                    window.toggleFullScreen(None);
                }
            }
            WindowChange::Visible(true) => window.orderFront(None),
            WindowChange::Visible(false) => window.orderOut(None),
            WindowChange::Focus => {
                if window.isMiniaturized() {
                    window.deminiaturize(None);
                }
                window.makeKeyAndOrderFront(None);
                #[allow(deprecated)]
                objc2_app_kit::NSApplication::sharedApplication(mtm)
                    .activateIgnoringOtherApps(true);
            }
        }
    }
}
/// Taille actuelle ramenée dans les bornes (après un changement de borne).
fn enforce_bounds(window: &NSWindow) {
    let content = window.contentRectForFrameRect(window.frame()).size;
    let (width, height) = bounded(window, content.width, content.height);
    if (width, height) != (content.width, content.height) {
        set_content_size(window, width, height);
    }
}

impl Session {
    /// Suite d'un changement appliqué par [`apply`] : options gardées par la session et
    /// nouvel état remis à l'application.
    pub fn changed(&mut self, change: WindowChange, mtm: MainThreadMarker) {
        if let WindowChange::Topmost(topmost) = change {
            self.options.topmost = topmost;
        }
        self.report(mtm);
    }
    /// État actuel de la fenêtre.
    pub fn snapshot(&self, mtm: MainThreadMarker) -> WindowState {
        let window = &self.window;
        let frame = window.frame();
        let content = window.contentRectForFrameRect(frame).size;
        let (x, y) = primary_area(mtm).map_or((0, 0), |area| {
            let rect = downward(frame, area);
            (rect.x - area.origin.x.round() as i32, rect.y)
        });
        let style = window.styleMask();
        WindowState {
            x,
            y,
            width: content.width.round() as i32,
            height: content.height.round() as i32,
            scale: window.backingScaleFactor() as f32,
            maximized: window.isZoomed(),
            minimized: window.isMiniaturized(),
            fullscreen: style.contains(NSWindowStyleMask::FullScreen),
            topmost: self.options.topmost,
            resizable: style.contains(NSWindowStyleMask::Resizable),
            dragging: window.inLiveResize()
                || GRIP.with(Cell::get).is_some_and(|grip| grip.id == self.id),
        }
    }
    /// Remet l'état à l'application s'il a changé depuis le dernier envoi.
    pub fn report(&mut self, mtm: MainThreadMarker) {
        let current = self.snapshot(mtm);
        if self.reported == Some(current) {
            return;
        }
        self.reported = Some(current);
        if self.runtime.window_changed(current) {
            self.dirty = true;
        }
    }
    pub fn resize_region_at(&self, point: PointF) -> Option<ResizeEdge> {
        self.runtime.resize_region_at(point)
    }
}

/// Appui sur une poignée de l'application : le geste commence.
pub(super) fn grip_start(id: WindowId, window: &NSWindow, edge: ResizeEdge) {
    GRIP.with(|g| {
        g.set(Some(Grip {
            id,
            edge,
            mouse: NSEvent::mouseLocation(),
            frame: window.frame(),
        }))
    });
}
/// Souris déplacée pendant le geste : nouveau cadre ; `false` sans geste en cours.
pub(super) fn grip_drag(id: WindowId, window: &NSWindow) -> bool {
    let Some(grip) = GRIP.with(Cell::get).filter(|g| g.id == id) else {
        return false;
    };
    let mouse = NSEvent::mouseLocation();
    // Repère AppKit : y vers le haut ; la règle commune compte y vers le bas.
    let delta = (
        (mouse.x - grip.mouse.x).round() as i32,
        (grip.mouse.y - mouse.y).round() as i32,
    );
    let edges = Edges::of(grip.edge);
    let start = window.contentRectForFrameRect(grip.frame).size;
    let ratio = window.contentAspectRatio();
    let ratio = (ratio.width > 0. && ratio.height > 0.)
        .then(|| (ratio.width.round() as u32, ratio.height.round() as u32));
    let (min, max) = (window.contentMinSize(), window.contentMaxSize());
    let points = |v: f64| v.round().min(f64::from(i32::MAX / 4)) as i32;
    let (width, height) = grip_size(
        grip.edge,
        (points(start.width), points(start.height)),
        delta,
        ratio,
        (points(min.width), points(min.height)),
        (points(max.width), points(max.height)),
    );
    let (width, height) = (f64::from(width), f64::from(height));
    let mut frame =
        window.frameRectForContentRect(NSRect::new(grip.frame.origin, NSSize::new(width, height)));
    // Le bord opposé au geste reste en place.
    frame.origin.x = if edges.left {
        grip.frame.origin.x + grip.frame.size.width - frame.size.width
    } else {
        grip.frame.origin.x
    };
    frame.origin.y = if edges.top {
        grip.frame.origin.y
    } else {
        grip.frame.origin.y + grip.frame.size.height - frame.size.height
    };
    window.setFrame_display(frame, true);
    true
}
/// Fin de la boucle : plus aucun geste (une boucle relancée ne doit pas retrouver celui
/// d'une ancienne fenêtre de même identité).
pub(super) fn grip_reset() {
    GRIP.with(|g| g.set(None));
}
/// Bouton relâché : fin du geste ; `false` sans geste en cours.
pub(super) fn grip_end(id: WindowId) -> bool {
    GRIP.with(|g| match g.get() {
        Some(grip) if grip.id == id => {
            g.set(None);
            true
        }
        _ => false,
    })
}
