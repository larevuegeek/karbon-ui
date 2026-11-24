//! État des sélecteurs de couleur d'une fenêtre : le seul glisser en cours (avec la
//! couleur d'avant, pour Échap), le texte hexadécimal en cours de frappe, et la teinte
//! retenue d'un gris ou d'un noir.
use super::convert::{Hsv, parse_hex};
use crate::ui::{
    editor::{FieldChange, FieldMsg, FieldState},
    runtime::{Key, KeyCode, KeyPress, PointerButton, PointerEvent, PointerPhase},
};

/// Les parties d'un sélecteur qu'on fait glisser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// Le carré : la saturation en travers, la valeur en hauteur.
    Square,
    /// La barre de teinte.
    Hue,
    /// La barre d'opacité.
    Alpha,
}

/// Ce que porte un geste de la souris sur une partie : la couleur montrée quand il a
/// commencé, la taille de la partie peinte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Shown {
    pub(super) key: u64,
    pub(super) hsv: Hsv,
    pub(super) alpha: f32,
    pub(super) size: (f32, f32),
}

/// A gesture on a colour picker, to pass to [`ColorPickerState::update`]. The pickers
/// build them; an application only forwards them.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMsg(pub(super) Inner);

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Inner {
    Pointer {
        part: Part,
        shown: Shown,
        event: PointerEvent,
    },
    /// Le texte hexadécimal tapé, et l'opacité montrée.
    HexText {
        key: u64,
        text: String,
        alpha: f32,
    },
    HexKey(Key),
    /// La zone hexadécimale a pris (`true`) ou perdu le clavier.
    HexFocus(bool),
    /// Un geste sur un champ des canaux (rouge, vert, bleu de 0 à 255) ou de l'intensité,
    /// avec la couleur montrée.
    Channel {
        key: u64,
        rgb: [f32; 3],
        alpha: f32,
        message: FieldMsg,
    },
    /// Clic sur une couleur récente.
    Recent {
        key: u64,
        rgba: [f32; 4],
    },
}

/// A new colour for a picker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorEvent {
    /// The key given to [`ColorPicker::new`](super::ColorPicker::new).
    pub key: u64,
    /// Red, green and blue, from 0 to 1.
    pub rgb: [f32; 3],
    /// Opacity from 0 to 1 (the one given to the picker when it has no opacity bar).
    pub alpha: f32,
    /// Preview during a drag, commit at its end or for a typed value, cancel on Escape
    /// (`rgb` and `alpha` are then the colour before the drag).
    pub change: FieldChange,
}

/// Answer of [`ColorPickerState::update`] and [`ColorPickerState::key`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorReply {
    /// The pickers changed on screen: return `true` from `Application::update`.
    pub rebuild: bool,
    /// The colour to store, if any.
    pub event: Option<ColorEvent>,
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    part: Part,
    before: Shown,
    now: Shown,
}

#[derive(Clone, Debug)]
struct HexEdit {
    key: u64,
    text: String,
    alpha: f32,
}

/// State of the colour pickers of a window, kept by the application: the drag in
/// progress (one at a time), the hexadecimal value being typed, and the hue and
/// saturation last chosen, kept for a colour that has none of its own (a grey keeps the
/// hue it was dragged from). Press in the square or a bar to set the colour there, drag
/// to change it (previews), release to commit; Escape (forwarded to
/// [`ColorPickerState::key`]) puts the colour back as it was before the drag. The
/// hexadecimal box is confirmed by Enter, Tab or a click elsewhere, and left by Escape.
///
/// ```
/// use karbon_ui::ui::editor::ColorPickerState;
///
/// let colors = ColorPickerState::default();
/// assert!(!colors.is_active());
/// ```
#[derive(Default)]
pub struct ColorPickerState {
    drag: Option<Drag>,
    hex: Option<HexEdit>,
    kept: Option<(u64, Hsv)>,
    /// Champs des canaux et de l'intensité (`extras.rs`).
    pub(super) fields: FieldState,
    /// Couleurs validées, la plus récente d'abord.
    pub(super) recent: Vec<[f32; 4]>,
    /// Intensité changée, en attente de `take_intensity`.
    pub(super) intensity: Option<(u64, f32, FieldChange)>,
    /// Dernier sélecteur dont un champ de canal a bougé, et sa couleur (Échap).
    pub(super) last_channel: Option<(u64, [f32; 3], f32)>,
}

