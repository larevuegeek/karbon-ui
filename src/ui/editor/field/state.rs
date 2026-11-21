//! État des champs numériques d'une fenêtre : le seul champ glissé ou en saisie à un
//! instant donné, avec la valeur de départ pour l'annulation.
use super::expression::evaluate;
use crate::ui::runtime::{Key, KeyCode, KeyPress, PointerButton, PointerEvent, PointerPhase};

/// Distance, en pixels logiques, au-delà de laquelle un appui devient un glisser.
const DRAG_THRESHOLD: f32 = 3.;
/// Multiplicateurs du pas pendant un glisser : Maj va dix fois plus vite, Ctrl dix fois
/// plus finement.
const COARSE: f64 = 10.;
const FINE: f64 = 0.1;

/// Réglages d'un champ, portés par ses messages : le champ ne garde rien entre deux vues.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Spec {
    pub(super) value: f64,
    pub(super) min: f64,
    pub(super) max: f64,
    pub(super) step: f64,
    pub(super) decimals: usize,
    pub(super) unit: std::sync::Arc<str>,
}

impl Spec {
    /// `value` bornée et arrondie au pas `granularity`.
    pub(super) fn settle(&self, value: f64, granularity: f64) -> f64 {
        let snapped = if granularity > 0. {
            (value / granularity).round() * granularity
        } else {
            value
        };
        // Arrondi à l'affichage : 0.1 + 0.2 ne montre pas 0.30000000000000004.
        let scale = 10f64.powi(self.decimals.min(12) as i32);
        let rounded = (snapped * scale).round() / scale;
        let bounded = rounded.clamp(self.min, self.max);
        if bounded == 0. { 0. } else { bounded }
    }

    /// Le texte affiché : la valeur avec ses décimales et son unité.
    pub(super) fn format(&self, value: f64) -> String {
        format!("{}{}", self.number(value), self.unit)
    }

    /// La valeur seule, comme elle est proposée à la saisie.
    pub(super) fn number(&self, value: f64) -> String {
        let value = if value == 0. { 0. } else { value };
        format!("{value:.*}", self.decimals)
    }
}

/// Identity of a field: the key given to [`NumberField::new`](super::NumberField::new)
/// or [`VectorField::new`](super::VectorField::new), and the axis (0, 1, 2 for X, Y, Z;
/// 0 for a single number).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldId {
    /// Key of the field.
    pub key: u64,
    /// Axis of a vector, `0` for a single number.
    pub axis: usize,
}

/// How a value changed ([`FieldEvent`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldChange {
    /// During a drag: show the value, but do not record it in the undo history yet.
    Preview,
    /// End of the gesture, or a value typed and confirmed: the one change to record.
    Commit,
    /// The gesture was cancelled (Escape): `value` is the value before it.
    Cancel,
}

/// A new value for a field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldEvent {
    /// The field.
    pub field: FieldId,
    /// Its new value, within its bounds and rounded to its decimals.
    pub value: f64,
    /// Preview, commit or cancel.
    pub change: FieldChange,
}

/// Answer of [`FieldState::update`] and [`FieldState::key`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FieldReply {
    /// The fields changed on screen: return `true` from `Application::update`.
    pub rebuild: bool,
    /// The value to store, if any.
    pub event: Option<FieldEvent>,
}

/// A gesture on a number field, to pass to [`FieldState::update`]. The fields build
/// them; an application only forwards them.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMsg(pub(super) Inner);

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Inner {
    Pointer {
        field: FieldId,
        spec: Spec,
        event: PointerEvent,
    },
    Text(String),
    Key(Key),
    /// Le champ de saisie a pris (`true`) ou perdu le clavier.
    Focus(bool),
}

#[derive(Clone, Debug)]
pub(super) struct Drag {
    pub(super) field: FieldId,
    pub(super) spec: Spec,
    pub(super) last_x: f32,
    pub(super) travel: f32,
    /// Valeur non arrondie, accumulée mouvement après mouvement.
    pub(super) raw: f64,
    pub(super) shown: f64,
}

#[derive(Clone, Debug)]
pub(super) struct Edit {
    pub(super) field: FieldId,
    pub(super) spec: Spec,
    pub(super) text: String,
}

/// State of the number fields of a window, kept by the application: the field being
/// dragged or typed into, if any (one at a time). Drag a field to change its value
/// (Shift: ten times faster, Ctrl: ten times finer, Escape: back to the value before),
/// click it to type a value or an expression (`2*3`) in its place, confirmed with Enter,
/// Tab or a click elsewhere, cancelled with Escape.
///
/// ```
/// use karbon_ui::ui::editor::{FieldChange, FieldState};
///
/// let fields = FieldState::default();
/// assert!(!fields.is_active());
/// # let _ = FieldChange::Commit;
/// ```
#[derive(Default)]
pub struct FieldState {
    pub(super) drag: Option<Drag>,
    pub(super) edit: Option<Edit>,
}

