pub(super) use super::paint::{average, blend, premultiplied};
pub(super) use super::shape::{Rounded, Shape, intersect};
pub(super) use crate::ui::{
    font::Font,
    rendering::{brush::Background, transform::Affine},
    scene::{DrawCommand, PointF, RectF, Scene},
    styling::text::TextFormat,
    theme::Color,
};

/// Clip actif : forme locale et inverse de la transformation au moment du `Push`.
pub(super) struct Clip {
    pub(super) shape: Rounded,
    pub(super) inverse: Affine,
    pub(super) device: RectF,
}

pub(super) struct Canvas<'a> {
    pub(super) pixels: &'a mut [u8],
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale: f32,
    /// Transformation locale courante (logique), opacité de groupe, et pile des effets.
    pub(super) transform: Affine,
    pub(super) opacity: f32,
    pub(super) effects: Vec<(Affine, f32)>,
    pub(super) clips: Vec<Clip>,
    /// Pile des `Push*` pour fermer le bon type de portée.
    pub(super) scopes: Vec<Scope>,
    /// Couleurs imposées au texte (`background-clip: text`).
    pub(super) text_fills: Vec<Color>,
}
#[derive(PartialEq)]
pub(super) enum Scope {
    Clip,
    Effect,
    Filter,
    TextFill,
}

impl Canvas<'_> {
    /// Transformation complète du repère local vers les pixels.
    pub(super) fn device(&self) -> Affine {
        self.transform.then(Affine {
            m11: self.scale,
            m22: self.scale,
            ..Affine::IDENTITY
        })
    }
    pub(super) fn visible(&self) -> RectF {
        self.clips.iter().fold(
            RectF::new(0., 0., self.width as f32, self.height as f32),
            |area, clip| intersect(area, clip.device),
        )
    }
    /// Parcourir les pixels couverts par `local` (repère local) et non masqués par un
    /// clip ; `paint` reçoit le point local et renvoie la couleur source, s'il y en a une.
    pub(super) fn cover(&mut self, local: RectF, mut paint: impl FnMut(PointF) -> Option<[u8; 4]>) {
        let device = self.device();
        let Some(inverse) = device.inverse() else {
            return;
        };
        let area = intersect(device.bounds(local), self.visible());
        if !(area.left < area.right && area.top < area.bottom) {
            return;
        }
        let (x0, x1) = (area.left.floor().max(0.) as u32, area.right.ceil() as u32);
        let (y0, y1) = (area.top.floor().max(0.) as u32, area.bottom.ceil() as u32);
        for y in y0..y1.min(self.height) {
            for x in x0..x1.min(self.width) {
                let center = PointF {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                };
                if !self
                    .clips
                    .iter()
                    .all(|clip| clip.shape.contains(clip.inverse.point(center)))
                {
                    continue;
                }
                if let Some(source) = paint(inverse.point(center)) {
                    let offset = (y as usize * self.width as usize + x as usize) * 4;
                    blend(&mut self.pixels[offset..offset + 4], source);
                }
            }
        }
    }
    pub(super) fn push_clip(&mut self, shape: Rounded) {
        let device = self.device();
        let Some(inverse) = device.inverse() else {
            // Transformation dégénérée : rien n'est visible dans cette portée.
            self.clips.push(Clip {
                shape,
                inverse: Affine::IDENTITY,
                device: RectF::new(0., 0., 0., 0.),
            });
            self.scopes.push(Scope::Clip);
            return;
        };
        self.clips.push(Clip {
            shape,
            inverse,
            device: device.bounds(shape.rect),
        });
        self.scopes.push(Scope::Clip);
    }
    pub(super) fn pop(&mut self, scope: Scope) {
        if self.scopes.last() != Some(&scope) {
            return;
        }
        self.scopes.pop();
        match scope {
            Scope::Clip => {
                self.clips.pop();
            }
            Scope::Effect => {
                if let Some((transform, opacity)) = self.effects.pop() {
                    self.transform = transform;
                    self.opacity = opacity;
                }
            }
            Scope::Filter => {}
            Scope::TextFill => {
                self.text_fills.pop();
            }
        }
    }
}

