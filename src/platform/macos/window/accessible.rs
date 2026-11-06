//! Lien entre le pont NSAccessibility (`accessibility/`) et la session d'une fenêtre :
//! instantané de l'arbre, objets accessibles, coordonnées de l'écran, actions et
//! notifications. Un lecteur d'écran qui interroge la fenêtre active le pont ; sans lui,
//! aucun objet n'est créé ni aucune notification envoyée.
use super::{super::accessibility, session::Session, with};
use crate::{
    ui::{
        element::ElementId,
        runtime::{AccessibilityAction, AccessibleElement},
        scene::RectF,
    },
    window::WindowId,
};
use objc2::{MainThreadMarker, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSAccessibilityFocusedUIElementChangedNotification, NSAccessibilityLayoutChangedNotification,
    NSAccessibilityValueChangedNotification,
};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize};
use std::collections::HashMap;

type Version = (u64, u64, u64, u64, Option<ElementId>);
/// État du pont d'une fenêtre.
#[derive(Default)]
pub(super) struct Access {
    version: Option<Version>,
    snapshot: Vec<AccessibleElement>,
    nodes: HashMap<ElementId, Retained<accessibility::Node>>,
    /// Un lecteur d'écran a interrogé la fenêtre.
    active: bool,
    /// Version déjà annoncée par les notifications.
    notified: Option<Version>,
}
fn object<T: objc2::Message>(value: Retained<T>) -> Retained<AnyObject> {
    // SAFETY : tout objet Objective-C est un `AnyObject`.
    unsafe { Retained::cast_unchecked(value) }
}
impl Session {
    /// Instantané de l'arbre accessible, renouvelé quand sa version change.
    pub(super) fn accessible(&mut self) -> &[AccessibleElement] {
        let version = self.runtime_accessibility_version();
        if self.access.version != Some(version) {
            self.access.snapshot = self.runtime_accessibility();
            self.access.version = Some(version);
            let live = &self.access.snapshot;
            self.access
                .nodes
                .retain(|id, _| live.iter().any(|e| e.id == *id));
        }
        &self.access.snapshot
    }
    fn node(&mut self, id: ElementId) -> Retained<AnyObject> {
        let window = self.id;
        let mtm = MainThreadMarker::from(&*self.window);
        let node = self
            .access
            .nodes
            .entry(id)
            .or_insert_with(|| accessibility::Node::new(mtm, window, id))
            .clone();
        object(node)
    }
    /// Rectangle logique de la vue en coordonnées de l'écran.
    pub(super) fn screen_rect(&self, r: RectF) -> NSRect {
        let rect = NSRect::new(
            NSPoint::new(f64::from(r.left), f64::from(r.top)),
            NSSize::new(
                f64::from(r.right - r.left).max(0.),
                f64::from(r.bottom - r.top).max(0.),
            ),
        );
        let view = self.view_object();
        self.window
            .convertRectToScreen(view.convertRect_toView(rect, None))
    }
    /// Annonce aux lecteurs d'écran les changements depuis la dernière annonce : focus,
    /// valeurs, mise en page.
    pub(super) fn notify_accessibility(&mut self) {
        if !self.access.active {
            return;
        }
        let version = self.runtime_accessibility_version();
        let Some(previous) = self.access.notified.replace(version) else {
            return;
        };
        if previous == version {
            return;
        }
        // SAFETY : constantes d'AppKit.
        let (focus, value, layout) = unsafe {
            (
                NSAccessibilityFocusedUIElementChangedNotification,
                NSAccessibilityValueChangedNotification,
                NSAccessibilityLayoutChangedNotification,
            )
        };
        if previous.4 != version.4
            && let Some(id) = version.4
        {
            let node = self.node(id);
            accessibility::post(&node, focus);
        }
        if previous.3 != version.3
            && let Some(id) = version.4
        {
            let node = self.node(id);
            accessibility::post(&node, value);
        }
        if previous.0 != version.0 || previous.1 != version.1 || previous.2 != version.2 {
            let view = object(self.view_object());
            accessibility::post(&view, layout);
        }
    }
}
/// Élément `id` de la fenêtre `window`, d'après l'instantané courant.
pub(in crate::platform::macos) fn element(
    window: WindowId,
    id: ElementId,
) -> Option<AccessibleElement> {
    with(window, |s| {
        s.access.active = true;
        Ok(s.accessible().iter().find(|e| e.id == id).cloned())
    })
    .flatten()
}
/// Enfants accessibles de `parent` (ou les racines).
pub(in crate::platform::macos) fn children(
    window: WindowId,
    parent: Option<ElementId>,
) -> Option<Retained<NSArray<AnyObject>>> {
    with(window, |s| {
        s.access.active = true;
        let ids = s
            .accessible()
            .iter()
            .filter(|e| e.parent == parent && !e.offscreen)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        let nodes = ids.into_iter().map(|id| s.node(id)).collect::<Vec<_>>();
        Ok(NSArray::from_retained_slice(&nodes))
    })
}
/// Parent accessible : l'élément parent, ou la vue pour une racine.
pub(in crate::platform::macos) fn parent(
    window: WindowId,
    id: ElementId,
) -> Option<Retained<AnyObject>> {
    with(window, |s| {
        let parent = s
            .accessible()
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.parent);
        Ok(match parent {
            Some(parent) => s.node(parent),
            None => object(s.view_object()),
        })
    })
}
pub(in crate::platform::macos) fn screen(window: WindowId, r: RectF) -> NSRect {
    with(window, |s| Ok(s.screen_rect(r))).unwrap_or(NSRect::ZERO)
}
/// Applique l'action d'un lecteur d'écran ; `false` si le moteur la refuse.
pub(in crate::platform::macos) fn act(window: WindowId, action: AccessibilityAction) -> bool {
    with(window, |s| s.accessibility_action(action)).unwrap_or(false)
}
/// Élément le plus profond sous `point` (écran), ou `None` (la vue elle-même).
pub(in crate::platform::macos) fn hit(
    window: WindowId,
    point: NSPoint,
) -> Option<Retained<AnyObject>> {
    with(window, |s| {
        s.access.active = true;
        let local = s
            .view_object()
            .convertPoint_fromView(s.window.convertPointFromScreen(point), None);
        let (x, y) = (local.x as f32, local.y as f32);
        let found = s
            .accessible()
            .iter()
            .rev()
            .find(|e| {
                let r = e.clipped_bounds;
                !e.offscreen && x >= r.left && x < r.right && y >= r.top && y < r.bottom
            })
            .map(|e| e.id);
        Ok(found.map(|id| s.node(id)))
    })
    .flatten()
}
/// Élément focalisé, ou `None`.
pub(in crate::platform::macos) fn focused(window: WindowId) -> Option<Retained<AnyObject>> {
    with(window, |s| {
        s.access.active = true;
        let found = s.accessible().iter().find(|e| e.focused).map(|e| e.id);
        Ok(found.map(|id| s.node(id)))
    })
    .flatten()
}
