use super::*;
#[derive(Clone, Default)]
/// Appearance overrides for a [`Slider`](super::Slider); `None` keeps the default.
pub struct SliderStyle {
    /// Track thickness in logical units (4 by default); must be in `(0, 128]`.
    pub track_thickness: Option<f32>,
    /// Unfilled track colour (theme border by default).
    pub track: Option<Color>,
    /// Filled part of the track (theme accent by default).
    pub fill: Option<Color>,
    /// Thumb colour.
    pub thumb: Option<Color>,
    /// Focus ring colour (theme accent by default).
    pub focus: Option<Color>,
}