/// Rastériser `scene` en pixels BGRA prémultipliés de `width` × `height`, à `scale`.
pub(crate) fn rasterize(scene: &Scene, width: u32, height: u32, scale: f32) -> Vec<u8> {
    let mut pixels = premultiplied(scene.clear, 1.).repeat(width as usize * height as usize);
    let mut canvas = Canvas {
        pixels: &mut pixels,
        width,
        height,
        scale,
        transform: Affine::IDENTITY,
        opacity: 1.,
        effects: Vec::new(),
        clips: Vec::new(),
        scopes: Vec::new(),
        text_fills: Vec::new(),
    };
    for command in &scene.commands {
        match command {
            DrawCommand::Fill {
                rect,
                radius,
                color,
            } => canvas.fill(Shape::Rounded(Rounded::new(*rect, [*radius; 4])), *color),
            DrawCommand::Stroke {
                rect,
                radius,
                width,
                color,
            } => canvas.stroke(*rect, [*radius; 4], *width, *color),
            DrawCommand::Shape {
                rect,
                radii,
                fill,
                stroke,
            } => {
                if let Some(fill) = fill {
                    canvas.background(*rect, *radii, fill);
                }
                if let Some((width, color)) = stroke {
                    canvas.stroke(*rect, *radii, *width, *color);
                }
            }
            DrawCommand::Gradient {
                rect,
                radius,
                gradient,
            } => canvas.fill(
                Shape::Rounded(Rounded::new(*rect, [*radius; 4])),
                average(&gradient.stops),
            ),
            DrawCommand::RadialGradient {
                rect,
                radius,
                gradient,
            } => canvas.fill(
                Shape::Rounded(Rounded::new(*rect, [*radius; 4])),
                average(&gradient.stops),
            ),
            DrawCommand::Line {
                start,
                end,
                width,
                color,
            } => canvas.fill(
                Shape::Segment {
                    start: *start,
                    end: *end,
                    half: width / 2.,
                },
                *color,
            ),
            DrawCommand::Path {
                points,
                closed,
                color,
            } => {
                if points.len() < 2 {
                    continue;
                }
                if *closed {
                    canvas.fill(Shape::Polygon(points), *color);
                } else {
                    for pair in points.windows(2) {
                        canvas.fill(
                            Shape::Segment {
                                start: pair[0],
                                end: pair[1],
                                half: 1.,
                            },
                            *color,
                        );
                    }
                }
            }
            DrawCommand::Text {
                rect,
                text,
                size,
                color,
                wrap,
                ..
            }
            | DrawCommand::FontText {
                rect,
                text,
                size,
                color,
                wrap,
                ..
            } => {
                let format = TextFormat::new(Default::default(), Font::system(false), *size);
                canvas.text(*rect, text, &format, *color, *wrap)
            }
            DrawCommand::FormattedText {
                rect,
                text,
                format,
                color,
                wrap,
            } => canvas.text(*rect, text, format, *color, *wrap),
            DrawCommand::RichText {
                rect,
                paragraph,
                color,
                wrap,
            } => canvas.rich_text(*rect, paragraph, *color, *wrap),
            DrawCommand::Image { rect, image } => canvas.image(*rect, image),
            DrawCommand::Vector { .. } => {}
            DrawCommand::Svg { rect, image, color } => canvas.svg(*rect, image, *color),
            DrawCommand::PushClip(rect) => canvas.push_clip(Rounded::new(*rect, [0.; 4])),
            DrawCommand::PushShapeClip { rect, radii } => {
                canvas.push_clip(Rounded::new(*rect, *radii))
            }
            DrawCommand::PopClip | DrawCommand::PopShapeClip => canvas.pop(Scope::Clip),
            DrawCommand::PushEffect {
                transform, opacity, ..
            } => {
                canvas.effects.push((canvas.transform, canvas.opacity));
                canvas.transform = transform.then(canvas.transform);
                canvas.opacity *= opacity.clamp(0., 1.);
                canvas.scopes.push(Scope::Effect);
            }
            DrawCommand::PopEffect => canvas.pop(Scope::Effect),
            DrawCommand::PushFilter { .. } | DrawCommand::PushMask { .. } => {
                canvas.scopes.push(Scope::Filter)
            }
            DrawCommand::PopFilter | DrawCommand::PopMask => canvas.pop(Scope::Filter),
            DrawCommand::PushTextFill { fill, .. } => {
                // Remplissage du texte : couleur moyenne, comme les dégradés.
                canvas.text_fills.push(match fill {
                    Background::Solid(c) => *c,
                    Background::Linear(g) => average(&g.stops),
                    Background::Radial(g) => average(&g.stops),
                });
                canvas.scopes.push(Scope::TextFill);
            }
            DrawCommand::PopTextFill => canvas.pop(Scope::TextFill),
            DrawCommand::Backdrop {
                rect,
                radii,
                filters,
            } => canvas.backdrop(Rounded::new(*rect, *radii), filters),
        }
    }
    pixels
}
