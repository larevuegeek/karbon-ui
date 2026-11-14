//! Compilation d'un sous-ensemble CSS volontairement explicite en classes typées,
//! comme celles de `styles!`. Lecture et appariement ont lieu à la compilation.
//!
//! Organisation :
//! - `parser` : syntaxe des feuilles strictes (règles, at-rules, `@theme`…) ;
//! - `selector`, `media`, `animation` : sélecteurs, requêtes `@media`/`@container`,
//!   `@keyframes` et propriétés `animation-*` ;
//! - `values` : valeurs et propriétés CSS traduites en propriétés du moteur ;
//! - `class` : classe compilée (règles accumulées, états, rendu en fonction de style) ;
//! - `sheet` : feuille stricte de `css_file!` ; `framework` : feuilles de framework
//!   (cascade évaluée par élément) ; `module` : point d'entrée de `css_file!` ;
//! - `inline` : attribut `style="…"` des templates.
mod animation;
mod class;
pub(crate) mod framework;
mod inline;
mod media;
mod module;
pub(crate) mod parser;
mod selector;
mod sheet;
mod values;
pub(crate) use class::{
    Class, UtilityClass, display_kind, framework_animated_opacity, framework_class,
    framework_drop_animated_opacity, render_framework_class, strip_container, utility_class,
};
pub use inline::inline;
pub use module::{compile, module_tokens};
pub use parser::class_symbol;

mod conversions;
#[cfg(test)]
mod tests;
pub use conversions::*;
