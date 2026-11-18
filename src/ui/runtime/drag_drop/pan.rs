//! Déplacement libre (`onpan`) : un bouton qu'on fait glisser suit le pointeur.
//! L'application reçoit la position au début, à chaque mouvement et à la fin ; une
//! reconstruction de la vue n'interrompt pas le geste.
//!
//! Dans un bloc `data-pan-frame` ([`Element::pan_frame`]), le moteur déplace lui-même ce
//! bloc pendant le geste (placement translaté, sans reconstruire ni redisposer : une
//! simple peinture par mouvement), borné à la fenêtre. Au relâchement, l'application
//! donne la place finale ; le bloc y glisse depuis le point de lâcher.
use super::super::*;

/// Distance (unités logiques) au-delà de laquelle un appui devient un déplacement.
const THRESHOLD: f32 = 4.;
/// Glissement du bloc vers sa place finale, après le relâchement.
const SETTLE: f32 = 0.22;
const FRAME: std::time::Duration = std::time::Duration::from_nanos(16_666_667);

/// Phase of a pan gesture ([`PanEvent`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanPhase {
    /// The pointer moved past the threshold: the press becomes a pan and the click is
    /// cancelled.
    Start,
    /// The pointer moved again.
    Move,
    /// The button was released, or the gesture was cancelled (Escape, focus or capture
    /// loss).
    End,
}

/// Press-and-drag over a pannable button (`onpan`, [`Element::on_pan`]).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PanEvent {
    /// Start, move or end of the gesture.
    pub phase: PanPhase,
    /// Pointer position in window coordinates, logical pixels.
    pub x: f32,
    /// Pointer position in window coordinates, logical pixels.
    pub y: f32,
    /// Motion since the press, logical pixels; inside a `pan_frame`, bounded so that the
    /// frame stays in the window (the frame moved by exactly this much).
    pub dx: f32,
    /// Motion since the press, logical pixels (bounded like `dx`).
    pub dy: f32,
    /// Width of the button's parent when the gesture started, logical pixels: the column
    /// or panel that a resize handle sizes.
    pub parent_width: f32,
    /// Height of the button's parent when the gesture started, logical pixels.
    pub parent_height: f32,
}

/// Bloc déplacé par le moteur : son coin au début du geste et les décalages permis.
#[derive(Clone, Copy)]
struct Frame {
    id: ElementId,
    origin: PointF,
    /// Décalages permis : gauche, droite, haut, bas (le bloc reste dans la fenêtre).
    limits: [f32; 4],
}

/// Geste en cours : le bouton appuyé, le point d'appui et le dernier point vu.
#[derive(Clone, Copy)]
pub(in crate::ui::runtime) struct PanState {
    id: ElementId,
    start: PointF,
    last: PointF,
    active: bool,
    frame: Option<Frame>,
    /// Décalage appliqué au bloc (borné).
    offset: (f32, f32),
    /// Taille du parent du bouton à l'appui.
    parent: (f32, f32),
}

/// Bloc qui glisse vers la place donnée par l'application après le relâchement.
#[derive(Clone, Copy)]
pub(in crate::ui::runtime) struct PanSettle {
    id: ElementId,
    /// Coin du bloc au relâchement.
    from: PointF,
    /// Place finale et écart de départ, connus à la disposition qui suit le relâchement.
    to: Option<(PointF, (f32, f32))>,
    start: Instant,
    next: Instant,
}

fn path(node: &Element, id: ElementId, out: &mut Vec<ElementId>) -> bool {
    out.push(node.id);
    if node.id == id || node.children.iter().any(|c| path(c, id, out)) {
        return true;
    }
    out.pop();
    false
}

fn corner(root: &Element, id: ElementId) -> Option<PointF> {
    root.find(id).map(|n| PointF {
        x: n.rect.left,
        y: n.rect.top,
    })
}