impl FieldState {
    /// `true` while a field is dragged or typed into.
    pub fn is_active(&self) -> bool {
        self.drag.is_some() || self.edit.is_some()
    }

    /// The field being typed into, if any.
    pub fn editing(&self) -> Option<FieldId> {
        self.edit.as_ref().map(|edit| edit.field)
    }

    /// Apply a gesture of a field ([`FieldMsg`]).
    pub fn update(&mut self, message: FieldMsg) -> FieldReply {
        match message.0 {
            Inner::Pointer { field, spec, event } => match event.phase {
                PointerPhase::Down if event.button == Some(PointerButton::Primary) => {
                    self.press(field, spec, event)
                }
                PointerPhase::Move => self.drag_to(field, event),
                PointerPhase::Up => self.release(field),
                _ => FieldReply::default(),
            },
            Inner::Text(text) => {
                if let Some(edit) = &mut self.edit {
                    edit.text = text;
                }
                FieldReply {
                    rebuild: true,
                    event: None,
                }
            }
            Inner::Key(Key::Enter | Key::Tab) | Inner::Focus(false) => self.confirm(),
            Inner::Key(Key::Escape) => self.cancel_edit(),
            Inner::Key(_) | Inner::Focus(true) => FieldReply::default(),
        }
    }

    /// Apply a key given to [`Application::key`](crate::application::Application::key):
    /// Escape during a drag puts the value back as it was before it. Other keys change
    /// nothing.
    pub fn key(&mut self, press: KeyPress) -> FieldReply {
        if !press.pressed || press.key != KeyCode::Escape {
            return FieldReply::default();
        }
        let Some(drag) = self.drag.take() else {
            return FieldReply::default();
        };
        FieldReply {
            rebuild: true,
            event: Some(FieldEvent {
                field: drag.field,
                value: drag.spec.value,
                change: FieldChange::Cancel,
            }),
        }
    }

    fn press(&mut self, field: FieldId, spec: Spec, event: PointerEvent) -> FieldReply {
        // Un appui sur un autre champ pendant une saisie la confirme d'abord.
        let confirmed = self.confirm();
        self.drag = Some(Drag {
            field,
            raw: spec.value,
            shown: spec.value,
            spec,
            last_x: event.x,
            travel: 0.,
        });
        FieldReply {
            rebuild: true,
            event: confirmed.event,
        }
    }

    fn drag_to(&mut self, field: FieldId, event: PointerEvent) -> FieldReply {
        let Some(drag) = self.drag.as_mut().filter(|d| d.field == field) else {
            return FieldReply::default();
        };
        let dx = event.x - drag.last_x;
        drag.last_x = event.x;
        drag.travel += dx.abs();
        if drag.travel < DRAG_THRESHOLD {
            return FieldReply::default();
        }
        let speed = if event.shift {
            COARSE
        } else if event.control {
            FINE
        } else {
            1.
        };
        let step = drag.spec.step * speed;
        drag.raw = (drag.raw + f64::from(dx) * step).clamp(drag.spec.min, drag.spec.max);
        let value = drag.spec.settle(drag.raw, step);
        if value == drag.shown {
            return FieldReply::default();
        }
        drag.shown = value;
        FieldReply {
            rebuild: true,
            event: Some(FieldEvent {
                field,
                value,
                change: FieldChange::Preview,
            }),
        }
    }

    fn release(&mut self, field: FieldId) -> FieldReply {
        let Some(drag) = self.drag.take_if(|d| d.field == field) else {
            return FieldReply::default();
        };
        if drag.travel < DRAG_THRESHOLD {
            // Un clic : la saisie au clavier, avec la valeur sans unité.
            let text = drag.spec.number(drag.spec.value);
            self.edit = Some(Edit {
                field,
                spec: drag.spec,
                text,
            });
            return FieldReply {
                rebuild: true,
                event: None,
            };
        }
        FieldReply {
            rebuild: true,
            event: Some(FieldEvent {
                field,
                value: drag.shown,
                change: FieldChange::Commit,
            }),
        }
    }

    /// Fin de la saisie : la valeur tapée si elle se lit, sinon celle d'avant.
    fn confirm(&mut self) -> FieldReply {
        let Some(edit) = self.edit.take() else {
            return FieldReply::default();
        };
        let value = evaluate(&edit.text, &edit.spec.unit)
            .map_or(edit.spec.value, |typed| edit.spec.settle(typed, 0.));
        FieldReply {
            rebuild: true,
            event: Some(FieldEvent {
                field: edit.field,
                value,
                change: FieldChange::Commit,
            }),
        }
    }

    fn cancel_edit(&mut self) -> FieldReply {
        match self.edit.take() {
            Some(_) => FieldReply {
                rebuild: true,
                event: None,
            },
            None => FieldReply::default(),
        }
    }
}
