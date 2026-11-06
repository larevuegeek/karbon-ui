//! `backdrop-filter` : copie de la cible sous une forme, chaîne d'effets et repeinte.
use super::*;

impl Canvas {
    /// `backdrop-filter` : copie la cible déjà dessinée sous `rect` (marge de flou
    /// comprise), applique la chaîne d'effets et repeint le résultat dans la forme. Sans
    /// cible bitmap (liste de commandes d'un filtre), rien n'est fait.
    pub(super) fn backdrop(
        &self,
        rect: RectF,
        radii: [f32; 4],
        filters: crate::ui::rendering::filter::Filters,
    ) -> crate::Result<()> {
        let t = &self.target;
        // SAFETY: passe de dessin ouverte ; sorties valides pendant les appels.
        let (target, transform, dpi) = unsafe {
            let target = t.GetTarget().ok();
            let mut native = matrix(Affine::IDENTITY);
            t.GetTransform(&mut native);
            let (mut dpi, mut _y) = (96., 96.);
            t.GetDpi(&mut dpi, &mut _y);
            (target, affine(native), dpi)
        };
        let Some(bitmap) = target.and_then(|image| image.cast::<ID2D1Bitmap1>().ok()) else {
            return Ok(());
        };
        // SAFETY: passe de dessin ouverte ; exécute les commandes déjà émises avant la copie.
        unsafe { t.Flush(None, None) }.check("Flush (backdrop)")?;
        let scale = dpi / 96.;
        // SAFETY: bitmap cible valide.
        let size = unsafe { bitmap.GetPixelSize() };
        let blur = filters
            .values
            .iter()
            .map(|f| match f {
                crate::ui::rendering::filter::Filter::Blur(n) => *n,
                _ => 0.,
            })
            .sum::<f32>()
            * filters.scale;
        let margin = (blur * 3.).ceil();
        let device = transform.bounds(RectF {
            left: rect.left - margin,
            top: rect.top - margin,
            right: rect.right + margin,
            bottom: rect.bottom + margin,
        });
        let left = (device.left * scale).floor().clamp(0., size.width as f32) as u32;
        let top = (device.top * scale).floor().clamp(0., size.height as f32) as u32;
        let right = (device.right * scale).ceil().clamp(0., size.width as f32) as u32;
        let bottom = (device.bottom * scale).ceil().clamp(0., size.height as f32) as u32;
        if right <= left || bottom <= top {
            return Ok(());
        }
        let properties = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: PIXEL_FORMAT,
            dpiX: dpi,
            dpiY: dpi,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: std::mem::ManuallyDrop::new(None),
        };
        // SAFETY: contexte valide ; la copie lit la cible après les commandes déjà émises.
        let copy = unsafe {
            let copy = t
                .CreateBitmap(
                    D2D_SIZE_U {
                        width: right - left,
                        height: bottom - top,
                    },
                    None,
                    0,
                    &properties,
                )
                .check("CreateBitmap (backdrop)")?;
            copy.CopyFromBitmap(
                Some(&D2D_POINT_2U { x: 0, y: 0 }),
                &bitmap,
                Some(&D2D_RECT_U {
                    left,
                    top,
                    right,
                    bottom,
                }),
            )
            .check("CopyFromBitmap (backdrop)")?;
            copy
        };
        let image = filters::effect_chain(t, copy.into(), filters)?;
        // Pinceau image : repère du bitmap (DIP de l'appareil) vers le repère local.
        let Some(inverse) = transform.inverse() else {
            return Ok(());
        };
        let place = Affine::translation(left as f32 / scale, top as f32 / scale).then(inverse);
        let brush_properties = D2D1_BRUSH_PROPERTIES {
            opacity: 1.,
            transform: matrix(place),
        };
        let image_properties = D2D1_IMAGE_BRUSH_PROPERTIES {
            sourceRectangle: D2D_RECT_F {
                left: 0.,
                top: 0.,
                right: (right - left) as f32 / scale,
                bottom: (bottom - top) as f32 / scale,
            },
            extendModeX: D2D1_EXTEND_MODE_CLAMP,
            extendModeY: D2D1_EXTEND_MODE_CLAMP,
            interpolationMode: D2D1_INTERPOLATION_MODE_LINEAR,
        };
        // SAFETY: contexte valide ; propriétés lues pendant l'appel.
        let brush =
            unsafe { t.CreateImageBrush(&image, &image_properties, Some(&brush_properties)) }
                .check("CreateImageBrush (backdrop)")?;
        self.shapes.draw(t, &brush.into(), rect, radii, None);
        Ok(())
    }
}