impl ColorPickerState {
    /// `true` while a picker is dragged or its hexadecimal value typed.
    pub fn is_active(&self) -> bool {
        self.drag.is_some() || self.hex.is_some()
    }

    /// Apply a gesture of a picker ([`ColorMsg`]). A committed colour joins the recent
    /// colours ([`recent`](Self::recent)).
    pub fn update(&mut self, message: ColorMsg) -> ColorReply {
        let reply = self.apply(message);
        if let Some(event) = reply.event.filter(|e| e.change == FieldChange::Commit) {
            self.remember(event.rgb, event.alpha);
        }
        reply
    }

    fn apply(&mut self, message: ColorMsg) -> ColorReply {
        match message.0 {
            Inner::Pointer { part, shown, event } => match event.phase {
                PointerPhase::Down if event.button == Some(PointerButton::Primary) => {
                    self.press(part, shown, event)
                }
                PointerPhase::Move => self.drag_to(part, event),
                PointerPhase::Up => self.release(part, event),
                _ => ColorReply::default(),
            },
            Inner::HexText { key, text, alpha } => {
                self.hex = Some(HexEdit { key, text, alpha });
                ColorReply {
                    rebuild: true,
                    event: None,
                }
            }
            Inner::HexKey(Key::Enter | Key::Tab) | Inner::HexFocus(false) => self.confirm(),
            Inner::HexKey(Key::Escape) => ColorReply {
                rebuild: self.hex.take().is_some(),
                event: None,
            },
            Inner::HexKey(_) | Inner::HexFocus(true) => ColorReply::default(),
            Inner::Channel {
                key,
                rgb,
                alpha,
                message,
            } => self.channel(key, rgb, alpha, message),
            Inner::Recent { key, rgba } => ColorReply {
                rebuild: true,
                event: Some(ColorEvent {
                    key,
                    rgb: [rgba[0], rgba[1], rgba[2]],
                    alpha: rgba[3],
                    change: FieldChange::Commit,
                }),
            },
        }
    }

    /// Apply a key given to [`Application::key`](crate::application::Application::key):
    /// Escape during a drag puts the colour back as it was before it. Other keys change
    /// nothing.
    pub fn key(&mut self, press: KeyPress) -> ColorReply {
        if !press.pressed || press.key != KeyCode::Escape {
            return ColorReply::default();
        }
        let Some(drag) = self.drag.take() else {
            // Échap pendant le glisser d'un champ de canal : il rend sa valeur.
            let field = self.fields.key(press);
            return match (field.event, self.last_channel) {
                (Some(event), Some((key, rgb, alpha))) => {
                    self.channel_event(key, rgb, alpha, event)
                }
                _ => ColorReply {
                    rebuild: field.rebuild,
                    event: None,
                },
            };
        };
        ColorReply {
            rebuild: true,
            event: Some(event_of(drag.before, FieldChange::Cancel)),
        }
    }

    /// La couleur que montre le sélecteur `key` dont la couleur est `rgb` : celle du
    /// glisser en cours, ou la teinte retenue si elle donne bien `rgb`.
    pub(super) fn shown(&self, key: u64, rgb: [f32; 3]) -> Hsv {
        if let Some(drag) = self.drag.filter(|d| d.now.key == key) {
            return drag.now.hsv;
        }
        let close = |a: [f32; 3]| a.iter().zip(rgb).all(|(x, y)| (x - y).abs() < 1. / 512.);
        match self.kept {
            Some((k, hsv)) if k == key && close(hsv.to_rgb()) => hsv,
            _ => Hsv::from_rgb(rgb),
        }
    }

