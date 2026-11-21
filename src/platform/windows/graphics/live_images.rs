//! Bitmaps des images vivantes (`LiveImage`) : la nouvelle image d'une source déjà
//! montrée, de même taille, est recopiée dans le bitmap de la précédente
//! (`ID2D1Bitmap::CopyFromMemory`) au lieu d'un bitmap neuf.
use super::*;

impl Canvas {
    /// Pour chaque image de `scene` sans bitmap qui est la dernière d'une source vivante :
    /// reprendre le bitmap de l'image précédente de la source et y copier les pixels, si
    /// la taille est la même ; sinon `prepare_images` en crée un, retenu pour la suite.
    pub(super) fn update_live_bitmaps(&mut self, scene: &Scene) -> crate::Result<()> {
        let bitmaps = &self.bitmaps;
        self.live.retain(|_, key| bitmaps.contains_key(key));
        for command in &scene.commands {
            let DrawCommand::Image { image, .. } = command else {
                continue;
            };
            let key = Arc::as_ptr(image) as usize;
            if self.bitmaps.contains_key(&key) {
                continue;
            }
            let Some(source) = crate::ui::scene::live_source(image) else {
                continue;
            };
            let previous = self.live.insert(source, key);
            let Some((old, bitmap)) = previous.and_then(|old| self.bitmaps.remove(&old)) else {
                continue;
            };
            let same = old.width == image.width
                && old.height == image.height
                && image.pixels.len() as u64
                    == u64::from(image.width) * u64::from(image.height) * 4;
            if !same {
                continue;
            }
            // SAFETY : bitmap créé par ce contexte à la taille de l'image ; tampon de
            // largeur × hauteur × 4 octets vérifié ci-dessus, pas = largeur × 4, lu pendant
            // l'appel seulement.
            unsafe { bitmap.CopyFromMemory(None, image.pixels.as_ptr().cast(), image.width * 4) }
                .check("CopyFromMemory")?;
            self.bitmaps.insert(key, (Arc::clone(image), bitmap));
        }
        Ok(())
    }
}
