//! Compile-time templates target these small, statically dispatched construction helpers.
mod action;
mod aria;
mod context;
mod html;
mod image;
mod image_slot;
mod node;
mod slots;
/// Text style capabilities that template attributes map to on custom widgets.
pub mod typography;
pub use aria::*;
pub use context::*;
pub use html::*;
pub use image::ImageWidget;
pub(crate) use image::intrinsic as image_intrinsic;
pub use image_slot::ImageSlot;
pub use node::{ContainerStyle, Empty, Node, style, unstyled};
pub use slots::{Component, Slots};
mod helpers;
pub use helpers::*;
