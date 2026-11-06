//! Couleurs et pinceaux : prémultiplication, mélange, dégradés moyennés, aplats, tracés
//! et images (plus proche voisin).
use super::{
    Canvas,
    shape::{Rounded, Shape},
};
use crate::ui::{
    rendering::brush::{Background, GradientStop},
    scene::{Image, RectF},
    theme::Color,
};

pub(super) fn premultiplied(color: Color, opacity: f32) -> [u8; 4] {
    let alpha = (f32::from(color.alpha) * opacity.clamp(0., 1.)).round() as u32;
    let channel = |c: u8| ((u32::from(c) * alpha + 127) / 255) as u8;
    [
        channel(color.blue),
        channel(color.green),
        channel(color.red),
        alpha as u8,
    ]
}
pub(super) fn blend(destination: &mut [u8], source: [u8; 4]) {
    let inverse = 255 - u32::from(source[3]);
    for (d, s) in destination.iter_mut().zip(source) {
        *d = (u32::from(s) + (u32::from(*d) * inverse + 127) / 255).min(255) as u8;
    }
}
pub(super) fn average(stops: &[GradientStop]) -> Color {
    let count = stops.len().max(1) as u32;
    let mean = |f: fn(&Color) -> u8| {
        (stops.iter().map(|s| u32::from(f(&s.color))).sum::<u32>() / count) as u8
    };
    Color {
        red: mean(|c| c.red),
        green: mean(|c| c.green),
        blue: mean(|c| c.blue),
        alpha: mean(|c| c.alpha),
    }
}

impl Canvas<'_> {
    pub(super) fn fill(&mut self, shape: Shape, color: Color) {
        let source = premultiplied(color, self.opacity);
        if source[3] == 0 {
            return;
        }
        self.cover(shape.bounds(), |p| shape.contains(p).then_some(source));
    }
    pub(super) fn background(&mut self, rect: RectF, radii: [f32; 4], fill: &Background) {
        let color = match fill {
            Background::Solid(color) => *color,
            Background::Linear(gradient) => average(&gradient.stops),
            Background::Radial(gradient) => average(&gradient.stops),
        };
        self.fill(Shape::Rounded(Rounded::new(rect, radii)), color);
    }
    pub(super) fn stroke(&mut self, rect: RectF, radii: [f32; 4], width: f32, color: Color) {
        let outer = Rounded::new(rect, radii).inset(-width / 2.);
        self.fill(
            Shape::Ring {
                outer,
                inner: outer.inset(width),
            },
            color,
        );
    }
    pub(super) fn image(&mut self, rect: RectF, image: &Image) {
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        if width <= 0. || height <= 0. || image.width == 0 || image.height == 0 {
            return;
        }
        let opacity = self.opacity;
        self.cover(rect, |p| {
            if p.x < rect.left || p.x >= rect.right || p.y < rect.top || p.y >= rect.bottom {
                return None;
            }
            let u = (((p.x - rect.left) / width * image.width as f32) as u32).min(image.width - 1);
            let v =
                (((p.y - rect.top) / height * image.height as f32) as u32).min(image.height - 1);
            let offset = (v as usize * image.width as usize + u as usize) * 4;
            let pixel = image.pixels.get(offset..offset + 4)?;
            let scale = |c: u8| (f32::from(c) * opacity.clamp(0., 1.)).round() as u8;
            Some([
                scale(pixel[0]),
                scale(pixel[1]),
                scale(pixel[2]),
                scale(pixel[3]),
            ])
        });
    }
}
