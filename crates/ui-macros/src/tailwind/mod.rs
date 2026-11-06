//! Utilitaires Tailwind CSS v4 compilés à la construction, sans Node.js.
//!
//! Une feuille qui commence par `@import "tailwindcss";` rend les utilitaires de
//! Tailwind utilisables dans les `class="…"` des templates qui l'emploient. Chaque liste
//! de classes d'un élément est compilée en un tout, comme le navigateur calcule les
//! styles d'un élément : les utilitaires produisent les déclarations de Tailwind (avec
//! leurs variables `--tw-*`), triées dans l'ordre de la feuille générée par Tailwind,
//! regroupées par condition (variantes), puis les variables `--tw-*` sont résolues pour
//! chaque condition et les propriétés composées (`transform`, `box-shadow`, `filter`,
//! `transition`, dégradés) traduites en propriétés du moteur.
//!
//! Organisation : `candidate` (découpage d'un jeton), `theme` (thème et `@theme`),
//! `variants` (conditions), `utilities` (déclarations de chaque utilitaire), `set`
//! (liste d'un élément), `order` (tri), `resolve` (variables et propriétés composées) ;
//! `tests` (comportement et oracle), `tools` (outils de mesure ignorés).
mod candidate;
mod order;
mod resolve;
mod set;
mod theme;
mod utilities;
mod variants;
pub use set::{Group, compile_set};
pub use theme::Config;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tools;
