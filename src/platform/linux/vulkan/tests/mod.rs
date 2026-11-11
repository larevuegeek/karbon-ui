//! Tests du renderer Vulkan, hors écran : sur un GPU s'il y en a un, sinon sur llvmpipe
//! (Vulkan sur processeur). Sans Vulkan, ils se sautent en l'écrivant sur la sortie
//! d'erreur ; `KARBON_REQUIRE_VULKAN=1` en fait des échecs. `KARBON_VULKAN_DUMP=<dossier>`
//! y enregistre les images comparées à Cairo (`dump`).
mod filters;
mod images;
mod instances;
mod layers;
mod limits;
mod scenes;
mod support;
mod text;
pub(crate) use support::*;
