//! Une toile d'éléments posés dans le monde (KUI-036) : des rectangles (nœuds, clés,
//! blocs d'une frise) placés en coordonnées du monde, dessinés à travers une
//! [`CanvasView`] et découpés au bord de la toile. Seuls ceux qui touchent la toile
//! produisent des commandes ; la peinture est gardée d'une image à l'autre quand la vue
//! et la révision des éléments ne changent pas (KUI-056).
use super::canvas::CanvasView;
use crate::ui::{
    element::{PaintContext, SizeF, TextMeasurer, Widget},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::{rc::Rc, sync::Arc};

/// A rectangle placed in the world of a [`CanvasLayer`].
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasItem {
    /// Its place in world units: `left` < `right`, `bottom` < `top` (the world's `y`
    /// grows upwards, as in [`CanvasView`]).
    pub rect: RectF,
    /// Fill colour.
    pub fill: Color,
    /// Corner radius, in screen pixels.
    pub radius: f32,
    /// Outline width (screen pixels) and colour, if any.
    pub stroke: Option<(f32, Color)>,
    /// Text written in the top left corner, if any.
    pub label: Option<Arc<str>>,
}

impl CanvasItem {
    /// A filled rectangle at `rect` (world units, see [`rect`](Self::rect)).
    pub fn new(rect: RectF, fill: Color) -> Self {
        Self {
            rect,
            fill,
            radius: 0.,
            stroke: None,
            label: None,
        }
    }
}

/// A 2D canvas of [`CanvasItem`]s seen through a [`CanvasView`]: a widget to put in an
/// element (`Element::custom`) that fills its box. Items outside the box produce no
/// command, and what crosses the edge is clipped, so 10,000 items zoom and pan at the
/// pace of the screen. Give a [`revision`](Self::revision) that changes with the items:
/// while it and the view stay the same, the runtime reuses the commands of the previous
/// frame (see [`Widget::paint_key`]).
///
/// ```
/// use karbon_ui::ui::{
///     editor::canvas::{CanvasItem, CanvasLayer, CanvasView},
///     element::Element,
///     scene::RectF,
///     theme::Color,
/// };
/// use std::rc::Rc;
///
/// let items = Rc::new(vec![CanvasItem::new(RectF::new(0., 0., 2., 1.), Color::hex(0x8B9CFF))]);
/// let layer = CanvasLayer::new(CanvasView::default(), items).revision(1);
/// let canvas = Element::custom(7, layer);
/// # let _ = canvas;
/// ```
pub struct CanvasLayer {
    view: CanvasView,
    items: Rc<Vec<CanvasItem>>,
    background: Option<Color>,
    text: Color,
    font_size: f32,
    revision: Option<u64>,
}

impl CanvasLayer {
    /// The `items` seen through `view`.
    pub fn new(view: CanvasView, items: Rc<Vec<CanvasItem>>) -> Self {
        Self {
            view,
            items,
            background: None,
            text: Color::hex(0xE6E8EE),
            font_size: 12.,
            revision: None,
        }
    }

    /// Fill the canvas with `color` under the items.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Colour and size of the labels (light grey, 12 by default).
    pub fn labels(mut self, color: Color, size: f32) -> Self {
        self.text = color;
        self.font_size = size;
        self
    }

    /// A number that changes whenever the items change: the commands are then kept
    /// while it and the view stay the same. Without it the canvas paints every frame.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Les éléments qui touchent la toile de `size` pixels, et leur place à l'écran (à
    /// partir du coin de la toile).
    pub fn visible(&self, size: (f32, f32)) -> impl Iterator<Item = (&CanvasItem, RectF)> {
        let view = self.view;
        self.items.iter().filter_map(move |item| {
            let a = view.to_screen(PointF {
                x: item.rect.left,
                y: item.rect.top,
            });
            let b = view.to_screen(PointF {
                x: item.rect.right,
                y: item.rect.bottom,
            });
            let screen = RectF {
                left: a.x.min(b.x),
                top: a.y.min(b.y),
                right: a.x.max(b.x),
                bottom: a.y.max(b.y),
            };
            let inside = screen.right >= 0.
                && screen.bottom >= 0.
                && screen.left <= size.0
                && screen.top <= size.1;
            inside.then_some((item, screen))
        })
    }
}

impl Widget for CanvasLayer {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        if let Some(color) = self.background {
            scene.fill(r, 0., color);
        }
        scene.commands.push(DrawCommand::PushClip(r));
        let size = (r.right - r.left, r.bottom - r.top);
        for (item, screen) in self.visible(size) {
            let at = RectF {
                left: screen.left + r.left,
                right: screen.right + r.left,
                top: screen.top + r.top,
                bottom: screen.bottom + r.top,
            };
            scene.fill(at, item.radius, item.fill);
            if let Some((width, color)) = item.stroke {
                scene.commands.push(DrawCommand::Stroke {
                    rect: at,
                    radius: item.radius,
                    width,
                    color,
                });
            }
            if let Some(label) = &item.label {
                let line = RectF {
                    left: at.left + 4.,
                    top: at.top + 2.,
                    right: (at.right - 4.).max(at.left + 4.),
                    bottom: at.top + 2. + self.font_size * 1.4,
                };
                scene.text_single_line(line, Arc::clone(label), self.font_size, false, self.text);
            }
        }
        scene.commands.push(DrawCommand::PopClip);
    }

    fn paint_key(&self) -> Option<u64> {
        let revision = self.revision?;
        let v = self.view;
        let rgba = |c: Color| u32::from_be_bytes([c.red, c.green, c.blue, c.alpha]);
        let background = self.background.map_or(0, |c| u64::from(rgba(c)) | 1 << 32);
        let bits = [v.origin.x, v.origin.y, v.zoom.0, v.zoom.1, self.font_size].map(f32::to_bits);
        let mut key = revision ^ 0x9E37_79B9_7F4A_7C15 ^ background.rotate_left(7);
        for b in bits.into_iter().chain([rgba(self.text)]) {
            key = (key.rotate_left(13) ^ u64::from(b)).wrapping_mul(0x100_0000_01B3);
        }
        Some(key)
    }
}
