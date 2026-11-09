//! Native UI: foundations, composition, components, layout, interaction and rendering.
//!
//! Most applications only need [`prelude`]. The submodules are grouped by concern:
//! [`core`] (elements, geometry, themes), [`view`] (builder functions such as
//! `row`/`column`), [`components`] (buttons, inputs, choices, sliders, selects…),
//! [`layout`], [`styling`], [`animation`], [`overlays`] (modals, popovers, tooltips),
//! [`template`] (support for the [`ui!`](crate::ui!) macros), [`rendering`] (scene and
//! paint types) and [`runtime`] (event handling, used by backends and tests).
//!
//! The flat re-exports below (`ui::element`, `ui::scroll`, `ui::widgets`…) are the
//! historical paths, kept for compatibility; each points to the single implementation
//! in its submodule.
pub mod animation;
pub mod bootstrap;
pub mod components;
pub mod core;
pub mod kit;
pub mod layout;
pub mod overlays;
pub mod prelude;
pub mod rendering;
pub mod runtime;
pub mod styling;
pub mod template;
#[cfg(test)]
pub(crate) mod tests;
pub mod view;

// Chemins historiques conservés ; chaque module possède une seule implémentation.
// Chemin utilisé par le code généré par ui-macros.
pub use components as widgets;
// Chemin utilisé par le code généré par ui-macros.
pub use core::{element, theme};
// Chemin utilisé par le code généré par ui-macros.
pub use layout::virtual_list;
// Chemin utilisé par le code généré par ui-macros.
pub use overlays::{modal, popover};
// Chemin utilisé par le code généré par ui-macros.
pub use rendering::font;
// Chemin utilisé par le code généré par ui-macros.
pub use runtime::drag_drop;
// Chemins publics conservés pour les exemples, les tests et les applications.
pub use components::{choices, select, slider, text_input, title_bar};
pub use core::{geometry, style};
pub use layout::scroll;
pub use overlays::{layers, tooltip};
pub use rendering::{scene, vector};
