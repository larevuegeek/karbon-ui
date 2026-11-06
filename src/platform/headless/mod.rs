//! Headless backend: run and test applications without any window, on every target.
//!
//! It is pure Rust and fully deterministic:
//!
//! - [`HeadlessApp`] drives an [`Application`](crate::application::Application) like a
//!   window would: build the view, inject clicks, typing, keys, scrolling, resizes and
//!   time, run background tasks, then read back the
//!   [`Scene`](crate::ui::scene::Scene), element bounds, focus and the accessibility
//!   tree.
//! - [`HeadlessTextMeasurer`] measures text with fixed, documented metrics, so layouts
//!   are identical on Windows, macOS and Linux.
//! - Images passed to
//!   [`Resources::decode_image`](crate::application::Resources::decode_image) are not
//!   decoded: their PNG/JPEG header is validated and a mid-grey placeholder of the same
//!   size is returned (background decoders produce placeholders of the thumbnail size).
//! - A small software rasteriser turns the scene into pixels
//!   ([`HeadlessApp::render`]): rounded rectangles, strokes, lines, paths, images, clips,
//!   transforms and group opacity are drawn exactly (one sample per pixel, no
//!   anti-aliasing); gradients use the average of their stops, filters are ignored,
//!   vector icons are skipped and each glyph is drawn as a box at the measurer's metrics.
//!
//! This backend has no event loop: [`platform::run`](crate::platform::run) still
//! returns [`Error::Unsupported`](crate::Error::Unsupported) on targets without a native
//! backend. There, [`diagnostics::render_preview`](crate::diagnostics::render_preview)
//! and [`render_benchmark`](crate::diagnostics::render_benchmark) use the headless
//! rasteriser, and [`write_png`](crate::diagnostics::write_png) a built-in encoder.
//!
//! ```
//! use karbon_ui::{platform::headless::HeadlessApp, ui::prelude::*};
//!
//! #[derive(Clone)]
//! enum Msg {
//!     Increment,
//! }
//! #[derive(Default)]
//! struct Counter {
//!     count: u32,
//! }
//! impl Application for Counter {
//!     type Message = Msg;
//!     fn view(&self, _: &ViewContext) -> Element {
//!         column("counter", [
//!             text("count", format!("Count: {}", self.count)),
//!             button("increment", "+1", Msg::Increment),
//!         ])
//!     }
//!     fn update(&mut self, message: Msg) -> bool {
//!         match message {
//!             Msg::Increment => self.count += 1,
//!         }
//!         true
//!     }
//! }
//!
//! let mut app = HeadlessApp::new(Counter::default())?;
//! app.click("increment")?;
//! app.click("increment")?;
//! assert_eq!(app.app().count, 2);
//! assert!(app.contains_text("Count: 2"));
//! # Ok::<(), karbon_ui::Error>(())
//! ```
mod app;
mod backend;
mod png;
mod raster;
mod resources;
#[cfg(test)]
mod tests;
mod text;

pub use app::{HeadlessApp, HeadlessOptions};
pub use text::HeadlessTextMeasurer;

pub(crate) use backend::Headless;
use backend::{HeadlessRenderer, LABEL};
