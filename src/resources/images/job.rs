//! Chargement d'une image sur un thread de travail : adresse `data:` décodée sur place,
//! sinon fichier choisi par la fonction `resolve` de l'application puis décodé en
//! miniature.
use super::super::{ImageDecoder, ImageLoadOptions, load_image_file};
use super::base64;
use crate::task::CancellationToken;
use crate::ui::scene::Image;
use std::path::PathBuf;
use std::sync::Arc;

/// Choix du fichier à lire pour une source et un côté (miniature déjà calculée…).
pub(super) type Resolve = Arc<dyn Fn(&str, u32) -> PathBuf + Send + Sync>;

/// Résultat d'une tâche du chargeur : l'image ou l'erreur.
pub(super) struct Loaded(pub(super) crate::Result<Arc<Image>>);

pub(super) fn load(
    source: &str,
    edge: u32,
    decoder: &dyn ImageDecoder,
    resolve: Option<&Resolve>,
    cancel: &CancellationToken,
) -> Loaded {
    let options = ImageLoadOptions {
        thumbnail_edge: edge,
        ..ImageLoadOptions::default()
    };
    if let Some(data) = source.strip_prefix("data:") {
        let malformed = || crate::Error::Image(crate::ImageError::Malformed { reason: "data URL" });
        let image = data
            .split_once(";base64,")
            .and_then(|(_, text)| base64::decode(text))
            .ok_or_else(malformed)
            .and_then(|bytes| decoder.decode(&bytes, options))
            .map(Arc::new);
        return Loaded(image);
    }
    let path = resolve.map_or_else(|| PathBuf::from(source), |r| r(source, edge));
    Loaded(load_image_file(decoder, &path, options, cancel).map(|loaded| loaded.image))
}
