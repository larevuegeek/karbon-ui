//! Numeric adjustment: drawing, range and provisional interaction, independent
//! of the OS.
use crate::application::Handler;
use crate::ui::{
    element::*,
    layout,
    runtime::{Key, UiEvent},
    scene::{DrawCommand, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

mod drawing;
mod events;
mod geometry;
mod state;
mod style;
mod widget;
pub use style::SliderStyle;
pub use widget::Slider;
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
struct Geometry {
    start: f32,
    length: f32,
    center: f32,
}

#[derive(Clone, Copy, PartialEq)]
struct Config {
    min: f64,
    max: f64,
    step: f64,
    axis: Axis,
}
#[derive(Clone, Copy)]
struct Edit {
    id: ElementId,
    config: Config,
    original: f64,
    value: f64,
    offset: f32,
    keys: u8,
    pointer: f32,
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
    drag: Option<Edit>,
    keyboard: Option<Edit>,
}
fn slider(root: &Element, id: ElementId) -> Option<(&Slider, RectF)> {
    let node = root.find(id)?;
    Some((node.widget.as_ref()?.slider()?, layout::content_rect(node)))
}
fn key_bit(key: Key) -> u8 {
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
fn commit(edit: Edit) -> Option<(ElementId, f64)> {
    if edit.value != edit.original {
        Some((edit.id, edit.value))
    } else {
        None
    }
}
