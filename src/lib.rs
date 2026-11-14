//! Native Rust UI library: applications describe their content as a tree of
//! [`Element`](ui::element::Element)s and the selected native backend lays it out,
//! renders it and routes input back as typed messages.
//!
//! # Overview
//!
//! - [`application`]: the [`Application`](application::Application) trait. An
//!   application owns its state, builds its view in
//!   [`view`](application::Application::view) and handles messages in
//!   [`update`](application::Application::update) (returning `true` rebuilds the view).
//! - [`ui`](mod@ui): elements, components, layout, styling, animation and the runtime.
//!   [`ui::prelude`] imports everything needed to write a view.
//! - [`ui!`], [`ui_file!`], [`styles!`] and [`css_file!`]: compile-time templates and
//!   stylesheets producing the same `Element` tree, with no runtime parser;
//!   [`icons_dir!`] compiles a folder of SVG icons.
//! - [`window`] and [`platform`]: window options and the backend entry points
//!   ([`platform::run`], [`platform::run_windows`]).
//! - [`task`]: background work with cooperative cancellation.
//! - [`intl`]: numbers, durations, relative times and dates in the user's language.
//! - [`resources`]: bounded image loading and caching.
//! - [`storage`] and [`secrets`]: the application's preferences (like the web's
//!   `localStorage`) and its passwords, encrypted by the operating system.
//! - [`diagnostics`]: offscreen benchmarks and previews.
//! - [`platform::headless`]: windowless test harness with deterministic text metrics.
//! - [`Error`] and [`Result`]: the error type returned by every fallible API.
//!
//! # Platforms
//!
//! Interactive backends exist for Windows (Win32, Direct2D, DirectWrite), macOS (AppKit,
//! Metal, Core Text) and Linux (Wayland or X11, Vulkan or Cairo, Pango); on any other
//! target the window entry points return [`Error::Unsupported`]. The
//! [`platform::headless`] backend runs everywhere: it drives applications without a
//! window for tests ([`HeadlessApp`](platform::headless::HeadlessApp)) and renders
//! previews with a software rasteriser.
//!
//! # Example
//!
//! ```no_run
//! use karbon_ui::{platform, ui::prelude::*, window::WindowOptions};
//!
//! #[derive(Clone)]
//! enum Msg {
//!     Increment,
//! }
//!
//! #[derive(Default)]
//! struct Counter {
//!     count: u32,
//! }
//!
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
//!         true // rebuild the view
//!     }
//! }
//!
//! fn main() -> karbon_ui::Result<()> {
//!     platform::run(
//!         WindowOptions {
//!             title: "Counter",
//!             ..WindowOptions::DEFAULT
//!         },
//!         Counter::default(),
//!     )
//! }
//! ```
#![warn(missing_docs)]
pub mod application;
pub mod diagnostics;
mod error;
pub mod intl;
pub mod platform;
pub mod resources;
pub mod secrets;
pub mod storage;
pub mod task;
pub mod ui;
pub mod window;

pub use error::{Error, ImageError, Result};

#[doc(hidden)]
pub use karbon_ui_macros::{
    compile_classes as __compile_classes, compile_css_file as __compile_css_file,
    compile_icons_dir as __compile_icons_dir, compile_styles as __compile_styles,
    compile_ui as __compile_ui, compile_ui_file as __compile_ui_file,
};

