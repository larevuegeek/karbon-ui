//! Éditeur de courbes (KUI-038) : clés, pentes, façons d'aller d'une clé à l'autre,
//! plusieurs courbes, cadrage, un pas d'historique par geste.
mod model;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use model::{Curve, CurveKey, KeyMode};
pub use state::{CurveEditorState, CurveEvent, CurveMsg, CurveRef, CurveReply};
pub use view::CurveEditor;
