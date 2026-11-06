// Géométrie commune ; repr(C) permet aussi les échanges avec les API natives.
/// Integer point, in pixels.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
}
/// Integer rectangle defined by its edges, in pixels.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}
/// Integer size, in pixels.
#[repr(C)]
pub struct Size {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

/// Floating-point point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointF {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

/// Floating-point rectangle defined by its edges.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectF {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}
impl RectF {
    /// Rectangle from its top-left corner and its size.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }
}

/// Floating-point size.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SizeF {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}
