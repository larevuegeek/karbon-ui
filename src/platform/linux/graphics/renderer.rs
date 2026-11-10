pub(super) use super::super::{
    bitmap::{Bitmap, ImageSurface},
    ffi::{cairo::OPERATOR_SOURCE, failure},
    fonts::Fonts,
};
pub(super) use super::cache::Caches;
pub(super) use super::canvas::Canvas;
use super::*;
pub(super) use crate::platform::shared::damage;
pub(super) use crate::ui::{
    font::FontAsset,
    rendering::transform::Affine,
    scene::{DrawCommand, Scene},
};
pub(super) use std::{rc::Rc, sync::Arc};

pub(crate) struct Renderer {
    pub(super) fonts: Fonts,
    pub(super) text: text::Cache,
    pub(super) caches: Caches,
    /// Inventaire de la dernière image peinte par `draw_changes`.
    pub(super) previous: Option<damage::Damage>,
    /// Zones redessinées par le dernier `draw_changes` (`None` : toute la surface).
    pub(crate) last_region: Option<Vec<damage::Pixels>>,
}
impl Renderer {
    /// Libellé du moteur, transmis à `ViewContext::backend`.
    pub(crate) const LABEL: &'static str = "CPU · Cairo";
    pub(crate) fn new() -> crate::Result<Self> {
        Ok(Self {
            fonts: Fonts::new()?,
            text: text::Cache::default(),
            caches: Caches::default(),
            previous: None,
            last_region: None,
        })
    }
    /// Fenêtre : `system-ui` et `monospace` désignent les polices du bureau (aperçus et
    /// exports gardent les génériques de fontconfig, reproductibles).
    pub(crate) fn follow_desktop(&mut self) {
        self.fonts.follow_desktop();
        self.text = text::Cache::default();
    }
    pub(crate) fn load_fonts(&mut self, fonts: &[FontAsset]) -> crate::Result<()> {
        self.fonts.load(fonts)?;
        self.text = text::Cache::default();
        Ok(())
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
    /// redessinant que les zones modifiées ; `false` si rien n'a changé.
    pub(crate) fn draw_changes(
        &mut self,
        target: &Bitmap,
        scene: &Scene,
        scale: f32,
    ) -> crate::Result<bool> {
        scene.validate()?;
        let current = damage::Damage::new(
            scene,
            scale,
            (target.width, target.height),
            &mut self.fonts,
            &mut self.text,
        )?;
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
    /// Une image approchée (image animée étirée) a été dessinée ; la suivante doit être
    /// dessinée entière, exactement. Remis à zéro par l'appel.
    pub(crate) fn take_refine(&mut self) -> bool {
        let refine = std::mem::take(&mut self.caches.refine);
        if refine {
            self.previous = None;
        }
        refine
    }
    /// Fond puis commandes, limités à `region` (pixels de la cible) si elle est donnée.
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
        let context = target.context()?;
        if let Some(zones) = region {
            // Découpe en pixels de la cible, valable pour toute la peinture.
            for &[left, top, right, bottom] in zones {
                context.rectangle(
                    f64::from(left),
                    f64::from(top),
                    f64::from(right - left),
                    f64::from(bottom - top),
                );
            }
            context.clip();
        }
        context.save();
        context.set_operator(OPERATOR_SOURCE);
        let [r, g, b, a] = text::rgba(scene.clear, 1.);
        context.set_source_rgba(r, g, b, a);
        context.paint();
        context.restore();
        let scaling = Affine {
            m11: scale,
            m22: scale,
            ..Affine::IDENTITY
        };
        let result = {
            let mut canvas = Canvas::new(
                &context,
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
        self.caches.end();
        result?;
        if context.status() != crate::platform::linux::ffi::cairo::STATUS_SUCCESS {
            return Err(failure("cairo_draw"));
        }
        Ok(())
    }
    /// Surfaces des images de la scène ; celles qui n'y sont plus sont libérées.
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
            self.caches
                .images
                .insert(key, Rc::new(ImageSurface::new(image)?));
        }
        Ok(())
    }
}
