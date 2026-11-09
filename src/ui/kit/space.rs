//! Distances des attributs `gap` et `padding` : nom d'échelle ou nombre de pixels.
use crate::ui::template::IntoProp;

/// Distance of a `gap` or `padding` attribute: `gap="md"` (`xs`, `sm`, `md`, `lg`, `xl`,
/// `2xl`), `gap="20"` or `gap={20}`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Space(Option<f32>);

impl Space {
    pub(super) fn or(self, default: f32) -> f32 {
        self.0.unwrap_or(default)
    }
}
impl IntoProp<Space> for &str {
    fn into_prop(self) -> Space {
        Space(match self {
            "none" => Some(0.),
            "xs" => Some(4.),
            "sm" => Some(8.),
            "md" => Some(12.),
            "lg" => Some(16.),
            "xl" => Some(24.),
            "2xl" => Some(32.),
            other => other.parse().ok(),
        })
    }
}
impl IntoProp<Space> for i32 {
    fn into_prop(self) -> Space {
        Space(Some(self as f32))
    }
}
impl IntoProp<Space> for f32 {
    fn into_prop(self) -> Space {
        Space(Some(self))
    }
}
