//! Le glisser libre en cours : appui sur un élément qui porte des données, seuil de 4
//! unités, cible qui accepte sous le pointeur, lâcher, annulation.
//!
//! Il passe par les entrées brutes du pointeur (`pointer_input`), comme `on_pointer` :
//! l'appui n'est pris que si aucun gestionnaire `on_pointer` ne l'a pris. Une fois le
//! seuil passé, le clic sur la source n'a pas lieu et les gestionnaires `on_pointer` ne
//! voient plus rien jusqu'au relâchement.
use super::super::hit_test::fixed_result;
use super::super::raw_input::{PointerInput, pointer_target};
use super::super::*;
use super::{DragEvent, DragHandler, DragPhase, Payload};

/// Distance (unités logiques) au-delà de laquelle un appui devient un glisser.
const THRESHOLD: f32 = 4.;

/// Un glisser : la source, ses données, le point d'appui, s'il a commencé, la cible qui
/// accepte en ce moment.
pub(crate) struct Session {
    source: ElementId,
    pub(super) payload: Payload,
    start: PointF,
    active: bool,
    pub(super) over: Option<ElementId>,
    /// Le glisser a quitté la fenêtre et passe au système (`outgoing.rs`).
    pub(super) out: bool,
    /// Ses fichiers ont été donnés au backend.
    pub(super) handed: bool,
}

/// Quel gestionnaire d'un élément.
#[derive(Clone, Copy)]
pub(super) enum Role {
    Source,
    Over,
    Drop,
}

fn handler_of(node: &Element, role: Role) -> Option<DragHandler> {
    let listeners = node.listeners.as_ref()?;
    match role {
        Role::Source => listeners.drag.clone(),
        Role::Over => listeners.drag_over.clone(),
        Role::Drop => listeners.drop.clone(),
    }
}

/// `target` et ses ancêtres, de la cible à la racine, avec leur rectangle.
fn path(node: &Element, target: ElementId, out: &mut Vec<(ElementId, RectF)>) -> bool {
    if node.id != target && !node.children.iter().any(|c| path(c, target, out)) {
        return false;
    }
    out.push((node.id, node.rect));
    true
}

/// Les données portées par `start` ou par le plus proche de ses ancêtres qui en porte,
/// avec cet élément.
fn payload_owner(root: &Element, start: ElementId) -> Option<(ElementId, Payload)> {
    let mut ids = Vec::new();
    path(root, start, &mut ids);
    ids.into_iter().find_map(|(id, _)| {
        let node = root.find(id)?;
        let payload = node.listeners.as_ref()?.drag_payload.clone()?;
        Some((id, payload))
    })
}

impl UiRuntime {
    /// Le glisser qui a commencé : les gestionnaires `on_pointer` ne reçoivent rien
    /// pendant qu'il dure, et la fenêtre garde la souris.
    pub(crate) fn free_dragging(&self) -> bool {
        self.raw_pointer
            .free_drag
            .as_ref()
            .is_some_and(|s| s.active)
    }

