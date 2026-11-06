//! Image d'une fenêtre : la scène est traduite (le renderer Cairo de la fenêtre met le
//! texte en page et prépare les images), une image de la chaîne d'échange est acquise,
//! rendue, puis présentée, et la main est rendue sans attendre le GPU : il termine l'image
//! pendant que la boucle traite les événements suivants. La fin de cette image en vol est
//! attendue au début de l'image suivante (`finish`), avant que tampons, réserves et
//! ressources ne soient réutilisés ou libérés.
use super::{
    Cairo, Renderer,
    commands::{ATTACHMENT, transition},
    encode::Built,
    surface::{Native, Surface},
};
use crate::{platform::linux::ffi::vulkan::*, ui::scene::Scene};
use std::time::Instant;

/// Issue d'une image de fenêtre.
pub(in crate::platform::linux) enum Presented {
    Shown,
    /// Aucune image de la chaîne ne s'est libérée à temps : l'image est à refaire.
    Busy,
}
fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}
impl Renderer {
    /// Surface Vulkan de la fenêtre `native`.
    pub(in crate::platform::linux) fn surface(
        &self,
        native: Native,
        transparent: bool,
    ) -> crate::Result<Surface> {
        Surface::new(&self.gpu, native, transparent)
    }
    /// Dessine `scene` à `scale` sur une image de `size` pixels de `surface` et la
    /// présente ; `before` est appelé juste avant la présentation (requêtes de la surface
    /// appliquées avec l'image : rappel de trame, échelle, viewport). Une erreur laisse la
    /// surface inutilisable : la fenêtre passe alors à Cairo.
    pub(in crate::platform::linux) fn present(
        &mut self,
        cairo: &mut Cairo,
        scene: &Scene,
        surface: &mut Surface,
        size: (usize, usize),
        scale: f32,
        before: impl FnOnce(),
    ) -> crate::Result<Presented> {
        let previous = self.finish()?;
        let start = Instant::now();
        let result = self
            .build(cairo, scene, size, scale)
            .and_then(|mut built| self.show(&mut built, surface, size, before, start));
        // Image soumise : ses ressources sont refermées une fois le GPU passé (`finish`).
        if self.in_flight.is_none() {
            self.resources.end();
        }
        self.timings[3] = previous;
        result
    }
    /// Attend la fin de l'image en vol, s'il y en a une : ses textures de couches et de
    /// filtres retournent aux réserves et ses ressources sont refermées. Durée de l'attente
    /// en millisecondes.
    pub(super) fn finish(&mut self) -> crate::Result<f64> {
        let Some((images, mut built)) = self.in_flight.take() else {
            return Ok(0.);
        };
        let start = Instant::now();
        if let Err(error) = self.commands.wait() {
            // Le GPU n'a pas fini (délai dépassé, périphérique perdu) : rien n'est recyclé
            // ni libéré, et l'image reste en vol jusqu'à la destruction du renderer, qui
            // attend l'appareil.
            self.in_flight = Some((images, built));
            return Err(error);
        }
        self.recycle(images, &mut built);
        self.resources.end();
        Ok(ms(start.elapsed()))
    }
    fn show(
        &mut self,
        built: &mut Built,
        surface: &mut Surface,
        size: (usize, usize),
        before: impl FnOnce(),
        start: Instant,
    ) -> crate::Result<Presented> {
        let translated = start.elapsed();
        let Some(index) = surface.acquire(size)? else {
            return Ok(Presented::Busy);
        };
        self.commands.begin()?;
        // Image entièrement effacée par son passage : son contenu précédent est ignoré.
        surface.images[index].layout = VK_IMAGE_LAYOUT_UNDEFINED;
        let images = match self.encode(built, &mut surface.images[index]) {
            Ok(images) => images,
            Err(error) => {
                // Rien n'est soumis : les copies enregistrées n'auront pas lieu.
                self.resources.reset();
                return Err(error);
            }
        };
        transition(
            &self.gpu,
            self.commands.buffer,
            &mut surface.images[index],
            VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
            ATTACHMENT,
            // Avant le signal du sémaphore attendu par la présentation (toutes étapes).
            (VK_PIPELINE_STAGE_2_ALL_COMMANDS_BIT, VK_ACCESS_2_NONE),
        );
        let recorded = start.elapsed();
        self.commands
            .submit(Some((surface.acquired, surface.rendered[index])))?;
        // En vol jusqu'à l'image suivante, même si la présentation échoue.
        self.in_flight = Some((images, std::mem::take(built)));
        before();
        let shown = surface.present(index);
        self.timings[..3].copy_from_slice(&[
            ms(translated),
            ms(recorded - translated),
            ms(start.elapsed() - recorded),
        ]);
        shown?;
        Ok(Presented::Shown)
    }
}
