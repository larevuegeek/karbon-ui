//! Petit lecteur CSS strict : aucune déclaration non prise en charge n'est écartée
//! en silence.
mod names;
mod scan;
pub use names::{class_name, class_symbol};
pub use scan::split;
mod reader;
pub use reader::*;
