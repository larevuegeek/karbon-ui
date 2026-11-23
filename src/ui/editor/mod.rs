//! Components for dense editing tools (3D editors, level editors, inspectors), drawn
//! with an [`EditorPalette`]: a virtualized [`TreeView`] of 100,000 nodes, a
//! [`NumberField`] changed by dragging and its [`VectorField`], a [`PropertyGrid`] of
//! labelled controls in folding sections, and a [`MenuBar`] with submenus, check marks
//! and shortcuts.
//!
//! They follow the rest of the library: the application keeps their state (a
//! [`TreeState`], a [`FieldState`], a [`MenuBarState`]), the view builds them from it,
//! and their messages come back to `Application::update`. Mouse gestures go through
//! [`Element::on_pointer`](crate::ui::element::Element::on_pointer) and keys through
//! [`Application::key`](crate::application::Application::key), delivered on Windows only
//! for now. See `docs/editor.md`.
//! A [`ColorPicker`] chooses a colour: a square of saturation and value, a hue bar, an
//! opacity bar and the hexadecimal value, with its [`ColorPickerState`]. A [`Sequencer`]
//! is a time line of tracks and keys with a play head, with its [`SequencerState`].
mod color;
mod field;
mod label;
mod menu_bar;
mod palette;
mod properties;
mod sequencer;
#[cfg(test)]
mod testing;
mod tree;

pub use color::{
    ColorEvent, ColorMsg, ColorPicker, ColorPickerState, ColorReply, Hsv, hex_of, parse_hex,
};
pub use field::{
    FieldChange, FieldEvent, FieldId, FieldMsg, FieldReply, FieldState, NumberField, VectorField,
};
pub use label::{label, tip};
pub use menu_bar::{
    Check, Menu, MenuBar, MenuBarMsg, MenuBarState, MenuEntry, MenuReply, context_menu,
};
pub use palette::{EditorPalette, contrast};
pub use properties::{
    Property, PropertyGrid, PropertySection, checkbox, choice, color_swatch, read_only, text_field,
};
pub use sequencer::{
    KeyRef, Sequencer, SequencerEvent, SequencerMsg, SequencerReply, SequencerState, SequencerTrack,
};
pub use tree::{
    DropPlace, TreeDrop, TreeEvent, TreeItem, TreeMsg, TreeReply, TreeRow, TreeSource, TreeState,
    TreeView,
};
