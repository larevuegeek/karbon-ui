//! Texte Core Text : une seule mise en page (`Layout`) sert la mesure, le dessin, les
//! carets de l'édition et les rectangles des liens.
//!
//! Sémantique alignée sur DirectWrite (backend Windows) : hauteur naturelle d'une ligne
//! = ascendante + descendante + interligne de la police ; interlignage uniforme avec la
//! ligne de base à `(line - size) / 2 + 0,8 × size` ; largeur mesurée espaces de fin
//! compris ; ellipse de fin, de début (`cut_start`) et limite de lignes.
mod attributed;
mod draw;
mod glyphs;
mod layout;
mod line;
mod measure;
pub(in crate::platform::macos) use glyphs::{GlyphRun, colored, same_font};
pub(super) use layout::Layout;

pub(super) use crate::platform::shared::{plain, simple_format};
mod cache;
pub(crate) use cache::*;
