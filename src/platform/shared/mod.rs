//! Services sans dépendance à l'OS partagés par les renderers qui ne passent pas par
//! Direct2D : Core Graphics et Metal (macOS), Cairo (Linux). Filtres CSS sur CPU,
//! structure des portées d'une scène, géométrie, empreintes des caches, cache des
//! mises en page de texte, zones modifiées d'une image à l'autre (redessin partiel) et
//! structures envoyées aux GPU (Metal, Vulkan), résumé de l'affichage réel des images
//! pendant la sonde de défilement. Sous Windows, seules les zones modifiées servent (le
//! redessin partiel de Direct2D, KUI-055).
pub(in crate::platform) mod damage;
#[cfg(not(target_os = "windows"))]
mod filters;
#[cfg(not(target_os = "windows"))]
mod geometry;
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) mod gpu;
#[cfg(not(target_os = "windows"))]
mod keys;
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) mod pacing;
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) mod presentation;
#[cfg(not(target_os = "windows"))]
mod scopes;
#[cfg(not(target_os = "windows"))]
mod text_cache;

#[cfg(target_os = "linux")]
pub(in crate::platform) use filters::boxes;
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) use filters::{apply as apply_filters, reduction};
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) use geometry::{linear_points, normalized};
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) use keys::{hash, placement};
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) use scopes::{scope_end, single};
#[cfg(not(target_os = "windows"))]
pub(in crate::platform) use text_cache::{TextCache, Typesetter, plain, simple_format};
