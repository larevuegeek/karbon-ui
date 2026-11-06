//! Component shadows are rasterized only when their shape/style changes.
//! Three separable box passes approximate the Gaussian; painting reuses the image.
//! Shadows without blur (rings such as `0 0 0 1px`, one-side borders such as
//! `inset 0 -1px 0`) are drawn as strokes and fills instead: an image of the whole
//! element for a one-pixel line cost megabytes of pixels and GPU memory.
use crate::ui::{
    element::Element,
    layout,
    scene::{Image, RectF},
    theme::Color,
};
use std::sync::Arc;
mod raster;
#[cfg(test)]
mod tests;
mod vector;
use raster::generate;
use vector::vector;
/// One shadow layer, like a CSS `box-shadow` entry. Lengths are in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    /// Horizontal offset.
    pub x: f32,
    /// Vertical offset.
    pub y: f32,
    /// Blur radius (non-negative).
    pub blur: f32,
    /// Growth (positive) or shrink (negative) of the shadow shape.
    pub spread: f32,
    /// Shadow colour.
    pub color: Color,
    /// Draw inside the element's border box instead of outside.
    pub inset: bool,
}
/// List of [`BoxShadow`] layers applied to an element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadows {
    /// Shadow layers (at most 16).
    pub values: &'static [BoxShadow],
    /// Multiplier for all lengths (layout scale); `1.0` by default.
    pub scale: f32,
}
impl Shadows {
    /// Shadow list at scale `1.0`.
    pub const fn new(values: &'static [BoxShadow]) -> Self {
        Self { values, scale: 1. }
    }
    /// `true` if the list can be rendered: at most 16 layers with finite values and blur
    /// radii no larger than 512 px once scaled.
    pub fn valid(self) -> bool {
        self.scale.is_finite()
            && self.scale > 0.
            && self.values.len() <= 16
            && self.values.iter().all(|s| {
                [s.x, s.y, s.blur, s.spread].iter().all(|n| n.is_finite())
                    && s.blur >= 0.
                    && s.blur * self.scale <= 512.
            })
    }
}
/// What a shadow layer paints, relative to the element's top-left corner.
enum Paint {
    Image(Arc<Image>),
    /// Rectangle filled with the shadow colour.
    Fill(Color),
    /// Rounded band drawn as a stroke of `width` centred on the frame's outline.
    Ring {
        radii: [f32; 4],
        width: f32,
        color: Color,
    },
}
struct Frame {
    /// Index de la couche `box-shadow` qui a produit ce cadre.
    layer: usize,
    paint: Paint,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    inset: bool,
}
impl Frame {
    /// Agrandissement appliqué au dessin : seules les images (étirées pendant un
    /// redimensionnement) suivent l'écart avec leur taille de calcul.
    fn grow(&self, w: f32, h: f32) -> (f32, f32) {
        if matches!(self.paint, Paint::Image(_)) {
            (w, h)
        } else {
            (0., 0.)
        }
    }
}
#[derive(PartialEq)]
struct Key {
    width: u32,
    height: u32,
    radii: [u32; 4],
    shadows: Shadows,
}
#[derive(Default)]
pub(crate) struct Cache {
    key: Option<Key>,
    frames: Vec<Frame>,
    /// Taille de l'élément pour laquelle les images ont été calculées ; pendant un
    /// redimensionnement de fenêtre, elles sont étirées à la taille courante.
    drawn: (f32, f32),
}
impl Cache {
    pub fn bounds(&self, rect: RectF, clip: RectF) -> RectF {
        let mut result = clip;
        let grow_w = layout::width(rect) - self.drawn.0;
        let grow_h = layout::height(rect) - self.drawn.1;
        for f in self.frames.iter().filter(|f| !f.inset) {
            result.left = result.left.min(rect.left + f.x);
            result.top = result.top.min(rect.top + f.y);
            let (gw, gh) = f.grow(grow_w, grow_h);
            result.right = result.right.max(rect.left + f.x + f.width + gw);
            result.bottom = result.bottom.max(rect.top + f.y + f.height + gh);
        }
        result
    }
    fn prepare(
        &mut self,
        rect: RectF,
        radii: [f32; 4],
        shadows: Shadows,
        stretch: bool,
    ) -> crate::Result<()> {
        let w = layout::width(rect);
        let h = layout::height(rect);
        let key = Key {
            width: w.to_bits(),
            height: h.to_bits(),
            radii: radii.map(f32::to_bits),
            shadows,
        };
        if self.key.as_ref() == Some(&key) {
            return Ok(());
        }
        // Redimensionnement de fenêtre en cours et seule la taille a changé : les couches
        // floutées (images) sont étirées, le flou reste plausible, au lieu d'être
        // recalculées à chaque pas ; les anneaux et bandes, peu coûteux, sont refaits à la
        // bonne taille. La clé garde l'ancienne taille : l'image exacte revient ensuite.
        if stretch
            && self
                .key
                .as_ref()
                .is_some_and(|k| k.radii == key.radii && k.shadows == key.shadows)
            && self
                .frames
                .iter()
                .any(|f| matches!(f.paint, Paint::Image(_)))
        {
            let mut images = std::mem::take(&mut self.frames);
            images.retain(|f| matches!(f.paint, Paint::Image(_)));
            for (layer, shadow) in shadows.values.iter().enumerate() {
                let scaled = BoxShadow {
                    x: shadow.x * shadows.scale,
                    y: shadow.y * shadows.scale,
                    blur: shadow.blur * shadows.scale,
                    spread: shadow.spread * shadows.scale,
                    ..*shadow
                };
                match vector(w, h, radii, scaled) {
                    Some(frames) => self
                        .frames
                        .extend(frames.into_iter().map(|f| Frame { layer, ..f })),
                    None => self
                        .frames
                        .extend(images.extract_if(.., |f| f.layer == layer)),
                }
            }
            return Ok(());
        }
        self.key = None;
        self.frames.clear();
        let mut bytes = 0usize;
        if w > 0. && h > 0. {
            for (layer, shadow) in shadows.values.iter().enumerate() {
                let scaled = BoxShadow {
                    x: shadow.x * shadows.scale,
                    y: shadow.y * shadows.scale,
                    blur: shadow.blur * shadows.scale,
                    spread: shadow.spread * shadows.scale,
                    ..*shadow
                };
                if let Some(frames) = vector(w, h, radii, scaled) {
                    self.frames
                        .extend(frames.into_iter().map(|f| Frame { layer, ..f }));
                    continue;
                }
                let frame = Frame {
                    layer,
                    ..generate(w, h, radii, *shadow, shadows.scale)?
                };
                if let Paint::Image(image) = &frame.paint {
                    bytes += image.pixels.len();
                }
                if bytes > 16 * 1024 * 1024 {
                    return Err(crate::Error::invalid_input(
                        "component shadows exceed their combined 16 MiB limit",
                    ));
                }
                self.frames.push(frame);
            }
        }
        self.key = Some(key);
        self.drawn = (w, h);
        Ok(())
    }
    pub fn paint(&self, rect: RectF, inset: bool, scene: &mut crate::ui::scene::Scene) {
        let grow_w = layout::width(rect) - self.drawn.0;
        let grow_h = layout::height(rect) - self.drawn.1;
        for f in self.frames.iter().rev().filter(|f| f.inset == inset) {
            let (gw, gh) = f.grow(grow_w, grow_h);
            let at = RectF::new(
                rect.left + f.x,
                rect.top + f.y,
                (f.width + gw).max(0.),
                (f.height + gh).max(0.),
            );
            scene.commands.push(match &f.paint {
                Paint::Image(image) => crate::ui::scene::DrawCommand::Image {
                    rect: at,
                    image: Arc::clone(image),
                },
                Paint::Fill(color) => crate::ui::scene::DrawCommand::Fill {
                    rect: at,
                    radius: 0.,
                    color: *color,
                },
                Paint::Ring {
                    radii,
                    width,
                    color,
                } => crate::ui::scene::DrawCommand::Shape {
                    rect: at,
                    radii: *radii,
                    fill: None,
                    stroke: Some((*width, *color)),
                },
            });
        }
    }
}
/// `stretch` : redimensionnement de fenêtre en cours, les ombres floutées d'un élément qui
/// change seulement de taille sont étirées au lieu d'être recalculées.
pub(crate) fn prepare(node: &mut Element, stretch: bool) -> crate::Result<()> {
    if let Some(styles) = &mut node.styles {
        if let Some(shadows) = styles.computed.local.shadows.filter(|_| {
            layout::width(node.clip) > 0.
                && layout::height(node.clip) > 0.
                && layout::width(layout::intersect(node.rect, node.clip)) > 0.
                && layout::height(layout::intersect(node.rect, node.clip)) > 0.
        }) {
            let radii = styles
                .computed
                .local
                .corner_radii
                .unwrap_or([node.style.radius; 4]);
            styles
                .shadow_cache
                .prepare(node.rect, radii, shadows, stretch)?;
        } else {
            styles.shadow_cache = Default::default();
        }
    }
    for child in &mut node.children {
        prepare(child, stretch)?;
    }
    Ok(())
}
fn normalized(w: f32, h: f32, mut r: [f32; 4]) -> [f32; 4] {
    let mut f = 1f32;
    for (n, sum) in [
        (w, r[0] + r[1]),
        (w, r[3] + r[2]),
        (h, r[0] + r[3]),
        (h, r[1] + r[2]),
    ] {
        if sum > 0. {
            f = f.min(n / sum);
        }
    }
    for n in &mut r {
        *n *= f;
    }
    r
}
