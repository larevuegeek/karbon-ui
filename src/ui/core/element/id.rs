/// Stable identifier of an [`Element`](super::Element).
///
/// Identifiers built with [`named`](Self::named), [`child`](Self::child) or
/// [`item`](Self::item) always have their high bit set, so they never collide with small
/// raw numeric identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub u64);

impl ElementId {
    /// Stable key computed without allocation and independent of the order of children.
    pub const fn named(name: &str) -> Self {
        Self::hash(0xcbf2_9ce4_8422_2325, name.as_bytes())
    }
    /// Scope keys of a component that is reused several times on the same page.
    pub const fn child(self, name: &str) -> Self {
        Self::hash(self.0.wrapping_mul(0x100_0000_01b3) ^ 0xff, name.as_bytes())
    }
    /// Numeric business key of a collection item, without string formatting.
    pub const fn item(self, key: u64) -> Self {
        Self::hash(
            self.0.wrapping_mul(0x100_0000_01b3) ^ 0xfe,
            &key.to_le_bytes(),
        )
    }
    const fn hash(mut hash: u64, bytes: &[u8]) -> Self {
        let mut i = 0;
        while i < bytes.len() {
            hash = (hash ^ bytes[i] as u64).wrapping_mul(0x100_0000_01b3);
            i += 1;
        }
        Self(hash | (1 << 63))
    }
}
impl From<u64> for ElementId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl From<&str> for ElementId {
    fn from(value: &str) -> Self {
        Self::named(value)
    }
}
