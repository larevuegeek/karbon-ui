//! RustMusic home page, translated from the Svelte/Tailwind sources into `.ui`
//! templates and plain CSS. Static data, no audio: a visual comparison target.
mod assets;
mod color;
mod components;
mod data;
pub mod dump;
mod icons;
mod menus;
pub use menus::{Menu, Veille};
#[cfg(test)]
mod tests;

css_file!(pub mod css, "styles/app.css");
mod music_app;
pub use music_app::*;
