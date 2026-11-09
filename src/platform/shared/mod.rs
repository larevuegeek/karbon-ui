//! Services sans dépendance à l'OS partagés par les renderers qui ne passent pas par
//! Direct2D : Core Graphics et Metal (macOS), Cairo (Linux). Filtres CSS sur CPU,
//! structure des portées d'une scène, géométrie, empreintes des caches, cache des
//! mises en page de texte, zones modifiées d'une image à l'autre (redessin partiel) et
//! structures envoyées aux GPU (Metal, Vulkan).
pub(in crate::platform) mod damage;
mod filters;
mod geometry;
pub(in crate::platform) mod gpu;
mod keys;
pub(in crate::platform) mod pacing;
mod scopes;
mod text_cache;

#[cfg(target_os = "linux")]
pub(in crate::platform) use filters::boxes;
pub(in crate::platform) use filters::{apply as apply_filters, reduction};
pub(in crate::platform) use geometry::{linear_points, normalized};
pub(in crate::platform) use keys::{hash, placement};
pub(in crate::platform) use scopes::{scope_end, single};
pub(in crate::platform) use text_cache::{TextCache, Typesetter, plain, simple_format};
