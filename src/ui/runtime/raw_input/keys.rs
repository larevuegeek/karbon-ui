//! Toutes les touches du clavier, données à l'application quand aucun champ de saisie
//! n'a le clavier.
use super::super::*;
use super::PhysicalKey;

/// A key of the keyboard, named after what the active layout prints on it. On Windows a
/// letter comes from the virtual-key code, which follows the layout: the key marked Z
/// gives [`KeyCode::Z`] on a French (AZERTY) keyboard as on a US one, so Ctrl+Z means
/// undo for every user. To bind keys by their place instead (WASD on a US keyboard, ZQSD
/// on a French one: the same keys), read [`KeyPress::physical`].
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
    // Ajouts (KUI-012) : ponctuation nommée d'après le caractère de la touche dans la
    // disposition active, pavé numérique, F13 à F24, touches système.
    /// `-` (the character of the key in the active layout).
    Minus,
    /// `=`.
    Equal,
    /// `[`.
    BracketLeft,
    /// `]`.
    BracketRight,
    /// `\`.
    Backslash,
    /// `;`.
    Semicolon,
    /// `'`.
    Quote,
    /// `` ` ``.
    Backquote,
    /// `,`.
    Comma,
    /// `.`.
    Period,
    /// `/`.
    Slash,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    /// The Windows key, or Cmd on a Mac keyboard.
    Meta,
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    /// The key that opens the context menu.
    ContextMenu,
    /// A key that has no name of its own in the active layout (`^`, `ù`, `$` on a French
    /// keyboard, the extra key left of Z): read its place in
    /// [`KeyPress::physical`] to bind it.
    Unidentified,
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
    /// Where the key is on the keyboard, whatever the layout prints on it: the key marked
    /// Z on a French keyboard is [`PhysicalKey::KeyW`], like W on a US one. `None` for a
    /// key the table does not know, and where the system does not give it yet.
    pub physical: Option<PhysicalKey>,
    /// The raw scan code given by the system, `0` when unknown. On Windows, the set-1 code
    /// with `0xE000` added for the extended keys (`0xE048` is the up arrow, `0x48` the 8
    /// of the numeric keypad). Its meaning depends on the system: prefer
    /// [`physical`](Self::physical).
    pub scancode: u32,
    /// The Windows key (Cmd on a Mac keyboard) is held.
    pub meta: bool,
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
            physical: None,
            scancode: 0,
            meta: false,
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
        if self.panel_key(press) {
            self.invalidation.repaint = true;
            result.repaint = true;
            result.handled = true;
            return result;
        }
        if self.app.key(press) {
            self.invalidate();
            result.repaint = true;
        }
        result
    }
}
