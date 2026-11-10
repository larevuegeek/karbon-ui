pub(super) use super::super::*;
pub(super) use crate::ui::{element::Axis, slider::RangeInfo};
pub(super) use windows::core::BOOL;

/// Valeur scalaire d'une propriété RangeValue, comparable avant conversion en VARIANT.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Scalar {
    Integer(i32),
    Number(f64),
    Boolean(bool),
    Empty,
}
impl Scalar {
    pub(crate) fn variant(self) -> Variant {
        match self {
            Self::Integer(value) => Variant::integer(value),
            Self::Number(value) => Variant::number(value),
            Self::Boolean(value) => Variant::boolean(value),
            Self::Empty => Variant::default(),
        }
    }
}
pub(crate) fn property(range: RangeInfo, id: i32) -> Scalar {
    match id {
        30023 => Scalar::Integer(if range.axis == Axis::Horizontal { 1 } else { 2 }),
        30047 => Scalar::Number(range.value),
        30048 => Scalar::Boolean(range.read_only),
        30049 => Scalar::Number(range.min),
        30050 => Scalar::Number(range.max),
        30051 => Scalar::Number(range.large_change),
        30052 => Scalar::Number(range.small_change),
        _ => Scalar::Empty,
    }
}
/// IRangeValueProvider.
impl provider::Core {
    pub(crate) fn set_number(&self, value: f64) -> Result<()> {
        check(self.bridge.dispatch(Command::SetNumber(self.key, value)))
    }
    pub(crate) fn number(&self, id: i32) -> Result<f64> {
        let range = self.node()?.range.ok_or_else(|| fail(UNSUPPORTED))?;
        match property(range, id) {
            Scalar::Number(value) => Ok(value),
            _ => Err(fail(INVALID_ARGUMENT)),
        }
    }
    pub(crate) fn range_read_only(&self) -> Result<BOOL> {
        let range = self.node()?.range.ok_or_else(|| fail(UNSUPPORTED))?;
        Ok(range.read_only.into())
    }
}
pub(crate) fn raise_events(
    provider: &IRawElementProviderSimple,
    before: Option<RangeInfo>,
    after: Option<RangeInfo>,
) {
    if before.is_some() != after.is_some() {
        raise_property(
            provider,
            30033,
            Ok(Variant::boolean(before.is_some())),
            Ok(Variant::boolean(after.is_some())),
        );
    }
    if let (Some(before), Some(after)) = (before, after) {
        for id in [30023, 30047, 30048, 30049, 30050, 30051, 30052] {
            let a = property(before, id);
            let b = property(after, id);
            if a != b {
                raise_property(provider, id, Ok(a.variant()), Ok(b.variant()));
            }
        }
    }
}
