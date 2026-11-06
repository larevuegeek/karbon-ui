use super::ComputedStyle;
use crate::ui::{
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
/// Fills `rect` with `color`, using the computed per-corner radii when present,
/// otherwise the uniform `radius`.
pub fn fill(
    scene: &mut Scene,
    rect: RectF,
    radius: f32,
    color: Color,
    style: Option<&ComputedStyle>,
) {
    if let Some(radii) = style.and_then(|s| s.local.corner_radii) {
        scene.commands.push(DrawCommand::Shape {
            rect,
            radii,
            fill: Some(color.into()),
            stroke: None,
        });
    } else {
        scene.fill(rect, radius, color);
    }
}
/// Strokes the outline of `rect` with `width` and `color`, using the computed
/// per-corner radii when present, otherwise the uniform `radius`.
pub fn border(
    scene: &mut Scene,
    rect: RectF,
    radius: f32,
    width: f32,
    color: Color,
    style: Option<&ComputedStyle>,
) {
    if let Some(radii) = style.and_then(|s| s.local.corner_radii) {
        scene.commands.push(DrawCommand::Shape {
            rect,
            radii,
            fill: None,
            stroke: Some((width, color)),
        });
    } else {
        crate::ui::style::border(scene, rect, radius, width, color);
    }
}
