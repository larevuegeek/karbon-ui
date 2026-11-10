pub(super) use crate::ui::{
    element::Insets,
    layout,
    scene::{DrawCommand, RectF, Scene},
    theme::{Color, Theme},
};
pub(crate) fn font_size(size: f32) -> crate::Result<()> {
    if size.is_finite() && size > 0. {
        Ok(())
    } else {
        Err(crate::Error::invalid_input("invalid font size"))
    }
}
pub(super) fn dimensions(values: &[f32]) -> crate::Result<()> {
    if values.iter().all(|v| v.is_finite() && *v >= 0.) {
        Ok(())
    } else {
        Err(crate::Error::invalid_input("invalid style dimensions"))
    }
}
pub(crate) fn border(scene: &mut Scene, rect: RectF, radius: f32, width: f32, color: Color) {
    let width = width.min(layout::width(rect)).min(layout::height(rect));
    if width <= 0. {
        return;
    }
    let inset = width / 2.;
    scene.commands.push(DrawCommand::Stroke {
        rect: RectF::new(
            rect.left + inset,
            rect.top + inset,
            (layout::width(rect) - width).max(0.),
            (layout::height(rect) - width).max(0.),
        ),
        radius: (radius - inset).max(0.),
        width,
        color,
    });
}
