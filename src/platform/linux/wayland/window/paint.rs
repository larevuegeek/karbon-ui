//! Présentation d'une image, envoyée avec un rappel de trame : l'image suivante attend ce
//! rappel (rythme de l'écran ; aucune image pour une fenêtre cachée), sauf après une
//! nouvelle configuration, qui doit être suivie d'une image.
//!
//! Chemin par défaut : Vulkan (`linux/vulkan/`), image entière rendue dans la chaîne
//! d'échange de la surface ; le rappel de trame, l'échelle et la destination du viewport
//! sont demandés juste avant la présentation, dont l'envoi de la surface les applique.
//! Chemin Cairo (mode logiciel, sans Vulkan, ou après une erreur du GPU, qui désactive
//! Vulkan pour la fenêtre) : scène dessinée à l'échelle de la surface, copiée dans un
//! tampon `wl_shm` libre ; seules les zones modifiées sont redessinées, recopiées et
//! déclarées au compositeur.
use super::{Buffer, Kind, NEW, Ptr, Window, create, destroy, request};
use crate::{
    platform::linux::{
        bitmap::Bitmap,
        ffi::wayland::{wl_callback_interface, wl_region_interface},
        vulkan,
        wayland::protocol::{wl_compositor, wl_region, wl_surface, wp_viewport},
    },
    window::{RenderMode, WindowOptions},
};
use std::time::{Duration, Instant};

/// Tampons au plus par fenêtre (un affiché, un en attente, un en dessin).
const BUFFERS: usize = 3;
/// Délai avant l'image exacte qui suit des images approchées (image animée étirée).
const REFINE_DELAY: Duration = Duration::from_millis(150);
/// Délai après la dernière image avant d'alléger les ressources GPU (`Trim::Idle`).
const TRIM_DELAY: Duration = Duration::from_secs(1);

