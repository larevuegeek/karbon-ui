//! Hachage rapide des tables internes du moteur (mémoires de layout, de texte et de
//! style), dont les clés sont de petits entiers produits par le moteur lui-même :
//! la résistance aux collisions provoquées de SipHash n'y sert à rien, et son coût
//! pesait sur chaque mesure.
use std::hash::{BuildHasherDefault, Hasher};

/// Hacheur multiplicatif (famille Fx), mot par mot.
#[derive(Default, Clone, Copy)]
pub(crate) struct FastHasher(u64);
const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
impl FastHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}
impl Hasher for FastHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for chunk in chunks {
            self.add(u64::from_le_bytes(*chunk));
        }
        if !rest.is_empty() {
            let mut word = [0u8; 8];
            word[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(word));
        }
    }
    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.add(u64::from(n));
    }
    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }
    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }
    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
/// Table de hachage du moteur, hachée par [`FastHasher`].
pub(crate) type FastMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FastHasher>>;
