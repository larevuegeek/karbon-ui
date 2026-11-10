//! Renderer GPU Vulkan de Linux : infrastructure (étape 1) et traduction de toutes les
//! scènes (étape 2) ; sert les rendus hors écran en mode `Gpu`, les fenêtres à l'étape 3.
//!
//! Même conception que le renderer Metal de macOS : chaque élément est un quad instancié
//! (`platform/shared/gpu.rs`) dont le fragment calcule la couverture par distance signée
//! ou lit une texture ; shaders GLSL portés de `macos/metal/shaders.metal`, compilés en
//! SPIR-V versionné (`shaders/build.sh`). Vulkan 1.3 (rendu dynamique, synchronization2)
//! et descripteurs poussés, chargé au lancement : sans Vulkan, Cairo dessine.
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `device.rs` | instance, choix du GPU, appareil logique et file |
//! | `memory.rs` | tampons projetés et images du GPU |
//! | `pipeline.rs` | shaders SPIR-V, disposition des descripteurs, pipeline graphique |
//! | `commands.rs` | tampon de commandes, barrières, passage de rendu des lots |
//! | `resources.rs` | ressources gardées d'une image à l'autre et leurs envois |
//! | `atlas.rs` | glyphes rastérisés par Cairo, pages et copies vers le GPU |
//! | `textures.rs` | textures des images et des rastérisations, mipmaps |
//! | `staging.rs` | tampon intermédiaire des envois |
//! | `encode/` | scène → couches, lots et étapes (copie de `macos/metal/encode/`) |
//! | `filter/` | passages des filtres CSS (copie adaptée de `macos/metal/filter/`) |
//! | `submit.rs` | exécution des étapes d'une image, textures des couches |
//! | `surface.rs` | surface Wayland et chaîne d'échange d'une fenêtre |
//! | `present.rs` | image d'une fenêtre : traduction, rendu, présentation |
//! | `shaders/` | sources GLSL et SPIR-V compilé |
mod atlas;
mod commands;
mod device;
mod encode;
mod filter;
mod memory;
mod pipeline;
mod present;
mod resources;
mod staging;
mod submit;
mod surface;
#[cfg(test)]
mod tests;
mod textures;
pub(super) use device::Gpu;
pub(super) use present::Presented;
pub(super) use surface::{Native, Surface};
mod renderer;
pub(crate) use renderer::*;
