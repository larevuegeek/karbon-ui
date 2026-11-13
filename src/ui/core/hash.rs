//! Hachage rapide des tables internes du moteur (mémoires de layout, de texte et de
//! style), dont les clés sont des entiers : identifiants d'éléments (empreintes des
//! gabarits, ou nombres choisis par l'application), mesures, indices. La résistance aux
//! collisions provoquées de SipHash n'y sert à rien (les clés ne viennent pas de
//! l'extérieur), et son coût pesait sur chaque mesure.
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
    /// La multiplication ne propage les bits que vers le haut, et la table range une clé
    /// selon ses bits bas : sans rotation, des identifiants qui ne diffèrent que par leurs
    /// bits hauts (`(ligne << 32) | colonne`) tombaient tous dans le même groupe.
    #[inline]
    fn finish(&self) -> u64 {
        self.0.rotate_left(26)
    }
}
/// Table de hachage du moteur, hachée par [`FastHasher`].
pub(crate) type FastMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FastHasher>>;
/// Ensemble du moteur, haché par [`FastHasher`].
pub(crate) type FastSet<K> = std::collections::HashSet<K, BuildHasherDefault<FastHasher>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::BuildHasher;

    #[test]
    fn keys_differing_only_in_high_bits_spread_over_the_buckets() {
        // Identifiants de l'application qui ne diffèrent que par leurs bits hauts
        // (`(ligne << 32) | colonne`) : la table range une clé selon les bits bas.
        let build = BuildHasherDefault::<FastHasher>::default();
        let buckets = (0..4096u64)
            .map(|i| build.hash_one(i << 32) & 0xfff)
            .collect::<FastSet<_>>();

        let start = std::time::Instant::now();
        let map = (0..20_000u64)
            .map(|i| (i << 32, i))
            .collect::<FastMap<_, _>>();
        let found = (0..20_000u64)
            .filter(|i| map.contains_key(&(i << 32)))
            .count();
        // 46 ms sans rotation (sondage linéaire depuis le même groupe), < 1 ms avec.
        eprintln!("20 000 clés : {:?}", start.elapsed());
        assert_eq!(found, 20_000);
        assert!(buckets.len() > 1024, "{} groupes sur 4096", buckets.len());
    }
}
