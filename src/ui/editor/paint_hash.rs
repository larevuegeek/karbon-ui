//! Clé de peinture des toiles de l'éditeur (graphe, frise, courbes ; KUI-056) : une
//! empreinte de tout ce qu'elles dessinent, calculée bien plus vite que la peinture
//! (les nombres par leurs bits, les textes par leurs octets).
use crate::ui::{
    scene::{PointF, RectF},
    theme::Color,
};

/// Empreinte FNV-1a sur 64 bits.
pub(super) struct PaintHash(u64);

impl PaintHash {
    pub(super) fn new(seed: u64) -> Self {
        let mut hash = Self(0xcbf2_9ce4_8422_2325);
        hash.u64(seed);
        hash
    }
    pub(super) fn u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    pub(super) fn f32(&mut self, value: f32) {
        self.u64(u64::from(value.to_bits()));
    }
    pub(super) fn bool(&mut self, value: bool) {
        self.u64(u64::from(value));
    }
    pub(super) fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }
    pub(super) fn str(&mut self, text: &str) {
        self.usize(text.len());
        for byte in text.bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    pub(super) fn color(&mut self, color: Color) {
        self.u64(u64::from(u32::from_le_bytes([
            color.red,
            color.green,
            color.blue,
            color.alpha,
        ])));
    }
    pub(super) fn option_color(&mut self, color: Option<Color>) {
        self.bool(color.is_some());
        if let Some(color) = color {
            self.color(color);
        }
    }
    pub(super) fn point(&mut self, point: PointF) {
        self.f32(point.x);
        self.f32(point.y);
    }
    pub(super) fn rect(&mut self, rect: RectF) {
        for value in [rect.left, rect.top, rect.right, rect.bottom] {
            self.f32(value);
        }
    }
    /// Un petit état (vue, geste en cours, palette) par son écriture de mise au point,
    /// exacte pour les nombres.
    pub(super) fn debug(&mut self, value: &impl std::fmt::Debug) {
        use std::fmt::Write;
        let _ = write!(self, "{value:?}");
    }
    pub(super) fn finish(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Write for PaintHash {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        for byte in text.bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
        Ok(())
    }
}
