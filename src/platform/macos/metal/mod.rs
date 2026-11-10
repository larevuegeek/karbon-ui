//! Renderer GPU Metal des fenêtres macOS (Apple Silicon et Mac Intel).
//!
//! Chaque image est redessinée en entière : la scène est traduite en couches
//! d'instances (`encode/`), un quad par élément, dont le fragment calcule la couverture
//! par distance signée (rectangles à coins indépendants, traits, segments) ou lit une
//! texture (images, atlas de glyphes, icônes et SVG rastérisés une fois par Core
//! Graphics). Les découpes rectangulaires deviennent scissor + couverture, les découpes
//! arrondies ou tournées et les masques en dégradé une chaîne évaluée par le shader, les
//! groupes à opacité et les groupes filtrés des textures intermédiaires bornées à leur
//! contenu ; filtres et `backdrop-filter` passent par des passages de filtre (`filter/`).
//!
//! Le renderer Core Graphics (`graphics/`) reste le repli (mode logiciel, pas de GPU,
//! erreur du GPU, image sans drawable, scène refusée par `Error::Unsupported`, aucune
//! aujourd'hui) et garde le rendu hors écran de référence (aperçus, exports, mesures). Il
//! prête aussi au chemin Metal ses mises en page Core Text et sa rastérisation.
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `device.rs` | périphérique, file, shaders compilés au lancement, pipeline, stockage |
//! | `shaders.metal` | source MSL embarquée |
//! | `instance.rs` | structures partagées avec les shaders |
//! | `encode/` | scène → couches, lots d'instances et étapes (formes, texte, images, raster, effets) |
//! | `filter/` | filtres CSS en passages GPU (flou, ombre, matrices de couleur) |
//! | `compare.rs` | diagnostic `KARBON_METAL_COMPARE` (Metal contre Core Graphics) |
//! | `atlas.rs` | atlas de glyphes Core Text |
//! | `resources.rs` | textures gardées d'une image à l'autre |
//! | `submit.rs` | étapes de l'image, anneau de tampons, relecture |
//! | `surface.rs` | `CAMetalLayer` de la fenêtre |
mod atlas;
mod compare;
mod device;
mod encode;
mod filter;
mod instance;
mod resources;
mod submit;
mod surface;
#[cfg(test)]
mod tests;
pub(super) use compare::Comparison;
pub(super) use resources::Trim;
pub(super) use surface::Surface;
mod renderer;
pub(crate) use renderer::*;
