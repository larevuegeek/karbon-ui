//! Texte Pango : une seule mise en page (`Layout`) sert la mesure, le dessin, les carets
//! de l'édition et les rectangles des liens.
//!
//! Sémantique alignée sur DirectWrite et Core Text : hauteur naturelle d'une ligne =
//! ascendante + descendante de la ligne + interligne de la police de base ; interlignage
//! uniforme avec la ligne de base à `(line - size) / 2 + 0,8 × size` ; largeur mesurée
//! espaces de fin compris ; ellipse de fin, de début (`cut_start`) et limite de lignes.
//! Pango coupe les lignes (UAX #14, mots puis caractères), met en forme par HarfBuzz et
//! justifie ; le placement, l'alignement, les ellipses par ligne et les décorations sont
//! faits ici, comme sous macOS.
mod attributes;
mod draw;
mod glyphs;
mod layout;
mod line;
mod measure;

use super::super::fonts::Fonts;
use crate::{
    platform::shared::{TextCache, Typesetter},
    ui::{element::SizeF, styling::text::RichParagraph},
};
pub(super) use draw::rgba;
pub(in crate::platform::linux) use glyphs::GlyphRun;
pub(super) use layout::Layout;

pub(super) use crate::platform::shared::{plain, simple_format};
/// Mises en page Pango retenues tant qu'elles servent, et tailles mesurées.
pub(super) type Cache = TextCache<Fonts>;
impl Typesetter for Fonts {
    type Layout = Layout;
    fn typeset(
        &mut self,
        paragraph: RichParagraph,
        width: f32,
        wrap: bool,
    ) -> crate::Result<Layout> {
        layout::build(self, paragraph, width, wrap)
    }
    fn size(layout: &Layout) -> SizeF {
        layout.size
    }
    fn left(layout: &Layout) -> f32 {
        layout.lines.iter().map(|l| l.x).fold(0f32, f32::min)
    }
}