impl Window {
    /// Une image peut être envoyée maintenant : configuration reçue, image précédente
    /// affichée (ou nouvelle configuration à confirmer) et un tampon disponible.
    pub fn ready(&self) -> bool {
        self.configured
            && (self.frame.is_null() || self.acked)
            && (self.session.gpu.is_some()
                || self.buffers.len() < BUFFERS
                || self.buffers.iter().any(|b| !b.busy))
    }
    /// Prépare l'image et l'envoie au compositeur si elle a changé.
    pub fn paint(&mut self, compositor: Ptr, shm: Ptr) -> crate::Result<()> {
        if !self.ready() {
            return Ok(());
        }
        self.session.dirty = false;
        if self.suspended {
            // Fenêtre réduite ou masquée : ses ressources viennent d'être libérées, une
            // image les recréerait pour rien. La configuration est tout de même validée
            // par un envoi sans tampon.
            if self.acked {
                request!(self.surface, wl_surface::COMMIT);
                self.acked = false;
            }
            return Ok(());
        }
        let start = Instant::now();
        self.session.prepare()?;
        let prepared = start.elapsed();
        let (_, width, height, scale) = self.session.context();
        let key = (
            width,
            height,
            scale.to_bits(),
            self.session.runtime.stats().paints,
        );
        if self.session.presented == Some(key) && !self.acked {
            return Ok(());
        }
        if self.paint_vulkan(compositor, key, start, prepared)? {
            return Ok(());
        }
        if self
            .surface_pixels
            .as_ref()
            .is_none_or(|s| (s.width, s.height) != (width, height))
        {
            self.surface_pixels = Some(Bitmap::new(width, height)?);
        }
        let pixels = self.surface_pixels.as_ref().expect("surface created above");
        // Seules les zones modifiées depuis l'image précédente sont redessinées.
        let changed =
            self.session
                .renderer
                .draw_changes(pixels, self.session.runtime.scene(), scale)?;
        if self.session.renderer.take_refine() {
            // Image animée étirée : l'image exacte suit l'arrêt du mouvement.
            self.session.refine_at = Some(Instant::now() + REFINE_DELAY);
        }
        let drawn = start.elapsed();
        if !changed {
            self.session.presented = Some(key);
            if self.acked {
                // Configuration confirmée sans changement visible : elle est validée
                // par un envoi sans nouveau tampon.
                request!(self.surface, wl_surface::COMMIT);
                self.acked = false;
            }
            return Ok(());
        }
        let zones = self.session.renderer.last_region.clone();
        for buffer in &mut self.buffers {
            buffer.invalidate(zones.as_deref());
        }
        self.buffers
            .retain(|b| b.busy || (b.width, b.height) == (width, height));
        let index = match self.buffers.iter().position(|b| !b.busy) {
            Some(index) => index,
            None if self.buffers.len() < BUFFERS => {
                self.buffers.push(Buffer::new(shm, width, height)?);
                self.buffers.len() - 1
            }
            // Tous les tampons sont encore lus par le compositeur : image reprise au
            // prochain rendu d'un tampon.
            None => {
                self.session.dirty = true;
                return Ok(());
            }
        };
        let buffer = &mut self.buffers[index];
        buffer.fill(pixels.pixels());
        buffer.busy = true;
        let raw = buffer.raw;
        self.declare_regions(compositor);
        if self.scale120.is_some() && !self.viewport.is_null() {
            request!(self.surface, wl_surface::SET_BUFFER_SCALE, 1i32);
            request!(
                self.viewport,
                wp_viewport::SET_DESTINATION,
                self.session.width,
                self.session.height
            );
        } else {
            request!(
                self.surface,
                wl_surface::SET_BUFFER_SCALE,
                self.integer_scale.max(1)
            );
        }
        request!(self.surface, wl_surface::ATTACH, raw, 0i32, 0i32);
        let whole = [[0, 0, width as i32, height as i32]];
        for &[left, top, right, bottom] in zones.as_deref().unwrap_or(&whole) {
            request!(
                self.surface,
                wl_surface::DAMAGE_BUFFER,
                left,
                top,
                right - left,
                bottom - top
            );
        }
        if self.frame.is_null() {
            self.frame = create!(
                self.surface,
                wl_surface::FRAME,
                &raw const wl_callback_interface,
                Kind::Callback,
                NEW
            );
        }
        request!(self.surface, wl_surface::COMMIT);
        self.acked = false;
        self.session.presented = Some(key);
        if self.session.options.profile {
            let zone = zones.as_ref().map_or("complète".to_string(), |zones| {
                let area: i32 = zones.iter().map(|r| (r[2] - r[0]) * (r[3] - r[1])).sum();
                format!("{} rect., {} kpx", zones.len(), area / 1000)
            });
            eprintln!(
                "karbon-ui: image {width}×{height}, chemin Cairo (wl_shm), zone {zone} : préparation {:.1} ms, dessin {:.1} ms, copie et envoi {:.1} ms, total {:.1} ms",
                prepared.as_secs_f64() * 1e3,
                (drawn - prepared).as_secs_f64() * 1e3,
                (start.elapsed() - drawn).as_secs_f64() * 1e3,
                start.elapsed().as_secs_f64() * 1e3,
            );
        }
        self.session.frame_presented()
    }
    /// Présente l'image par Vulkan ; `false` : elle passe par Cairo (pas de chemin GPU, ou
    /// erreur du GPU, qui le désactive pour la fenêtre).
    fn paint_vulkan(
        &mut self,
        compositor: Ptr,
        key: (usize, usize, u32, u64),
        start: Instant,
        prepared: Duration,
    ) -> crate::Result<bool> {
        if self.session.gpu.is_none() {
            return Ok(false);
        }
        let (_, width, height, scale) = self.session.context();
        self.declare_regions(compositor);
        let (surface, viewport) = (self.surface, self.viewport);
        let fractional = self.scale120.is_some() && !viewport.is_null();
        let (logical, integer) = (
            (self.session.width, self.session.height),
            self.integer_scale.max(1),
        );
        let mut frame = self.frame;
        // Requêtes appliquées par l'envoi de la surface que fait la présentation.
        let before = || {
            if fractional {
                request!(surface, wl_surface::SET_BUFFER_SCALE, 1i32);
                request!(viewport, wp_viewport::SET_DESTINATION, logical.0, logical.1);
            } else {
                request!(surface, wl_surface::SET_BUFFER_SCALE, integer);
            }
            if frame.is_null() {
                frame = create!(
                    surface,
                    wl_surface::FRAME,
                    &raw const wl_callback_interface,
                    Kind::Callback,
                    NEW
                );
            }
        };
        let (gpu, target) = self.session.gpu.as_mut().expect("GPU path checked above");
        let presented = gpu.present(
            &mut self.session.renderer,
            self.session.runtime.scene(),
            target,
            (width, height),
            scale,
            before,
        );
        self.frame = frame;
        match presented {
            Ok(vulkan::Presented::Shown) => {}
            Ok(vulkan::Presented::Busy) => {
                // Aucune image de la chaîne libre : reprise au tour suivant de la boucle.
                self.session.dirty = true;
                return Ok(true);
            }
            Err(error) => {
                eprintln!("karbon-ui: Vulkan désactivé pour cette fenêtre : {error}");
                self.session.gpu = None;
                self.session.presented = None;
                return Ok(false);
            }
        }
        // Surface présentée par Vulkan : l'image Cairo suivante sera dessinée en entier,
        // ses tampons sont libérés une fois rendus par le compositeur.
        self.session.renderer.forget();
        self.surface_pixels = None;
        self.buffers.retain(|b| b.busy);
        if self.session.renderer.take_refine() {
            // Image animée étirée : l'image exacte suit l'arrêt du mouvement.
            self.session.refine_at = Some(Instant::now() + REFINE_DELAY);
        }
        self.acked = false;
        self.session.presented = Some(key);
        self.session.trim_at = Some(Instant::now() + TRIM_DELAY);
        if self.session.options.profile
            && let Some((gpu, target)) = &self.session.gpu
        {
            let [translated, recorded, presented, previous] = gpu.timings;
            eprintln!(
                "karbon-ui: image {width}×{height}, chemin Vulkan ({}) : préparation {:.1} ms, traduction {translated:.1} ms, enregistrement {recorded:.1} ms, soumission et présentation {presented:.1} ms, attente de l'image précédente {previous:.1} ms, total {:.1} ms",
                target.mode(),
                prepared.as_secs_f64() * 1e3,
                start.elapsed().as_secs_f64() * 1e3,
            );
        }
        self.session.frame_presented()?;
        Ok(true)
    }
    /// Fenêtre réduite ou masquée (`suspended`) : ressources GPU, surface et tampons Cairo
    /// libérés (le compositeur garde l'image affichée) ; la prochaine image les recrée.
    pub(super) fn hidden(&mut self) {
        if let Some((gpu, _)) = &mut self.session.gpu {
            gpu.trim(vulkan::Trim::Hidden);
        }
        self.session.trim_at = None;
        self.session.renderer.forget();
        self.surface_pixels = None;
        self.buffers.retain(|b| b.busy);
    }
    /// Régions de la surface, déclarées quand la taille change : opaque (fond du thème)
    /// pour que le compositeur ne mélange pas ce qui est dessous, et entrée vide pour une
    /// fenêtre traversée par les clics.
    fn declare_regions(&mut self, compositor: Ptr) {
        let size = (self.session.width, self.session.height);
        if self.regions == Some(size) {
            return;
        }
        self.regions = Some(size);
        let region = |whole: bool| -> Ptr {
            let region = create!(
                compositor,
                wl_compositor::CREATE_REGION,
                &raw const wl_region_interface,
                Kind::Silent,
                NEW
            );
            if whole {
                request!(region, wl_region::ADD, 0i32, 0i32, size.0, size.1);
            }
            region
        };
        if !self.session.options.transparent {
            let opaque = region(true);
            request!(self.surface, wl_surface::SET_OPAQUE_REGION, opaque);
            destroy!(opaque, wl_region::DESTROY);
        }
        if self.session.options.transparent && self.session.options.click_through {
            let empty = region(false);
            request!(self.surface, wl_surface::SET_INPUT_REGION, empty);
            destroy!(empty, wl_region::DESTROY);
        }
    }
}
/// Chemin GPU de la fenêtre : Vulkan sur le contexte partagé `gpu` (absent en mode
/// logiciel ou sans GPU Vulkan : Cairo sert alors toutes les images), si le GPU peut
/// présenter sur la surface.
pub(super) fn gpu(
    options: &WindowOptions,
    gpu: Option<&std::rc::Rc<vulkan::Gpu>>,
    display: Ptr,
    surface: Ptr,
) -> Option<(vulkan::Renderer, vulkan::Surface)> {
    let gpu = gpu?;
    let created = vulkan::Renderer::on(gpu).and_then(|renderer| {
        let native = vulkan::Native::Wayland { display, surface };
        let target = renderer.surface(native, options.transparent)?;
        Ok((renderer, target))
    });
    match created {
        Ok((renderer, target)) => {
            if options.profile {
                eprintln!(
                    "karbon-ui: Vulkan sur {}, présentation {}",
                    renderer.device_name(),
                    target.mode()
                );
            }
            Some((renderer, target))
        }
        Err(error) => {
            if options.renderer == RenderMode::Gpu || options.profile {
                eprintln!("karbon-ui: Vulkan indisponible, rendu Cairo : {error}");
            }
            None
        }
    }
}
/// Contexte Vulkan partagé par les fenêtres de la boucle, créé à la première fenêtre qui
/// le demande (une seule tentative) ; `None` en mode logiciel ou sans GPU Vulkan.
pub(in crate::platform::linux::wayland) fn shared_gpu(
    slot: &mut Option<Option<std::rc::Rc<vulkan::Gpu>>>,
    options: &WindowOptions,
) -> Option<std::rc::Rc<vulkan::Gpu>> {
    if options.renderer == RenderMode::Software {
        return None;
    }
    slot.get_or_insert_with(|| match vulkan::Gpu::new(false) {
        Ok(gpu) => Some(std::rc::Rc::new(gpu)),
        Err(error) => {
            if options.renderer == RenderMode::Gpu || options.profile {
                eprintln!("karbon-ui: Vulkan indisponible, rendu Cairo : {error}");
            }
            None
        }
    })
    .clone()
}