    /// L'opacité du glisser en cours du sélecteur `key`, s'il y en a un.
    pub(super) fn dragged_alpha(&self, key: u64) -> Option<f32> {
        self.drag.filter(|d| d.now.key == key).map(|d| d.now.alpha)
    }

    /// La partie du sélecteur `key` en cours de glisser.
    pub(super) fn dragging(&self, key: u64) -> Option<Part> {
        self.drag.filter(|d| d.now.key == key).map(|d| d.part)
    }

    /// Le texte hexadécimal en cours de frappe dans le sélecteur `key`.
    pub(super) fn typing(&self, key: u64) -> Option<&str> {
        self.hex
            .as_ref()
            .filter(|h| h.key == key)
            .map(|h| h.text.as_str())
    }

    fn press(&mut self, part: Part, shown: Shown, event: PointerEvent) -> ColorReply {
        if shown.size.0 <= 0. || shown.size.1 <= 0. {
            return ColorReply::default();
        }
        // Un appui pendant une frappe la confirme d'abord : la couleur tapée est
        // dite, le glisser part d'elle au prochain mouvement.
        let typed = self.confirm().event;
        let now = moved(part, shown, event);
        self.drag = Some(Drag {
            part,
            before: shown,
            now,
        });
        ColorReply {
            rebuild: true,
            event: Some(typed.unwrap_or_else(|| event_of(now, FieldChange::Preview))),
        }
    }

    fn drag_to(&mut self, part: Part, event: PointerEvent) -> ColorReply {
        let Some(drag) = self.drag.as_mut().filter(|d| d.part == part) else {
            return ColorReply::default();
        };
        let now = moved(part, drag.now, event);
        if now == drag.now {
            return ColorReply::default();
        }
        drag.now = now;
        ColorReply {
            rebuild: true,
            event: Some(event_of(now, FieldChange::Preview)),
        }
    }

    fn release(&mut self, part: Part, event: PointerEvent) -> ColorReply {
        let Some(drag) = self.drag.take_if(|d| d.part == part) else {
            return ColorReply::default();
        };
        let now = moved(part, drag.now, event);
        self.kept = Some((now.key, now.hsv));
        ColorReply {
            rebuild: true,
            event: Some(event_of(now, FieldChange::Commit)),
        }
    }

    /// Fin de la frappe : la couleur tapée si elle se lit, sinon rien.
    fn confirm(&mut self) -> ColorReply {
        let Some(edit) = self.hex.take() else {
            return ColorReply::default();
        };
        let event = parse_hex(&edit.text).map(|rgb| {
            self.kept = None;
            ColorEvent {
                key: edit.key,
                rgb,
                alpha: edit.alpha,
                change: FieldChange::Commit,
            }
        });
        ColorReply {
            rebuild: true,
            event,
        }
    }
}

/// La couleur `shown` changée par le pointeur sur la partie `part` : la place du
/// pointeur dans la partie, de 0 à 1, en donne la saturation et la valeur, la teinte ou
/// l'opacité.
fn moved(part: Part, shown: Shown, event: PointerEvent) -> Shown {
    let (width, height) = shown.size;
    let x = (event.local_x / width.max(1.)).clamp(0., 1.);
    let y = (event.local_y / height.max(1.)).clamp(0., 1.);
    let mut next = shown;
    match part {
        Part::Square => {
            next.hsv.saturation = x;
            next.hsv.value = 1. - y;
        }
        // Le bout droit de la barre est le rouge du bout gauche : 360° vaut 0°.
        Part::Hue => next.hsv.hue = (x * 360.).min(359.99),
        Part::Alpha => next.alpha = x,
    }
    next
}

fn event_of(shown: Shown, change: FieldChange) -> ColorEvent {
    ColorEvent {
        key: shown.key,
        rgb: shown.hsv.to_rgb(),
        alpha: shown.alpha,
        change,
    }
}
