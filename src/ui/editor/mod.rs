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
//! is a time line of tracks and keys with a play head, with its [`SequencerState`]. A
//! [`Toolbar`] shows icon buttons, options and exclusive choices, and puts what does
//! not fit in an overflow menu. A [`CurveEditor`] edits curves of keys and slopes, on a
//! 2D view from [`canvas`] (zoom around the pointer, thick Bézier strokes). A
//! [`GraphEditor`] is a canvas of nodes with typed ports and wires, with its
//! [`GraphEditorState`].
pub mod canvas;
mod color;
mod curves;
mod field;
mod graph;
mod label;
mod menu_bar;
mod palette;
mod properties;
mod sequencer;
#[cfg(test)]
mod testing;
mod theme_file;
mod toolbar;
mod tree;

pub use color::{
    ColorEvent, ColorMsg, ColorPicker, ColorPickerState, ColorReply, Hsv, hex_of, parse_hex,
};
pub use curves::{
    Curve, CurveEditor, CurveEditorState, CurveEvent, CurveKey, CurveMsg, CurveRef, CurveReply,
    KeyMode,
};
pub use field::{
    FieldChange, FieldEvent, FieldId, FieldMsg, FieldReply, FieldState, NumberField, VectorField,
};
pub use graph::{
    Graph, GraphComment, GraphEditor, GraphEditorState, GraphEvent, GraphMsg, GraphNode, GraphPort,
    GraphReply, GraphView, GraphWire, NODE_WIDTH, PortRef,
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
pub use toolbar::{ToolItem, Toolbar, ToolbarMsg, ToolbarState};
pub use tree::{
    DropPlace, TreeDrop, TreeEvent, TreeItem, TreeMsg, TreeReply, TreeRow, TreeSource, TreeState,
    TreeView,
};
