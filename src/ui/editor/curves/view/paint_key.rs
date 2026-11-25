//! Clé de peinture de l'éditeur de courbes (KUI-056) : les courbes, l'état, la palette.
use super::{super::model::*, Painter};
use crate::ui::editor::paint_hash::PaintHash;

impl Painter {
    /// Empreinte de ce que l'éditeur dessine (hors sa place, son thème et son état).
    pub(super) fn key(&self) -> u64 {
        let mut h = PaintHash::new(0x636f_7572_6265);
        h.usize(self.curves.len());
        for Curve { name, color, keys } in &self.curves {
            h.str(name);
            h.option_color(*color);
            h.usize(keys.len());
            for CurveKey {
                time,
                value,
                in_slope,
                out_slope,
                mode,
            } in keys
            {
                for number in [*time, *value, *in_slope, *out_slope] {
                    h.f32(number);
                }
                h.u64(*mode as u64);
            }
        }
        h.debug(&self.state);
        h.debug(&self.palette);
        h.finish()
    }
}
