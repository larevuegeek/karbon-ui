//! Clé de peinture de la frise (KUI-056) : les pistes et leurs clés, la tête de lecture,
//! la vue, le choix, le geste et la palette.
use super::{Painter, SequencerTrack};
use crate::ui::editor::paint_hash::PaintHash;

impl Painter {
    /// Empreinte de ce que la frise dessine (hors sa place, son thème et son état).
    pub(super) fn key(&self) -> u64 {
        let mut h = PaintHash::new(0x0066_7269_7365);
        h.usize(self.tracks.len());
        for SequencerTrack {
            name,
            keys,
            indent,
            color,
        } in &self.tracks
        {
            h.str(name);
            h.usize(keys.len());
            for time in keys {
                h.f32(*time);
            }
            h.u64(u64::from(*indent));
            h.option_color(*color);
        }
        h.f32(self.time);
        h.bool(self.length.is_some());
        h.f32(self.length.unwrap_or_default());
        h.bool(self.comma);
        h.debug(&self.frame);
        h.debug(&self.selection);
        h.debug(&self.drag);
        h.debug(&self.palette);
        h.finish()
    }
}