/// Amène le coin du bloc `id` en `to` en translatant son placement (comme un défilement).
fn place(root: &mut Element, id: ElementId, to: PointF) -> bool {
    let mut ids = Vec::new();
    if !path(root, id, &mut ids) {
        return false;
    }
    let clip = match ids.len().checked_sub(2).and_then(|i| root.find(ids[i])) {
        Some(parent) => {
            let node = parent.children.iter().find(|c| c.id == id);
            if node.is_some_and(layout::out_of_flow) {
                parent.clip
            } else {
                layout::child_clip(parent, layout::content_rect(parent))
            }
        }
        None => root.clip,
    };
    let Some(node) = root.find_mut(id) else {
        return false;
    };
    let (dx, dy) = (to.x - node.rect.left, to.y - node.rect.top);
    if dx.abs() < 0.01 && dy.abs() < 0.01 {
        return false;
    }
    layout::scroll::translate(node, dx, dy, clip);
    true
}

fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

impl UiRuntime {
    /// Appui sur un bouton : suivi jusqu'au relâchement s'il porte `onpan`.
    pub(in crate::ui::runtime) fn pan_begin(&mut self, id: ElementId, p: PointF) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let pannable = root
            .find(id)
            .and_then(|n| n.listeners.as_deref())
            .is_some_and(|l| l.pan.is_some());
        if !pannable {
            return;
        }
        let mut ids = Vec::new();
        path(root, id, &mut ids);
        let frame_id = ids
            .iter()
            .rev()
            .copied()
            .find(|i| root.find(*i).is_some_and(|n| n.pan_frame));
        // Un bloc qui glissait encore est posé d'abord à sa place.
        if let Some(settle) = self.components.pan_settle.take()
            && let Some((to, _)) = settle.to
            && let Some(root) = self.root.as_mut()
        {
            place(root, settle.id, to);
        }
        let root = self.root.as_ref().unwrap();
        let parent = ids
            .len()
            .checked_sub(2)
            .and_then(|i| root.find(ids[i]))
            .map_or((0., 0.), |n| {
                (n.rect.right - n.rect.left, n.rect.bottom - n.rect.top)
            });
        let frame = frame_id.and_then(|fid| {
            let r = root.find(fid)?.rect;
            let window = root.rect;
            Some(Frame {
                id: fid,
                origin: PointF {
                    x: r.left,
                    y: r.top,
                },
                limits: [
                    (window.left - r.left).min(0.),
                    (window.right - r.right).max(0.),
                    (window.top - r.top).min(0.),
                    (window.bottom - r.bottom).max(0.),
                ],
            })
        });
        self.components.pan = Some(PanState {
            id,
            start: p,
            last: p,
            active: false,
            frame,
            offset: (0., 0.),
            parent,
        });
    }

    /// Mouvement du pointeur ; `true` quand un geste est en cours (l'événement est pris).
    pub(in crate::ui::runtime) fn pan_move(&mut self, p: PointF) -> bool {
        let Some(mut state) = self.components.pan else {
            return false;
        };
        if !p.x.is_finite() || !p.y.is_finite() {
            return state.active;
        }
        let phase = if state.active {
            PanPhase::Move
        } else if (p.x - state.start.x).hypot(p.y - state.start.y) >= THRESHOLD {
            // Le geste remplace le clic.
            self.interaction.pressed = None;
            state.active = true;
            PanPhase::Start
        } else {
            return false;
        };
        let (mut dx, mut dy) = (p.x - state.start.x, p.y - state.start.y);
        if let Some(f) = state.frame {
            dx = dx.clamp(f.limits[0], f.limits[1]);
            dy = dy.clamp(f.limits[2], f.limits[3]);
        }
        state.last = p;
        state.offset = (dx, dy);
        self.components.pan = Some(state);
        self.pan_follow_frame();
        self.pan_emit(state, phase, p);
        true
    }

    /// Relâchement ou annulation ; `true` si un geste était en cours (pas de clic).
    pub(in crate::ui::runtime) fn pan_end(&mut self) -> bool {
        let Some(state) = self.components.pan.take() else {
            return false;
        };
        if !state.active {
            return false;
        }
        if let Some(f) = state.frame
            && let Some(from) = self.root.as_ref().and_then(|r| corner(r, f.id))
        {
            let now = Instant::now();
            self.components.pan_settle = Some(PanSettle {
                id: f.id,
                from,
                to: None,
                start: now,
                next: now,
            });
        }
        self.pan_emit(state, PanPhase::End, state.last);
        true
    }

    /// Le bloc suit le geste (après un mouvement, ou après une disposition qui l'a remis à
    /// la place donnée par l'application).
    fn pan_follow_frame(&mut self) {
        let Some(state) = self.components.pan.filter(|s| s.active) else {
            return;
        };
        let (Some(f), Some(root)) = (state.frame, self.root.as_mut()) else {
            return;
        };
        let to = PointF {
            x: f.origin.x + state.offset.0,
            y: f.origin.y + state.offset.1,
        };
        if place(root, f.id, to) {
            self.invalidation.repaint = true;
        }
    }

    /// Après une disposition : le bloc en cours de geste reprend sa place sous le pointeur ;
    /// après un relâchement, il part du point de lâcher vers sa nouvelle place.
    pub(in crate::ui::runtime) fn pan_after_layout(&mut self, now: Instant) {
        self.pan_follow_frame();
        let Some(mut settle) = self.components.pan_settle else {
            return;
        };
        let Some(root) = self.root.as_mut() else {
            return;
        };
        let Some(here) = corner(root, settle.id) else {
            self.components.pan_settle = None;
            return;
        };
        let (to, gap) = match settle.to {
            Some(known) => known,
            None => {
                let gap = (settle.from.x - here.x, settle.from.y - here.y);
                if gap.0.abs() < 0.5 && gap.1.abs() < 0.5 {
                    self.components.pan_settle = None;
                    return;
                }
                settle.start = now;
                settle.next = now;
                (here, gap)
            }
        };
        settle.to = Some((to, gap));
        self.components.pan_settle = Some(settle);
        self.pan_settle_place(now);
    }

    /// Place le bloc qui glisse à l'instant `now` ; `true` s'il a bougé.
    fn pan_settle_place(&mut self, now: Instant) -> bool {
        let Some(settle) = self.components.pan_settle else {
            return false;
        };
        let Some((to, gap)) = settle.to else {
            return false;
        };
        let t = (now.saturating_duration_since(settle.start).as_secs_f32() / SETTLE).min(1.);
        let rest = 1. - ease_out(t);
        let at = PointF {
            x: to.x + gap.0 * rest,
            y: to.y + gap.1 * rest,
        };
        let moved = self
            .root
            .as_mut()
            .is_some_and(|root| place(root, settle.id, at));
        if t >= 1. {
            self.components.pan_settle = None;
        } else if let Some(s) = &mut self.components.pan_settle {
            s.next = now + FRAME;
        }
        moved
    }

    /// Une image du glissement vers la place finale ; `true` si le bloc a bougé.
    pub(in crate::ui::runtime) fn pan_settle_tick(&mut self, now: Instant) -> bool {
        if self
            .components
            .pan_settle
            .is_none_or(|s| s.to.is_none() || s.next > now)
        {
            return false;
        }
        self.pan_settle_place(now)
    }

    /// Prochaine image du glissement vers la place finale.
    pub(in crate::ui::runtime) fn pan_settle_wake(&self) -> Option<Instant> {
        self.components
            .pan_settle
            .filter(|s| s.to.is_some())
            .map(|s| s.next)
    }

    fn pan_emit(&mut self, state: PanState, phase: PanPhase, p: PointF) {
        let Some(handler) = self
            .root
            .as_ref()
            .and_then(|r| r.find(state.id))
            .and_then(|n| n.listeners.as_deref()?.pan.clone())
        else {
            return;
        };
        let event = PanEvent {
            phase,
            x: p.x,
            y: p.y,
            dx: state.offset.0,
            dy: state.offset.1,
            parent_width: state.parent.0,
            parent_height: state.parent.1,
        };
        if self.app.update(handler.emit(event)) {
            self.invalidate();
        }
        self.invalidation.repaint = true;
    }
}
