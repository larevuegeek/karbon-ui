use super::*;
/// Presentation recipe of a container (card, panel), applied with [`Element::styled`](crate::ui::element::Element::styled).
#[derive(Clone, Copy)]
pub struct SurfaceStyle {
    /// Background colour; `None` leaves the surface transparent.
    pub background: Option<StyleColor>,
    /// Border colour; `None` draws no border.
    pub border: Option<StyleColor>,
    /// Border width, in pixels.
    pub border_width: f32,
    /// Inner spacing.
    pub padding: Insets,
    /// Spacing between children, in pixels.
    pub gap: f32,
    /// Corner radius, in pixels.
    pub radius: f32,
}
impl Default for SurfaceStyle {
    fn default() -> Self {
        Self {
            background: Some(ThemeColor::Surface.into()),
            border: Some(ThemeColor::Border.into()),
            border_width: 1.,
            padding: Insets::all(16.),
            gap: 12.,
            radius: 14.,
        }
    }
}
impl SurfaceStyle {
    pub(crate) fn validate(&self) -> crate::Result<()> {
        dimensions(&[
            self.border_width,
            self.padding.left,
            self.padding.right,
            self.padding.top,
            self.padding.bottom,
            self.gap,
            self.radius,
        ])
    }
}
