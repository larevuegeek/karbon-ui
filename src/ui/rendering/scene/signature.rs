//! Signature d'une suite de commandes : identifie le contenu d'un groupe filtré pour que
//! les backends réutilisent son résultat tant qu'il ne change pas (textes comparés par
//! valeur, images et dégradés par identité de leur `Arc`).
use super::{DrawCommand, RectF};
use crate::ui::{
    font::Font,
    rendering::filter::{Filter, Filters},
    theme::Color,
};
use std::sync::Arc;
// Servie aux backends natifs (Windows, macOS, Linux) ; le backend headless ne garde pas
// de groupes filtrés.
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(dead_code)
)]
#[derive(PartialEq, Eq)]
pub(crate) struct Signature {
    data: Vec<u64>,
    texts: Vec<Arc<str>>,
}
#[cfg_attr(
    not(any(windows, target_os = "macos", target_os = "linux")),
    allow(dead_code)
)]
impl Signature {
    fn f(&mut self, n: f32) {
        self.data.push(n.to_bits() as u64);
    }
    fn rect(&mut self, r: RectF) {
        for n in [r.left, r.top, r.right, r.bottom] {
            self.f(n);
        }
    }
    fn color(&mut self, c: Color) {
        self.data
            .push(u32::from_le_bytes([c.red, c.green, c.blue, c.alpha]) as u64);
    }
    fn font(&mut self, font: Font) {
        self.data.extend([
            font.family.as_ptr() as usize as u64,
            font.family.len() as u64,
            font.weight as u64,
        ]);
    }
    fn background(&mut self, b: &crate::ui::rendering::brush::Background) {
        use crate::ui::rendering::brush::Background;
        match b {
            Background::Solid(c) => {
                self.data.push(0);
                self.color(*c);
            }
            Background::Linear(v) => {
                self.data.push(1);
                self.linear(v);
            }
            Background::Radial(v) => {
                self.data.push(2);
                self.radial(v);
            }
        }
    }
    /// Les dégradés sont comparés par valeur : une vue reconstruite les recrée dans de
    /// nouveaux `Arc` sans les changer.
    fn stops(&mut self, stops: &[crate::ui::rendering::brush::GradientStop]) {
        self.data.push(stops.len() as u64);
        for stop in stops {
            self.f(stop.position);
            self.color(stop.color);
        }
    }
    fn linear(&mut self, g: &crate::ui::rendering::brush::LinearGradient) {
        self.f(g.angle);
        self.data.push(g.corner.map_or(u64::MAX, |[x, y]| {
            u64::from(x as u8) << 8 | u64::from(y as u8)
        }));
        self.stops(&g.stops);
    }
    fn radial(&mut self, g: &crate::ui::rendering::brush::RadialGradient) {
        for v in g.center.into_iter().chain(g.radius) {
            self.f(v);
        }
        self.data.push(g.extent as u64 | u64::from(g.circle) << 8);
        self.stops(&g.stops);
    }
    fn filters(&mut self, filters: Filters) {
        self.f(filters.scale);
        self.data.push(filters.values.len() as u64);
        for value in filters.values {
            match value {
                Filter::Blur(n) => {
                    self.data.push(0);
                    self.f(*n);
                }
                Filter::Brightness(n) => {
                    self.data.push(1);
                    self.f(*n);
                }
                Filter::Contrast(n) => {
                    self.data.push(2);
                    self.f(*n);
                }
                Filter::Grayscale(n) => {
                    self.data.push(3);
                    self.f(*n);
                }
                Filter::Sepia(n) => {
                    self.data.push(4);
                    self.f(*n);
                }
                Filter::Saturate(n) => {
                    self.data.push(5);
                    self.f(*n);
                }
                Filter::HueRotate(n) => {
                    self.data.push(6);
                    self.f(*n);
                }
                Filter::Invert(n) => {
                    self.data.push(7);
                    self.f(*n);
                }
                Filter::Opacity(n) => {
                    self.data.push(8);
                    self.f(*n);
                }
                Filter::DropShadow { x, y, blur, color } => {
                    self.data.push(9);
                    self.f(*x);
                    self.f(*y);
                    self.f(*blur);
                    self.color(*color);
                }
            }
        }
    }
    /// Empreinte de la signature (données et textes), pour comparer des suites de
    /// commandes d'une image à l'autre sans les conserver.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn digest(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.data.hash(&mut hasher);
        for text in &self.texts {
            text.hash(&mut hasher);
        }
        hasher.finish()
    }
    pub fn new(commands: &[DrawCommand], bounds: RectF, filters: Filters, scale: f32) -> Self {
        let mut key = Self {
            data: Vec::new(),
            texts: Vec::new(),
        };
        key.rect(bounds);
        key.filters(filters);
        key.f(scale);
        for command in commands {
            match command {
                DrawCommand::PushShapeClip { rect, radii } => {
                    key.data.push(20);
                    key.rect(*rect);
                    for r in radii {
                        key.f(*r);
                    }
                }
                DrawCommand::PopShapeClip => key.data.push(21),
                DrawCommand::PushMask { bounds, gradient } => {
                    key.data.push(30);
                    key.linear(gradient);
                    key.rect(*bounds);
                }
                DrawCommand::PopMask => key.data.push(31),
                DrawCommand::PushTextFill { rect, fill } => {
                    use crate::ui::rendering::brush::Background;
                    key.data.push(32);
                    key.rect(*rect);
                    match fill {
                        Background::Solid(c) => key.data.extend([
                            0,
                            u64::from(u32::from_be_bytes([c.red, c.green, c.blue, c.alpha])),
                        ]),
                        Background::Linear(g) => {
                            key.data.push(1);
                            key.linear(g);
                        }
                        Background::Radial(g) => {
                            key.data.push(2);
                            key.radial(g);
                        }
                    }
                }
                DrawCommand::PopTextFill => key.data.push(33),
                DrawCommand::Backdrop {
                    rect,
                    radii,
                    filters,
                } => {
                    key.data.push(34);
                    key.rect(*rect);
                    for r in radii {
                        key.f(*r);
                    }
                    key.filters(*filters);
                }
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    key.data.extend([0, id.0]);
                    key.rect(*bounds);
                    key.filters(*filters);
                }
                DrawCommand::PopFilter => key.data.push(1),
                DrawCommand::PushClip(r) => {
                    key.data.push(2);
                    key.rect(*r);
                }
                DrawCommand::PopClip => key.data.push(3),
                DrawCommand::PushEffect {
                    transform,
                    opacity,
                    bounds,
                } => {
                    key.data.push(4);
                    for n in [
                        transform.m11,
                        transform.m12,
                        transform.m21,
                        transform.m22,
                        transform.dx,
                        transform.dy,
                        *opacity,
                    ] {
                        key.f(n);
                    }
                    key.rect(*bounds);
                }
                DrawCommand::PopEffect => key.data.push(5),
                DrawCommand::Image { rect, image } => {
                    key.data.extend([6, Arc::as_ptr(image) as usize as u64]);
                    key.rect(*rect);
                }
                DrawCommand::Vector {
                    rect,
                    icon,
                    color,
                    outline,
                } => {
                    key.data.extend([7, Arc::as_ptr(icon) as usize as u64]);
                    key.rect(*rect);
                    key.color(*color);
                    key.f(*outline);
                }
                DrawCommand::Svg { rect, image, color } => {
                    key.data.extend([
                        25,
                        *image as *const crate::ui::rendering::svg::SvgImage as usize as u64,
                    ]);
                    key.rect(*rect);
                    key.color(*color);
                }
                DrawCommand::Gradient {
                    rect,
                    radius,
                    gradient,
                } => {
                    key.data.push(8);
                    key.linear(gradient);
                    key.rect(*rect);
                    key.f(*radius);
                }
                DrawCommand::RadialGradient {
                    rect,
                    radius,
                    gradient,
                } => {
                    key.data.push(9);
                    key.radial(gradient);
                    key.rect(*rect);
                    key.f(*radius);
                }
                DrawCommand::Fill {
                    rect,
                    radius,
                    color,
                } => {
                    key.data.push(10);
                    key.rect(*rect);
                    key.f(*radius);
                    key.color(*color);
                }
                DrawCommand::Stroke {
                    rect,
                    radius,
                    width,
                    color,
                } => {
                    key.data.push(11);
                    key.rect(*rect);
                    key.f(*radius);
                    key.f(*width);
                    key.color(*color);
                }
                DrawCommand::Line {
                    start,
                    end,
                    width,
                    color,
                } => {
                    key.data.push(12);
                    for n in [start.x, start.y, end.x, end.y, *width] {
                        key.f(n);
                    }
                    key.color(*color);
                }
                DrawCommand::Path {
                    points,
                    closed,
                    color,
                } => {
                    key.data.extend([13, *closed as u64, points.len() as u64]);
                    for p in points {
                        key.f(p.x);
                        key.f(p.y);
                    }
                    key.color(*color);
                }
                DrawCommand::Text {
                    rect,
                    text,
                    size,
                    bold,
                    color,
                    wrap,
                } => {
                    key.data.extend([14, *bold as u64, *wrap as u64]);
                    key.rect(*rect);
                    key.f(*size);
                    key.color(*color);
                    key.texts.push(Arc::clone(text));
                }
                DrawCommand::FontText {
                    rect,
                    text,
                    size,
                    font,
                    color,
                    wrap,
                } => {
                    key.data.extend([15, *wrap as u64]);
                    key.rect(*rect);
                    key.f(*size);
                    key.font(*font);
                    key.color(*color);
                    key.texts.push(Arc::clone(text));
                }
                DrawCommand::RichText {
                    rect,
                    paragraph,
                    color,
                    wrap,
                } => {
                    let format = paragraph.format;
                    key.data.extend([
                        30,
                        *wrap as u64,
                        format.clip as u64,
                        format.align as u64,
                        format.decoration.underline as u64,
                        format.decoration.line_through as u64,
                        paragraph.spans.len() as u64,
                    ]);
                    key.rect(*rect);
                    key.f(format.size);
                    key.f(format.line_height.unwrap_or(0.));
                    key.font(format.font);
                    key.color(*color);
                    for span in &paragraph.spans {
                        let s = span.style;
                        key.data.extend([
                            span.range.start as u64,
                            span.range.end as u64,
                            s.weight.unwrap_or(0) as u64,
                            s.italic as u64,
                            s.decoration.underline as u64,
                            s.decoration.line_through as u64,
                            s.family.map_or(0, |f| f.as_ptr() as usize as u64),
                        ]);
                        key.color(s.color.unwrap_or(Color::rgba(0, 0, 0, 0)));
                        key.f(s.size.unwrap_or(0.));
                    }
                    key.texts.push(Arc::clone(&paragraph.text));
                }
                DrawCommand::FormattedText {
                    rect,
                    text,
                    format,
                    color,
                    wrap,
                } => {
                    key.data.extend([
                        16,
                        *wrap as u64,
                        format.clip as u64,
                        format.align as u64,
                        format.decoration.underline as u64,
                        format.decoration.line_through as u64,
                        format.decoration.overline as u64,
                    ]);
                    key.rect(*rect);
                    key.f(format.size);
                    key.f(format.line_height.unwrap_or(0.));
                    key.font(format.font);
                    key.color(*color);
                    key.texts.push(Arc::clone(text));
                }
                DrawCommand::Shape {
                    rect,
                    radii,
                    fill,
                    stroke,
                } => {
                    key.data
                        .extend([17, fill.is_some() as u64, stroke.is_some() as u64]);
                    key.rect(*rect);
                    for n in radii {
                        key.f(*n);
                    }
                    if let Some(fill) = fill {
                        key.background(fill);
                    }
                    if let Some((width, color)) = stroke {
                        key.f(*width);
                        key.color(*color);
                    }
                }
            }
        }
        key
    }
}
