//! Une piste de la frise : son nom, ses clés, son retrait, sa couleur.
use crate::ui::theme::Color;

/// A track of a [`Sequencer`](super::Sequencer): its name, the times of its keys in seconds (in the order
/// of time), and how far its name is indented (a property under its object).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SequencerTrack {
    /// Its name, in the column on the left.
    pub name: String,
    /// The times of its keys, in seconds, in the order of time.
    pub keys: Vec<f32>,
    /// Steps of indentation of its name.
    pub indent: u8,
    /// Colour of its keys; the text colour of the palette by default.
    pub color: Option<Color>,
}
