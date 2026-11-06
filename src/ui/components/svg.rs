//! Widget painting a compiled SVG drawing (`<svg>` in templates).
use crate::ui::{
    element::*,
    rendering::svg::SvgImage,
    scene::{DrawCommand, Scene},
    theme::Color,
};
use std::sync::Arc;

/// Leaf widget drawing a static [`SvgImage`] scaled to its content box.
///
/// `currentColor` takes the computed CSS `color` of the element (inherited from its
/// ancestors), or the theme text colour when no colour is declared, so hover and active
/// states recolour the drawing. Its natural size is the `width`/`height` given to
/// [`Svg::new`].
pub struct Svg {
    image: &'static SvgImage,
    size: SizeF,
    label: Arc<str>,
    color: Option<Color>,
}
impl Svg {
    /// Drawing with a natural size of `width` × `height` logical pixels.
    pub fn new(image: &'static SvgImage, width: f32, height: f32) -> Self {
        Self {
            image,
            size: SizeF { width, height },
            label: Arc::from(""),
            color: None,
        }
    }
    /// Accessible name (`aria-label` or `<title>`); the drawing is exposed as an image
    /// when it has one, and stays decorative otherwise.
    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = label.into();
        self
    }
    /// Fixed `currentColor`, overriding the computed CSS colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// The compiled drawing.
    pub fn image(&self) -> &'static SvgImage {
        self.image
    }
}
impl Widget for Svg {
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        self.color = style.text.color;
        false
    }
    fn validate(&self) -> crate::Result<()> {
        if !(self.size.width.is_finite()
            && self.size.height.is_finite()
            && self.size.width >= 0.
            && self.size.height >= 0.)
        {
            return Err(crate::Error::invalid_input("invalid SVG size"));
        }
        self.image.validate()
    }
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(self.size)
    }
    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        if context.rect.right <= context.rect.left || context.rect.bottom <= context.rect.top {
            return;
        }
        scene.commands.push(DrawCommand::Svg {
            rect: context.rect,
            image: self.image,
            color: self.color.unwrap_or(context.theme.text),
        });
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: if self.label.is_empty() {
                Role::Custom
            } else {
                Role::Image
            },
            name: &self.label,
            enabled: true,
        }
    }
}
