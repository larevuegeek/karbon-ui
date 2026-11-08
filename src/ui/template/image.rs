use super::ImageSlot;
use crate::ui::{
    element::*,
    scene::{DrawCommand, Image, Scene},
};
use std::sync::Arc;

/// Taille naturelle d'une image (nulle tant qu'elle n'est pas arrivée).
pub(crate) fn intrinsic(image: Option<&Image>) -> SizeF {
    image.map_or(SizeF::default(), |image| SizeF {
        width: image.width as f32,
        height: image.height as f32,
    })
}
/// Widget drawing a decoded image, sized to its intrinsic pixel size.
///
/// Fit, position and corner radii come from the computed style
/// ([`ImageFit`](crate::ui::styling::ImageFit), `image_position`, `radius`). The source
/// is an [`ImageSlot`]: an image that arrives later is painted without rebuilding the
/// view; until then the widget paints nothing and measures zero.
pub struct ImageWidget {
    image: ImageSlot,
    alt: Arc<str>,
    fit: crate::ui::styling::ImageFit,
    position: [f32; 2],
    radii: Option<[f32; 4]>,
}
impl ImageWidget {
    /// Creates an image widget, filled and centered by default, from a decoded image
    /// (`Arc<Image>`) or an [`ImageSlot`] filled later.
    pub fn new(image: impl Into<ImageSlot>) -> Self {
        Self {
            image: image.into(),
            alt: Arc::from(""),
            fit: Default::default(),
            position: [0.5, 0.5],
            radii: None,
        }
    }
}
impl ImageWidget {
    /// Sets the accessible name (`alt` text).
    pub fn alt(mut self, alt: impl Into<Arc<str>>) -> Self {
        self.alt = alt.into();
        self
    }
}
impl Widget for ImageWidget {
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Image,
            name: &self.alt,
            enabled: true,
        }
    }
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        self.fit = style.local.image_fit.unwrap_or_default();
        self.position = style.local.image_position.unwrap_or([0.5, 0.5]);
        self.radii = style
            .local
            .corner_radii
            .or_else(|| style.local.radius.map(|r| [r; 4]));
        false
    }
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(intrinsic(self.image.image().as_deref()))
    }
    fn deferred_image(&self) -> Option<&ImageSlot> {
        Some(&self.image)
    }
    fn paint(&self, c: &PaintContext, s: &mut Scene) {
        use crate::ui::{layout, styling::ImageFit};
        let Some(image) = self.image.image() else {
            return;
        };
        let w = layout::width(c.rect);
        let h = layout::height(c.rect);
        let iw = image.width as f32;
        let ih = image.height as f32;
        let (dw, dh) = match self.fit {
            ImageFit::Fill => (w, h),
            ImageFit::None => (iw, ih),
            fit => {
                let factor = if fit == ImageFit::Cover {
                    (w / iw).max(h / ih)
                } else {
                    (w / iw).min(h / ih)
                };
                let factor = if fit == ImageFit::ScaleDown {
                    factor.min(1.)
                } else {
                    factor
                };
                (iw * factor, ih * factor)
            }
        };
        let rect = crate::ui::scene::RectF::new(
            c.rect.left + (w - dw) * self.position[0],
            c.rect.top + (h - dh) * self.position[1],
            dw,
            dh,
        );
        s.commands.push(DrawCommand::PushClip(c.rect));
        if let Some(radii) = self.radii {
            s.commands.push(DrawCommand::PushShapeClip {
                rect: c.rect,
                radii,
            });
        }
        s.commands.push(DrawCommand::Image { rect, image });
        if self.radii.is_some() {
            s.commands.push(DrawCommand::PopShapeClip);
        }
        s.commands.push(DrawCommand::PopClip);
    }
}
