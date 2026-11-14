//! Chargement d'images pour la vue (`ImageLoader`) et teinte dominante d'une image.
// Organisation : `loader` (cache, demandes, tâches et sources d'image), `job` (travail
// d'une tâche : adresse `data:` ou fichier), `base64` (adresses `data:`), `tint`
// (`Image::dominant_hue`).
mod base64;
mod job;
mod loader;
#[cfg(test)]
mod tests;
mod tint;
pub use loader::ImageLoader;
