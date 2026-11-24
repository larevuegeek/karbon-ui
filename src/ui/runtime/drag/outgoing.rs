//! Un glisser qui sort de la fenêtre (KUI-047) : des données qui se disent fichiers
//! (`Payload::exported_as`) passent au glisser du système quand le pointeur quitte la
//! fenêtre ; le backend le mène (sous Windows, `DoDragDrop`), puis dit au runtime s'il
//! a été lâché, et la source apprend la fin comme d'habitude.
use super::super::*;
use super::DragPhase;
use super::session::Role;
use std::{path::PathBuf, sync::Arc};

impl UiRuntime {
    /// `p` est hors de la fenêtre (barre de titre comprise).
    pub(super) fn leaves_window(&self, p: PointF) -> bool {
        let Some(context) = self.context else {
            return false;
        };
        let area = super::super::prepare::window_area(context, self.chrome);
        p.x < 0. || p.y < 0. || p.x > area.right || p.y > area.bottom
    }

    /// Le pointeur quitte la fenêtre : la cible qui acceptait apprend son départ, et
    /// les fichiers attendent le backend (`take_outgoing_files`).
    pub(super) fn hand_off(&mut self, p: PointF) -> EventResult {
        if let Some(target) = self.raw_pointer.free_drag.as_ref().and_then(|s| s.over) {
            self.tell(
                target,
                Role::Over,
                DragPhase::Leave,
                p,
                (false, false, false),
            );
        }
        if let Some(session) = self.raw_pointer.free_drag.as_mut() {
            session.over = None;
            session.out = true;
        }
        self.drag_result()
    }

    // Seul le backend Windows mène un glisser du système (et les essais, partout).
    #[cfg_attr(not(windows), allow(dead_code))]
    /// The files of a drag that has just left the window, for the backend to start the
    /// drag of the system, once; `None` otherwise.
    pub(crate) fn take_outgoing_files(&mut self) -> Option<Arc<[PathBuf]>> {
        let session = self.raw_pointer.free_drag.as_mut()?;
        if !session.out || session.handed {
            return None;
        }
        session.handed = true;
        session.payload.exported().map(Arc::from)
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    /// The drag of the system ended: `dropped` when a program took the files. The
    /// source learns the end ([`DragPhase::End`]), and the drag is over.
    pub(crate) fn outgoing_drag_done(&mut self, dropped: bool) -> EventResult {
        if !self.raw_pointer.free_drag.as_ref().is_some_and(|s| s.out) {
            return EventResult::default();
        }
        let at = (self.raw_pointer.last_position).unwrap_or(PointF { x: -1., y: -1. });
        self.tell_source(DragPhase::End, at, (dropped, dropped, !dropped));
        self.raw_pointer.free_drag = None;
        self.drag_result()
    }
}
