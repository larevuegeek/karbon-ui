//! Génération des constructeurs natifs depuis le balisage HTML.
//!
//! Chaque balise devient un `Node` du moteur : la feuille de style par défaut
//! s'applique d'abord, puis les classes, `style="…"`, les attributs, et enfin les
//! comportements posés sur l'élément construit (infobulle, glisser-déposer…).
//!
//! `Generator` accumule le code Rust produit (texte relu en jetons par
//! [`Output`]) ; ses méthodes sont réparties par sorte d'élément : parcours des
//! enfants et blocs (`tree`), élément générique et finition (`element`), styles et
//! classes (`styles`), attributs d'état et ARIA (`attributes`), animations
//! (`motion`), texte (`text`, `runs`), conteneurs et mises en page (`layout`),
//! champs (`inputs`), boutons (`buttons`), listes (`select`), images et SVG
//! (`media`), surcouches (`overlays`), composants et inclusions (`components`).
mod aria;
mod attributes;
mod buttons;
mod components;
mod element;
mod generator;
mod inputs;
mod layout;
mod media;
mod motion;
mod overlays;
mod runs;
mod select;
mod styles;
mod text;
mod textarea;
mod tree;
pub use generator::*;