    /// Un appui au bouton principal que nul `on_pointer` n'a pris : un glisser peut
    /// commencer s'il est sur des données (`Element::drag_payload`).
    pub(crate) fn drag_press(&mut self, input: &PointerInput) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let scope = layers::active(root);
        let p = input.position;
        let Some(found) = fixed_result(scope, p, &self.motions, true, pointer_target)
            .or_else(|| pointer_target(scope, p, &self.motions, true))
        else {
            return;
        };
        self.raw_pointer.free_drag = payload_owner(scope, found).map(|(source, payload)| Session {
            source,
            payload,
            start: p,
            active: false,
            over: None,
            out: false,
            handed: false,
        });
    }

    /// Un mouvement ou un relâchement pendant un glisser possible : `Some` quand le
    /// glisser l'a pris (les gestionnaires `on_pointer` ne le voient pas).
    pub(crate) fn drag_input(&mut self, input: &PointerInput) -> Option<EventResult> {
        let p = input.position;
        let leaving = input.phase == PointerPhase::Move && self.leaves_window(p);
        let session = self.raw_pointer.free_drag.as_mut()?;
        if session.out {
            // Le système mène le glisser : rien n'arrive ici avant sa fin. Sans backend
            // qui le prenne (hors de Windows), le relâchement le finit, sans dépôt.
            let released = input.phase == PointerPhase::Up && !session.handed;
            return Some(if released {
                self.drag_finish(p, false)
            } else {
                EventResult::default()
            });
        }
        let exported = session.payload.exported().is_some();
        match (input.phase, input.button) {
            (PointerPhase::Move, _) => {
                if !session.active {
                    let (dx, dy) = (p.x - session.start.x, p.y - session.start.y);
                    if dx.hypot(dy) <= THRESHOLD {
                        return None;
                    }
                    session.active = true;
                    // Le clic sur la source n'aura pas lieu.
                    self.interaction.pressed = None;
                    self.tell_source(DragPhase::Start, p, (false, false, false));
                }
                if leaving && exported {
                    return Some(self.hand_off(p));
                }
                let accepted = self.follow_targets(p);
                self.tell_source(DragPhase::Move, p, (accepted, false, false));
                Some(self.drag_result())
            }
            (PointerPhase::Up, Some(PointerButton::Primary)) => {
                let active = session.active;
                if !active {
                    self.raw_pointer.free_drag = None;
                    return None;
                }
                Some(self.drag_finish(p, false))
            }
            _ => None,
        }
    }

    /// Échap, ou la fenêtre qui perd le clavier : le glisser s'arrête sans lâcher.
    pub(crate) fn drag_cancel(&mut self) -> Option<EventResult> {
        let session = self.raw_pointer.free_drag.as_ref()?;
        if !session.active {
            self.raw_pointer.free_drag = None;
            return None;
        }
        let at = self.raw_pointer.last_position.unwrap_or(session.start);
        Some(self.drag_finish(at, true))
    }

    /// Le lâcher (ou l'annulation) : la cible qui accepte le reçoit, la source apprend la
    /// fin, et le glisser est oublié.
    pub(super) fn drag_finish(&mut self, p: PointF, cancelled: bool) -> EventResult {
        let over = self.raw_pointer.free_drag.as_ref().and_then(|s| s.over);
        let mut dropped = false;
        if let Some(target) = over {
            if cancelled {
                self.tell(
                    target,
                    Role::Over,
                    DragPhase::Leave,
                    p,
                    (false, false, true),
                );
            } else {
                dropped = self.tell(target, Role::Drop, DragPhase::Drop, p, (true, true, false));
            }
        }
        let accepted = over.is_some() && !cancelled;
        self.tell_source(DragPhase::End, p, (accepted, dropped, cancelled));
        self.raw_pointer.free_drag = None;
        self.drag_result()
    }

    /// Ce que dit un glisser au backend : pris, à repeindre.
    pub(super) fn drag_result(&mut self) -> EventResult {
        self.invalidation.repaint = true;
        EventResult {
            handled: true,
            repaint: true,
            ..EventResult::default()
        }
    }

    /// Demande aux éléments sous `p` (du plus profond à la racine) s'ils acceptent les
    /// données ; la cible d'avant qui n'accepte plus apprend qu'il est parti. Rend si une
    /// cible accepte.
    fn follow_targets(&mut self, p: PointF) -> bool {
        let candidates = self.drag_candidates(p);
        let previous = self.raw_pointer.free_drag.as_ref().and_then(|s| s.over);
        let mut accepting = None;
        for id in candidates {
            if self.tell(id, Role::Over, DragPhase::Over, p, (false, false, false)) {
                accepting = Some(id);
                break;
            }
        }
        if let Some(old) = previous.filter(|&old| Some(old) != accepting) {
            self.tell(old, Role::Over, DragPhase::Leave, p, (false, false, false));
        }
        if let Some(session) = self.raw_pointer.free_drag.as_mut() {
            session.over = accepting;
        }
        accepting.is_some()
    }

    /// Les éléments sous `p` qui ont un gestionnaire `on_drag_over`, du plus profond à
    /// la racine.
    fn drag_candidates(&self, p: PointF) -> Vec<ElementId> {
        let Some(root) = self.root.as_ref() else {
            return Vec::new();
        };
        let scope = layers::active(root);
        let Some(found) = fixed_result(scope, p, &self.motions, true, pointer_target)
            .or_else(|| pointer_target(scope, p, &self.motions, true))
        else {
            return Vec::new();
        };
        let mut ids = Vec::new();
        path(scope, found, &mut ids);
        ids.into_iter()
            .filter(|(id, _)| {
                scope
                    .find(*id)
                    .is_some_and(|n| handler_of(n, Role::Over).is_some())
            })
            .map(|(id, _)| id)
            .collect()
    }

    pub(super) fn tell_source(&mut self, phase: DragPhase, p: PointF, flags: (bool, bool, bool)) {
        if let Some(source) = self.raw_pointer.free_drag.as_ref().map(|s| s.source) {
            self.tell(source, Role::Source, phase, p, flags);
        }
    }

    /// Donne l'étape `phase` au gestionnaire `role` de l'élément `id` ; rend s'il a
    /// répondu par un message (envoyé). `flags` : accepté, lâché, annulé.
    pub(super) fn tell(
        &mut self,
        id: ElementId,
        role: Role,
        phase: DragPhase,
        p: PointF,
        (accepted, dropped, cancelled): (bool, bool, bool),
    ) -> bool {
        let Some(session) = self.raw_pointer.free_drag.as_ref() else {
            return false;
        };
        let found = self.root.as_ref().and_then(|root| {
            let node = root.find(id)?;
            Some((handler_of(node, role)?, node.rect))
        });
        let Some((handler, rect)) = found else {
            return false;
        };
        let event = DragEvent {
            phase,
            payload: session.payload.clone(),
            x: p.x,
            y: p.y,
            local_x: p.x - rect.left,
            local_y: p.y - rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            screen: self.raw_pointer.extras.on_screen(p),
            accepted,
            dropped,
            cancelled,
        };
        match (handler.0)(&event) {
            Some(message) => {
                self.send(message);
                true
            }
            None => false,
        }
    }
}
