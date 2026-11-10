use super::*;
pub(super) use crate::application::Handler;
pub(super) use crate::ui::{
    element::*,
    layout,
    runtime::{Key, UiEvent},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
pub(super) use std::sync::Arc;
/// Accessibility range snapshot of a [`Slider`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RangeInfo {
    /// Current value, including a provisional value during an interaction.
    pub value: f64,
    /// Lower bound.
    pub min: f64,
    /// Upper bound.
    pub max: f64,
    /// Step applied by arrow keys.
    pub small_change: f64,
    /// Step applied by Page Up/Down (ten steps, capped at the range).
    pub large_change: f64,
    /// Whether the value cannot be changed by the user.
    pub read_only: bool,
    /// Orientation of the slider.
    pub axis: Axis,
}
pub(super) struct Geometry {
    pub(super) start: f32,
    pub(super) length: f32,
    pub(super) center: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Config {
    pub(super) min: f64,
    pub(super) max: f64,
    pub(super) step: f64,
    pub(super) axis: Axis,
}
#[derive(Clone, Copy)]
pub(super) struct Edit {
    pub(super) id: ElementId,
    pub(super) config: Config,
    pub(super) original: f64,
    pub(super) value: f64,
    pub(super) offset: f32,
    pub(super) keys: u8,
    pub(super) pointer: f32,
}
#[derive(Default)]
pub(crate) struct SliderResponse {
    pub handled: bool,
    pub repaint: bool,
    pub capture: bool,
    pub release: bool,
    /// Le runtime résout le gestionnaire dans l'arbre courant au moment de l'envoi.
    pub change: Option<(ElementId, f64)>,
    pub commit: Option<(ElementId, f64)>,
}
#[derive(Default)]
pub(crate) struct Sliders {
    pub(super) drag: Option<Edit>,
    pub(super) keyboard: Option<Edit>,
}
pub(super) fn slider(root: &Element, id: ElementId) -> Option<(&Slider, RectF)> {
    let node = root.find(id)?;
    Some((node.widget.as_ref()?.slider()?, layout::content_rect(node)))
}
pub(super) fn key_bit(key: Key) -> u8 {
    match key {
        Key::Left => 1,
        Key::Right => 2,
        Key::Up => 4,
        Key::Down => 8,
        Key::Home => 16,
        Key::End => 32,
        Key::PageUp => 64,
        Key::PageDown => 128,
        _ => 0,
    }
}
pub(super) fn commit(edit: Edit) -> Option<(ElementId, f64)> {
    if edit.value != edit.original {
        Some((edit.id, edit.value))
    } else {
        None
    }
}
