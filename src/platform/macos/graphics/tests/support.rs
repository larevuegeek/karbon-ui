pub(super) use super::super::Renderer;
pub(super) use crate::ui::{scene::Scene, theme::Color};

pub(super) fn renderer() -> Renderer {
    Renderer::new().unwrap()
}
pub(super) fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}
pub(super) fn clear() -> Scene {
    Scene::new(Color::rgba(0, 0, 0, 0))
}
