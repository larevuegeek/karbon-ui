//! Empreintes des caches des renderers : pixels relus (`backdrop-filter`) et repère
//! d'une couche (groupes filtrés).

/// Empreinte FNV-1a de pixels (comparaison d'une image à l'autre, pas de sécurité).
pub(in crate::platform) fn hash(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    let (words, rest) = bytes.as_chunks::<8>();
    for word in words {
        h = (h ^ u64::from_le_bytes(*word)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    for byte in rest {
        h = (h ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
/// Bits du repère d'une couche (translation quantifiée au quart de pixel) et de sa taille
/// (aussi clé des groupes filtrés du renderer Metal).
pub(in crate::platform) fn placement(
    transform: crate::ui::rendering::transform::Affine,
    width: usize,
    height: usize,
) -> [u32; 8] {
    let quarter = |v: f32| ((v * 4.).round() / 4.).to_bits();
    [
        transform.m11.to_bits(),
        transform.m12.to_bits(),
        transform.m21.to_bits(),
        transform.m22.to_bits(),
        quarter(transform.dx),
        quarter(transform.dy),
        width as u32,
        height as u32,
    ]
}
