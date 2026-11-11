//! Limite des couches : un groupe filtré de plus de 16 M pixels se dessine sans ses
//! filtres, par Vulkan comme par Cairo, au lieu de faire échouer l'image entière (ce qui
//! couperait Vulkan pour la fenêtre).
use super::{pixel, renderer};
use crate::ui::{
    element::ElementId,
    rendering::filter::{Filter, Filters},
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};

#[test]
fn a_filter_group_over_16m_pixels_is_drawn_without_its_filters() {
    static BLUR: [Filter; 1] = [Filter::Blur(20.)];
    let (width, height) = (4100usize, 4100usize);
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    let bounds = RectF::new(0., 0., width as f32, height as f32);
    scene.commands.extend([
        DrawCommand::PushFilter {
            id: ElementId(1),
            bounds,
            filters: Filters {
                values: &BLUR,
                scale: 1.,
            },
        },
        DrawCommand::Fill {
            rect: bounds,
            radius: 0.,
            color: Color::rgb(40, 120, 220),
        },
        DrawCommand::PopFilter,
    ]);
    let expected = [220, 120, 40, 255];
    let Some(mut gpu) = renderer() else {
        return;
    };
    let pixels = gpu.render(&scene, width, height, 1.).expect("Vulkan image");
    assert_eq!(pixel(&pixels, width, 2000, 2000), expected);
    let pixels = gpu
        .cairo
        .render(&scene, width as u32, height as u32, 1.)
        .expect("Cairo image");
    assert_eq!(pixel(&pixels, width, 2000, 2000), expected);
}
