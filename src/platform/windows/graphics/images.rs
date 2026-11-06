//! Bitmaps Direct2D des images de la scène, conservés tant que leur `Arc` y reste.
use super::*;

impl Canvas {
    pub(super) fn prepare_images(&mut self, scene: &Scene) -> crate::Result<()> {
        // Garder seulement les images de la scène courante ; les Arc empêchent la réutilisation d'une adresse encore en cache.
        self.bitmaps.retain(|key, _| {
            scene.commands.iter().any(
                |c| matches!(c,DrawCommand::Image {image,..} if Arc::as_ptr(image) as usize==*key),
            )
        });
        for command in &scene.commands {
            if let DrawCommand::Image { image, .. } = command {
                let key = Arc::as_ptr(image) as usize;
                if self.bitmaps.contains_key(&key) {
                    continue;
                }
                if image.width == 0
                    || image.height == 0
                    || image.width > 16384
                    || image.height > 16384
                {
                    return Err(crate::Error::invalid_input("invalid image buffer"));
                }
                if (image.width as u64) * (image.height as u64) * 4 != image.pixels.len() as u64 {
                    return Err(crate::Error::invalid_input("invalid image buffer size"));
                }
                let props = D2D1_BITMAP_PROPERTIES {
                    pixelFormat: PIXEL_FORMAT,
                    dpiX: 96.,
                    dpiY: 96.,
                };
                // ID2D1RenderTarget::CreateBitmap : bitmap 1.0, comme avant la migration.
                let target: &ID2D1RenderTarget = &self.target;
                // SAFETY: buffer vérifié ci-dessus (largeur × hauteur × 4 octets, pitch = largeur × 4),
                // copié par Direct2D pendant l'appel.
                let bitmap = unsafe {
                    target.CreateBitmap(
                        D2D_SIZE_U {
                            width: image.width,
                            height: image.height,
                        },
                        Some(image.pixels.as_ptr().cast()),
                        image.width * 4,
                        &props,
                    )
                }
                .check("CreateBitmap")?;
                self.bitmaps.insert(key, (Arc::clone(image), bitmap));
            }
        }
        Ok(())
    }
    /// Bitmap préparé par `prepare_images`, étiré dans `rect`.
    pub(super) fn image(&self, rect: RectF, image: &Arc<Image>) {
        let bitmap = &self.bitmaps[&(Arc::as_ptr(image) as usize)].1;
        // ID2D1RenderTarget::DrawBitmap, interpolation linéaire, comme avant la migration.
        let target: &ID2D1RenderTarget = &self.target;
        // SAFETY: passe de dessin ouverte ; bitmap créé par ce contexte.
        unsafe {
            target.DrawBitmap(
                bitmap,
                Some(&rect_f(rect)),
                1.,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            )
        };
    }
}
