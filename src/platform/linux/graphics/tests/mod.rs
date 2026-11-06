//! Tests scène → pixels du renderer Cairo et du texte Pango, à plusieurs échelles
//! (modèle : `platform/macos/graphics/tests/`).
mod damage;
mod effects;
mod glyphs;
mod paint;
mod text;

use super::Renderer;
use crate::ui::{scene::Scene, theme::Color};

fn renderer() -> Renderer {
    Renderer::new().unwrap()
}
fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}
fn clear() -> Scene {
    Scene::new(Color::rgba(0, 0, 0, 0))
}