/// Compiled native UI written as HTML. Rust expressions stay between braces; no runtime
/// parser.
///
/// The markup is checked and translated into [`Element`](ui::element::Element)
/// builder calls at compile time. Text is written directly (HTML whitespace rules and
/// entities apply); `{expr}` shows a Rust value, `attr={expr}` passes a typed Rust value
/// (messages, data, images), and `{#if}` / `{#each}` blocks control the structure.
/// Lowercase tags map to native components (`div`, `p`, `button`, `input`, `select`,
/// `img`, `dialog`…), capitalised tags to Rust [`Component`](ui::template::Component)s.
/// Unsupported tags, attributes and events are compile errors. Optional leading
/// directives such as `@context(cx);`, `@slots(slots);` and `@styles(css);` select the
/// [`TemplateContext`](ui::template::TemplateContext), slots and stylesheet to use.
///
/// Inside `ui!` the markup must also be valid Rust tokens (balanced quotes, no `//`);
/// [`ui_file!`] has no such restriction.
///
/// ```
/// use karbon_ui::ui::prelude::*;
///
/// #[derive(Clone)]
/// enum Msg {
///     Play,
/// }
///
/// let title = "Hello";
/// let page: Element = ui! {
///     <section style="padding: 24px; gap: 16px">
///         <h1>{title}</h1>
///         <p>A <b>native</b> page, no browser.</p>
///         <button onclick={Msg::Play}>Play</button>
///     </section>
/// };
/// # let _ = page;
/// ```
#[macro_export]
macro_rules! ui {
    ($($tokens:tt)*) => { $crate::__compile_ui!(($crate) $($tokens)*) };
}
/// Include a native HTML template (`.ui` file) relative to the Rust source file at
/// compile time.
///
/// The file holds the same markup as [`ui!`], without the `ui! { ... }` wrapper,
/// and the build is re-run when it (or a file it includes with `{@include "…"}`)
/// changes. Optional `context = ...`, `scope = ...`, `slots = ...` and `styles = ...`
/// arguments follow the path:
///
/// ```ignore
/// let cx = TemplateContext::default();
/// ui_file!("views/home.ui", context = cx, styles = css)
/// ```
#[macro_export]
macro_rules! ui_file {
    ($($tokens:tt)*) => { $crate::__compile_ui_file!(($crate) $($tokens)*) };
}
/// Typed reusable style functions. Inline styles and attributes override classes.
///
/// Each `NAME { property: value; ... }` entry becomes a style function usable as a
/// template class (`class={NAME}`, or `class={[BASE, NAME]}` to combine, applied in
/// order).
///
/// ```
/// use karbon_ui::ui::prelude::*;
///
/// styles! {
///     pub CARD { padding: 20; gap: 12; radius: 16; background: "#17202b"; }
///     pub TITLE { font_size: 24; weight: 700; color: "#ffffff"; }
/// }
///
/// let card: Element = ui! {
///     <article class={CARD}>
///         <h2 class={TITLE}>My card</h2>
///     </article>
/// };
/// # let _ = card;
/// ```
#[macro_export]
macro_rules! styles {
    ($($tokens:tt)*) => { $crate::__compile_styles!(($crate) $($tokens)*); };
}
/// Compile a local stylesheet into typed style functions, without a runtime parser.
///
/// `css_file!(mod css, "styles.css");` reads the file relative to the Rust source at
/// compile time and generates a module `css` with one constant per class (e.g.
/// `.card` becomes `css::CARD`). Pass it to templates with `styles = css` (or
/// `@styles(css);` in [`ui!`]) to use `class="card"`. Only a documented subset of CSS
/// is supported.
///
/// ```ignore
/// css_file!(mod css, "views/styles.css");
/// ```
///
/// An existing framework stylesheet (Bootstrap, a Tailwind build output…) is compiled
/// in *framework mode*: unsupported selectors, declarations and at-rules are skipped
/// and counted instead of failing, and templates using the module apply the full
/// cascade (specificity, order, `!important`, `@layer`, type and attribute selectors)
/// to each element. The module exports `COMPATIBILITY` and `REPORT`; `report` prints
/// the summary while compiling, `report = "all"` every skipped item.
///
/// ```ignore
/// css_file!(mod bs, "vendor/bootstrap.css", framework);
/// css_file!(mod bs, "vendor/bootstrap.css", framework, report);
/// ```
#[macro_export]
macro_rules! css_file {
    ($($tokens:tt)*) => {$crate::__compile_css_file!(($crate) $($tokens)*);};
}
/// Compile a folder of SVG icons into a module that returns an icon by name.
///
/// `icons_dir!(pub mod icons, "assets/icons");` reads the folder relative to the Rust
/// source at compile time. Each `<set>/<name>.svg` becomes the icon `"set:name"` (the
/// Iconify naming, e.g. `lucide/plus.svg` is `"lucide:plus"`), and a file at the root of
/// the folder, `<name>.svg`, the icon `"name"`. Other files (a licence list, notes) are
/// ignored. Every file is compiled like `<svg src="…">` in a template: an unsupported SVG
/// element is a build error naming the file, and `currentColor` takes the CSS `color` of
/// the element.
///
/// The module holds `NAMES`, the sorted list of names, and
/// `svg(cx, name) -> Option<Element>`, for names known at run time only (data, settings):
///
/// ```ignore
/// icons_dir!(pub mod icons, "assets/icons");
///
/// let cx = TemplateContext::default();
/// let play = icons::svg(&cx, "lucide:play").expect("lucide/play.svg is in the folder");
/// ```
///
/// Cargo rebuilds the crate when one of the files read changes, but not when a file is
/// added to the folder. To pick up new icons without touching a Rust file, declare the
/// folder in the build script of the package (`build.rs`):
///
/// ```ignore
/// println!("cargo:rerun-if-changed=src/assets/icons");
/// ```
#[macro_export]
macro_rules! icons_dir {
    ($($tokens:tt)*) => {$crate::__compile_icons_dir!(($crate) $($tokens)*);};
}
