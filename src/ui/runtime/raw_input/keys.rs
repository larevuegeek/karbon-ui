//! Toutes les touches du clavier, données à l'application quand aucun champ de saisie
//! n'a le clavier.
use super::super::*;

/// A key of the keyboard, named after its place on a US layout: [`KeyCode::W`] is the key
/// at the place of W, whatever the layout prints on it (Z on a French keyboard). Games
/// and tools bind keys by place, so that the same hand position works everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    Shift,
    Control,
    Alt,
}

/// A key pressed or released, given to [`Application::key`](crate::application::Application::key)
/// with the state of the modifier keys at that moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeyPress {
    /// The key.
    pub key: KeyCode,
    /// `true` when it goes down, `false` when it is released.
    pub pressed: bool,
    /// The key was already down: the system repeats it while it is held.
    pub repeat: bool,
    /// Shift is held.
    pub shift: bool,
    /// Ctrl is held.
    pub control: bool,
    /// Alt is held.
    pub alt: bool,
}

impl KeyPress {
    /// `key` going down (`pressed`) or up, without repeat nor modifier: a starting point
    /// for backends and tests, which set the other fields.
    pub fn new(key: KeyCode, pressed: bool) -> Self {
        Self {
            key,
            pressed,
            repeat: false,
            shift: false,
            control: false,
            alt: false,
        }
    }
}

impl UiRuntime {
    /// Gives `press` to the application ([`Application::key`](crate::application::Application::key)),
    /// unless a text field has the keyboard focus: typing in a field is the field's. The
    /// key keeps its usual effect on the interface (focus navigation, activation,
    /// Escape), which backends deliver separately as a [`UiEvent`].
    ///
    /// The result asks for a repaint when the application rebuilt its view.
    pub fn key_input(&mut self, press: KeyPress) -> EventResult {
        let mut result = EventResult::default();
        if self.focused_input_geometry().is_some() {
            return result;
        }
        if self.app.key(press) {
            self.invalidate();
            result.repaint = true;
        }
        result
    }
}
