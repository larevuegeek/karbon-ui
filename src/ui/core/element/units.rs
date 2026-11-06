use super::{Insets, Length};

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}
impl From<i32> for Length {
    fn from(value: i32) -> Self {
        Self::Px(value as f32)
    }
}
impl From<u32> for Length {
    fn from(value: u32) -> Self {
        Self::Px(value as f32)
    }
}
impl Insets {
    /// Sum of the left and right insets.
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }
    /// Sum of the top and bottom insets.
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
    /// Same order as CSS: top, right, bottom, left.
    pub const fn edges(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
    /// Same horizontal inset on the left and right, same vertical inset on the top and bottom.
    pub const fn xy(horizontal: f32, vertical: f32) -> Self {
        Self::edges(vertical, horizontal, vertical, horizontal)
    }
}
impl From<[f32; 3]> for Insets {
    fn from([top, horizontal, bottom]: [f32; 3]) -> Self {
        Self::edges(top, horizontal, bottom, horizontal)
    }
}
impl From<[i32; 3]> for Insets {
    fn from([top, horizontal, bottom]: [i32; 3]) -> Self {
        Self::edges(
            top as f32,
            horizontal as f32,
            bottom as f32,
            horizontal as f32,
        )
    }
}
impl From<f32> for Insets {
    fn from(value: f32) -> Self {
        Self::all(value)
    }
}
impl From<i32> for Insets {
    fn from(value: i32) -> Self {
        Self::all(value as f32)
    }
}
impl From<[f32; 2]> for Insets {
    fn from([vertical, horizontal]: [f32; 2]) -> Self {
        Self::xy(horizontal, vertical)
    }
}
impl From<[f32; 4]> for Insets {
    fn from([top, right, bottom, left]: [f32; 4]) -> Self {
        Self::edges(top, right, bottom, left)
    }
}
impl From<[i32; 2]> for Insets {
    fn from([vertical, horizontal]: [i32; 2]) -> Self {
        Self::xy(horizontal as f32, vertical as f32)
    }
}
impl From<[i32; 4]> for Insets {
    fn from([top, right, bottom, left]: [i32; 4]) -> Self {
        Self::edges(top as f32, right as f32, bottom as f32, left as f32)
    }
}
