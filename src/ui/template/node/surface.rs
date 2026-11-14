//! Peinture de la boîte : fonds, bordures, rayons, ombres, contour, filtres et images.
use super::Node;
use crate::ui::{element::*, theme::Color};
impl<W: Widget + 'static> Node<W> {
    /// Sets the background (color or gradient).
    pub fn background(mut self, value: impl Into<crate::ui::rendering::brush::Background>) -> Self {
        self.element = self.element.background(value);
        self.properties().background = self.element.style.background;
        self.properties().color_expressions[1] = None;
        self
    }
    /// Sets a solid background color.
    pub fn background_color(mut self, value: Color) -> Self {
        self.element.style.background = Some(value);
        self.properties().background = Some(value);
        self.properties().color_expressions[1] = None;
        self
    }
    /// Replaces the background with stacked layers, clearing the background color.
    pub fn background_layers(
        mut self,
        value: Vec<crate::ui::rendering::brush::Background>,
    ) -> Self {
        self.element.style.background = None;
        self.properties().background = None;
        self.element = self.element.backgrounds(value);
        self
    }
    /// Sets background image layers, keeping the background color.
    pub fn image_layers(mut self, value: Vec<crate::ui::rendering::brush::Background>) -> Self {
        self.element = self.element.backgrounds(value);
        self
    }
    /// Sets a linear-gradient background image (`None` clears it) and drops
    /// background layers.
    pub fn background_image(
        mut self,
        value: Option<std::sync::Arc<crate::ui::rendering::brush::LinearGradient>>,
    ) -> Self {
        self.element.gradient = value;
        self.element.backgrounds = None;
        self
    }
    /// Masks the node and its subtree with the alpha of a linear gradient
    /// (`mask-image`); `None` removes the mask.
    pub fn mask_image(
        mut self,
        value: Option<std::sync::Arc<crate::ui::rendering::brush::LinearGradient>>,
    ) -> Self {
        self.element
            .styles
            .get_or_insert_with(Default::default)
            .mask = value;
        self
    }
    /// Sets a uniform corner radius in pixels, replacing per-corner radii.
    pub fn radius(mut self, value: f32) -> Self {
        self.properties().corner_radii = None;
        self.element = self.element.radius(value);
        self.properties().radius = Some(value);
        self
    }
    /// Sets per-corner radii (top-left, top-right, bottom-right, bottom-left).
    pub fn corner_radii(mut self, value: [f32; 4]) -> Self {
        self.properties().corner_radii = Some(value);
        self
    }
    /// Sets the radius of one corner (`0` top-left, `1` top-right, `2` bottom-right,
    /// `3` bottom-left), keeping the others.
    pub fn corner_radius(mut self, corner: usize, value: f32) -> Self {
        let mut radii = self
            .properties()
            .corner_radii
            .unwrap_or([self.element.style.radius; 4]);
        radii[corner] = value;
        self.corner_radii(radii)
    }
    /// Sets the box shadows.
    pub fn box_shadows(mut self, value: crate::ui::rendering::component_shadow::Shadows) -> Self {
        self.properties().shadows = Some(value);
        self.properties().shadow_expressions = None;
        self
    }
    /// Sets the border color, keeping the current border width.
    pub fn border_color(mut self, value: Color) -> Self {
        self.border_style().border = Some(value.into());
        let width = self.border_style().border_width;
        self.element.layout_rules().border = Insets::all(width);
        self.properties().border_width = Some(width);
        self.properties().border = Some(value);
        self.properties().color_expressions[2] = None;
        self
    }
    /// Sets the border width in pixels on every edge.
    pub fn border_width(mut self, value: f32) -> Self {
        self.border_style().border_width = value;
        self.element.layout_rules().border = Insets::all(value);
        self.properties().border_width = Some(value);
        self
    }
    pub(super) fn border_style(&mut self) -> &mut crate::ui::style::SurfaceStyle {
        let style = self.element.surface_style.get_or_insert_with(|| {
            std::sync::Arc::new(crate::ui::style::SurfaceStyle {
                background: None,
                border: None,
                padding: Insets::all(0.),
                gap: 0.,
                ..Default::default()
            })
        });
        std::sync::Arc::make_mut(style)
    }
    /// Sets the border width of one side (`0` top, `1` right, `2` bottom, `3` left).
    pub fn border_edge(mut self, side: usize, value: f32) -> Self {
        self.properties().border_edges[side] = Some(value);
        self
    }
    /// Sets the border colour of one side (`0` top, `1` right, `2` bottom, `3` left).
    pub fn border_edge_color(mut self, side: usize, value: Color) -> Self {
        self.properties().border_colors[side] = Some(value);
        self
    }
    /// Sets the outline width, drawn outside the border box without affecting layout.
    pub fn outline_width(mut self, value: f32) -> Self {
        self.properties().outline_width = Some(value);
        self
    }
    /// Sets the outline colour (the text colour by default).
    pub fn outline_color(mut self, value: Color) -> Self {
        self.properties().outline_color = Some(value);
        self
    }
    /// Sets the distance between the border box and the outline.
    pub fn outline_offset(mut self, value: f32) -> Self {
        self.properties().outline_offset = Some(value);
        self
    }
    /// Applies visual filters.
    pub fn filters(mut self, value: crate::ui::rendering::filter::Filters) -> Self {
        self.properties().filters = Some(value);
        self
    }
    /// Filters what is painted behind the node (`backdrop-filter`).
    pub fn backdrop_filter(mut self, value: crate::ui::rendering::filter::Filters) -> Self {
        self.properties().backdrop = Some(value);
        self
    }
    /// Paints the background through the text of the node only (`background-clip: text`).
    pub fn text_clip(mut self, value: bool) -> Self {
        self.properties().text_clip = Some(value);
        self
    }
    /// Sets the accent color used by controls.
    pub fn accent_color(mut self, value: Color) -> Self {
        self.properties().accent = Some(value);
        self.properties().color_expressions[3] = None;
        self
    }
    /// Sets how an image fills the node.
    pub fn image_fit(mut self, value: crate::ui::styling::ImageFit) -> Self {
        self.properties().image_fit = Some(value);
        self
    }
    /// Sets the image position as horizontal and vertical fractions of the free
    /// space (`0.5` centers).
    pub fn image_position(mut self, value: [f32; 2]) -> Self {
        self.properties().image_position = Some(value);
        self
    }
}
