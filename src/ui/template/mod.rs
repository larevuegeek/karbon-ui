//! Compile-time templates target these small, statically dispatched construction helpers.
mod action;
mod context;
mod html;
mod image;
mod node;
mod slots;
/// Text style capabilities that template attributes map to on custom widgets.
pub mod typography;
pub use context::*;
pub use html::*;
pub use image::ImageWidget;
pub use node::{ContainerStyle, Empty, Node, style, unstyled};
pub use slots::{Component, Slots};
/// Whether the base `display` of a class list makes a flex or grid container: the last
/// class declaring `display` wins (`-1`: none, `0`: block, `1`: flex, `2`: grid). Inline
/// children of such a container become separate items, as in HTML.
pub const fn blockifies(displays: &[i8]) -> bool {
    let mut index = displays.len();
    while index > 0 {
        index -= 1;
        if displays[index] >= 0 {
            return displays[index] > 0;
        }
    }
    false
}
