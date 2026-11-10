pub(super) use super::super::{bitmap::Bitmap, fonts::Fonts};
pub(super) use super::cache::Caches;
pub(super) use super::canvas::Canvas;
use super::*;
pub(super) use crate::platform::shared::damage;
pub(super) use crate::ui::{
    font::FontAsset,
    rendering::transform::Affine,
    scene::{DrawCommand, Scene},
};
pub(super) use objc2_core_graphics::{CGBlendMode, CGContext};
pub(super) use std::sync::Arc;

pub(crate) struct Renderer {
    pub(super) fonts: Fonts,
    pub(super) text: text::Cache,
    pub(super) caches: Caches,
    /// Inventaire de la dernière image peinte par `draw_changes`.
    pub(super) previous: Option<damage::Damage>,
    /// Inventaire de la dernière scène examinée par `full_change`.
    pub(super) examined: Option<damage::Damage>,
    /// `examined` décrit la scène que `draw_changes` va dessiner.
    pub(super) fresh: bool,
    /// Zone redessinée par le dernier `draw_changes` (`None` : toute la surface).
    pub(crate) last_region: Option<Vec<damage::Pixels>>,
}
impl Renderer {
    /// Libellé du moteur, transmis à `ViewContext::backend`.
    pub(crate) const LABEL: &'static str = "CPU · Core Graphics";
    pub(crate) fn new() -> crate::Result<Self> {
        Ok(Self {
            fonts: Fonts::default(),
            text: text::Cache::default(),
            caches: Caches::default(),
            previous: None,
            examined: None,
            fresh: false,
            last_region: None,
        })
    }
    pub(crate) fn load_fonts(&mut self, fonts: &[FontAsset]) -> crate::Result<()> {
        self.fonts.load(fonts)?;
        self.text = text::Cache::default();
        Ok(())
    }
    /// `true` si `scene` change toute la surface depuis la scène examinée par l'appel
    /// précédent (quelle que soit la façon dont celle-ci a été dessinée).
    pub(crate) fn full_change(
        &mut self,
        scene: &Scene,
        scale: f32,
        size: (usize, usize),
    ) -> crate::Result<bool> {
        let current = damage::Damage::new(scene, scale, size, &mut self.fonts, &mut self.text)?;
        let full = self
            .examined
            .as_ref()
            .is_none_or(|e| matches!(current.change(e), damage::Change::Full));
        // Réutilisé par `draw_changes` pour la même scène.
        self.examined = Some(current);
        self.fresh = true;
        Ok(full)
    }
    /// Une image approchée (animation en cours) a été dessinée ; la zone suivante doit
    /// être redessinée exactement. Remis à zéro par l'appel.
    pub(crate) fn take_refine(&mut self) -> bool {
        let refine = std::mem::take(&mut self.caches.refine);
        if refine {
            self.previous = None;
        }
        refine
    }
    /// Rastérise `scene` à `scale` dans une surface de `width` × `height` pixels.
    pub(crate) fn render(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid render size"));
        }
        if !(scale.is_finite() && scale > 0.) {
            return Err(crate::Error::invalid_input("invalid render scale"));
        }
        let target = Bitmap::new(width as usize, height as usize)?;
        self.draw(&target, scene, scale)?;
        Ok(target.into_pixels())
    }
    /// Dessine `scene` sur `target` : fond, puis commandes dans l'ordre.
    pub(crate) fn draw(&mut self, target: &Bitmap, scene: &Scene, scale: f32) -> crate::Result<()> {
        self.previous = None;
        self.paint(target, scene, scale, None)
    }
    /// Met à jour `target`, qui contient l'image précédente de ce renderer, en ne
    /// redessinant que la zone modifiée ; `false` si rien n'a changé.
    pub(crate) fn draw_changes(
        &mut self,
        target: &Bitmap,
        scene: &Scene,
        scale: f32,
    ) -> crate::Result<bool> {
        scene.validate()?;
        let current = match self
            .examined
            .as_ref()
            .filter(|_| std::mem::take(&mut self.fresh))
        {
            Some(examined) => examined.clone(),
            None => damage::Damage::new(
                scene,
                scale,
                (target.width, target.height),
                &mut self.fonts,
                &mut self.text,
            )?,
        };
        let change = match self.previous.take() {
            Some(previous) => current.change(&previous),
            None => damage::Change::Full,
        };
        let region = match change {
            damage::Change::None => {
                self.previous = Some(current);
                return Ok(false);
            }
            damage::Change::Full => None,
            damage::Change::Region(region) => Some(region),
        };
        self.paint(target, scene, scale, region.as_deref())?;
        self.last_region = region;
        self.previous = Some(current);
        Ok(true)
    }
    /// Fond puis commandes, limités à `region` (pixels, y vers le bas) si elle est donnée.
    pub(super) fn paint(
        &mut self,
        target: &Bitmap,
        scene: &Scene,
        scale: f32,
        region: Option<&[damage::Pixels]>,
    ) -> crate::Result<()> {
        scene.validate()?;
        self.prepare_images(scene)?;
        self.caches.begin();
        let c = Some(&*target.context);
        CGContext::save_g_state(c);
        if let Some(zones) = region {
            // Repère natif (y vers le haut) : la découpe vaut pour toute la peinture.
            let rects = zones
                .iter()
                .map(|&[left, top, right, bottom]| {
                    objc2_core_foundation::CGRect::new(
                        objc2_core_foundation::CGPoint::new(
                            f64::from(left),
                            target.height as f64 - f64::from(bottom),
                        ),
                        objc2_core_foundation::CGSize::new(
                            f64::from(right - left),
                            f64::from(bottom - top),
                        ),
                    )
                })
                .collect::<Vec<_>>();
            if let Some(first) = std::ptr::NonNull::new(rects.as_ptr().cast_mut()) {
                // SAFETY : `rects` contient `rects.len()` rectangles, lus pendant l'appel.
                unsafe { CGContext::clip_to_rects(c, first, rects.len()) };
            }
        }
        CGContext::save_g_state(c);
        CGContext::set_blend_mode(c, CGBlendMode::Copy);
        CGContext::set_fill_color_with_color(c, Some(&super::super::ffi::cg_color(scene.clear)));
        CGContext::fill_rect(
            c,
            super::super::ffi::rect(crate::ui::scene::RectF::new(
                0.,
                0.,
                target.width as f32,
                target.height as f32,
            )),
        );
        CGContext::restore_g_state(c);
        CGContext::save_g_state(c);
        let scaling = Affine {
            m11: scale,
            m22: scale,
            ..Affine::IDENTITY
        };
        let result = {
            let mut canvas = Canvas::new(
                &target.context,
                target.width,
                target.height,
                Some(target),
                scaling,
                &mut self.fonts,
                &mut self.text,
                &mut self.caches,
            );
            let result = canvas.run(&scene.commands);
            canvas.finish();
            result
        };
        CGContext::restore_g_state(c);
        CGContext::restore_g_state(c);
        CGContext::flush(c);
        self.caches.end();
        result
    }
    /// Convertit les images de la scène ; celles qui n'y sont plus sont libérées.
    pub(super) fn prepare_images(&mut self, scene: &Scene) -> crate::Result<()> {
        let used = scene
            .commands
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Image { image, .. } => Some(image),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.caches
            .images
            .retain(|key, _| used.iter().any(|i| Arc::as_ptr(i) as usize == *key));
        for image in used {
            let key = Arc::as_ptr(image) as usize;
            if self.caches.images.contains_key(&key) {
                continue;
            }
            if image.width == 0 || image.height == 0 || image.width > 16384 || image.height > 16384
            {
                return Err(crate::Error::invalid_input("invalid image buffer"));
            }
            let native = super::super::bitmap::image(image)?;
            self.caches.images.insert(key, (Arc::clone(image), native));
        }
        Ok(())
    }
}
