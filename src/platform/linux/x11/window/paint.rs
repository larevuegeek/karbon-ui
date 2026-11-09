//! Présentation d'une image X11. Chemin par défaut : Vulkan (`linux/vulkan/`), image entière
//! dans la chaîne d'échange de la fenêtre (« fifo » : rythme du balayage de l'écran, X11
//! n'ayant pas de rappel de trame). Chemin Cairo (mode logiciel, sans Vulkan, ou après une
//! erreur du GPU, qui désactive Vulkan pour la fenêtre) : seules les zones modifiées sont
//! redessinées puis envoyées par `xcb_put_image`, en bandes sous la taille maximale d'une
//! requête.
use super::{super::connection::Connection, Window};
use crate::{
    platform::linux::{bitmap::Bitmap, ffi::xcb::*, vulkan},
    window::{RenderMode, WindowOptions},
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

/// Délai avant l'image exacte qui suit des images approchées (image animée étirée).
const REFINE_DELAY: Duration = Duration::from_millis(150);
/// Délai après la dernière image avant d'alléger les ressources GPU (`Trim::Idle`).
const TRIM_DELAY: Duration = Duration::from_secs(1);

impl Window {
    /// La fenêtre peut recevoir une image (affichée et exposée).
    pub fn ready(&self) -> bool {
        self.mapped && self.exposed
    }
    /// Prépare l'image et l'envoie si elle a changé.
    pub fn paint(&mut self, connection: &Connection) -> crate::Result<()> {
        if !self.ready() {
            return Ok(());
        }
        self.session.dirty = false;
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
        if self.session.presented == Some(key) {
            return Ok(());
        }
        if self.paint_vulkan(key, start, prepared)? {
            return Ok(());
        }
        if self
            .surface_pixels
            .as_ref()
            .is_none_or(|s| (s.width, s.height) != (width, height))
        {
            self.surface_pixels = Some(Bitmap::new(width, height)?);
            self.session.renderer.forget();
        }
        let pixels = self.surface_pixels.as_ref().expect("surface created above");
        let changed =
            self.session
                .renderer
                .draw_changes(pixels, self.session.runtime.scene(), scale)?;
        if self.session.renderer.take_refine() {
            self.session.refine_at = Some(Instant::now() + REFINE_DELAY);
        }
        let drawn = start.elapsed();
        self.session.presented = Some(key);
        if !changed {
            return Ok(());
        }
        let zones = self.session.renderer.last_region.clone();
        let whole = [[0, 0, width as i32, height as i32]];
        for &zone in zones.as_deref().unwrap_or(&whole) {
            self.put(connection, pixels, zone);
        }
        connection.flush();
        if self.session.options.profile {
            let zone = zones.as_ref().map_or("complète".to_string(), |zones| {
                let area: i32 = zones.iter().map(|r| (r[2] - r[0]) * (r[3] - r[1])).sum();
                format!("{} rect., {} kpx", zones.len(), area / 1000)
            });
            eprintln!(
                "karbon-ui: image {width}×{height}, chemin Cairo (X11), zone {zone} : préparation {:.1} ms, dessin {:.1} ms, envoi {:.1} ms, total {:.1} ms",
                prepared.as_secs_f64() * 1e3,
                (drawn - prepared).as_secs_f64() * 1e3,
                (start.elapsed() - drawn).as_secs_f64() * 1e3,
                start.elapsed().as_secs_f64() * 1e3,
            );
        }
        self.session.frame_presented()
    }
    /// Envoie la zone `[left, top, right, bottom]` de `pixels` (BGRA, format Z du serveur),
    /// en bandes de lignes sous la taille maximale d'une requête.
    fn put(&self, connection: &Connection, pixels: &Bitmap, [left, top, right, bottom]: [i32; 4]) {
        let (left, top) = (left.max(0) as usize, top.max(0) as usize);
        let right = (right.max(0) as usize).min(pixels.width);
        let bottom = (bottom.max(0) as usize).min(pixels.height);
        if right <= left || bottom <= top {
            return;
        }
        let row = (right - left) * 4;
        // SAFETY : connexion valide.
        let limit = unsafe { (connection.xcb.xcb_get_maximum_request_length)(connection.raw) };
        // Unités de 4 octets, en-tête de la requête (24 octets) déduit.
        let budget = (limit as usize * 4).saturating_sub(64).min(16 << 20);
        let rows = (budget / row).max(1);
        let (data, stride) = (pixels.pixels(), pixels.row_bytes());
        let mut band = Vec::with_capacity(rows.min(bottom - top) * row);
        let mut y = top;
        while y < bottom {
            let count = rows.min(bottom - y);
            band.clear();
            for line in y..y + count {
                let start = line * stride + left * 4;
                band.extend_from_slice(&data[start..start + row]);
            }
            // SAFETY : `band` contient `count` lignes de `right - left` pixels ; fenêtre et
            // contexte graphique valides.
            unsafe {
                (connection.xcb.xcb_put_image)(
                    connection.raw,
                    XCB_IMAGE_FORMAT_Z_PIXMAP,
                    self.window,
                    self.gc,
                    (right - left) as u16,
                    count as u16,
                    left as i16,
                    y as i16,
                    0,
                    self.depth,
                    band.len() as u32,
                    band.as_ptr(),
                )
            };
            y += count;
        }
    }
    /// Présente l'image par Vulkan ; `false` : elle passe par Cairo (pas de chemin GPU, ou
    /// erreur du GPU, qui le désactive pour la fenêtre).
    fn paint_vulkan(
        &mut self,
        key: (usize, usize, u32, u64),
        start: Instant,
        prepared: Duration,
    ) -> crate::Result<bool> {
        let (_, width, height, scale) = self.session.context();
        let session = &mut self.session;
        let Some((gpu, target)) = session.gpu.as_mut() else {
            return Ok(false);
        };
        let presented = gpu.present(
            &mut session.renderer,
            session.runtime.scene(),
            target,
            (width, height),
            scale,
            || {},
        );
        match presented {
            Ok(vulkan::Presented::Shown) => {}
            Ok(vulkan::Presented::Busy) => {
                session.dirty = true;
                return Ok(true);
            }
            Err(error) => {
                eprintln!("karbon-ui: Vulkan désactivé pour cette fenêtre : {error}");
                session.gpu = None;
                session.presented = None;
                return Ok(false);
            }
        }
        // Fenêtre présentée par Vulkan : l'image Cairo suivante sera dessinée en entier.
        session.renderer.forget();
        self.surface_pixels = None;
        if session.renderer.take_refine() {
            session.refine_at = Some(Instant::now() + REFINE_DELAY);
        }
        session.presented = Some(key);
        session.trim_at = Some(Instant::now() + TRIM_DELAY);
        if session.options.profile
            && let Some((gpu, target)) = &session.gpu
        {
            let [translated, recorded, presented, previous] = gpu.timings;
            eprintln!(
                "karbon-ui: image {width}×{height}, chemin Vulkan ({}, X11) : préparation {:.1} ms, traduction {translated:.1} ms, enregistrement {recorded:.1} ms, soumission et présentation {presented:.1} ms, attente de l'image précédente {previous:.1} ms, total {:.1} ms",
                target.mode(),
                prepared.as_secs_f64() * 1e3,
                start.elapsed().as_secs_f64() * 1e3,
            );
        }
        self.session.frame_presented()?;
        Ok(true)
    }
    /// Fenêtre réduite (`_NET_WM_STATE_HIDDEN`) ou retirée de l'écran : ressources GPU et
    /// image Cairo libérées ; la prochaine image les recrée.
    pub fn hidden(&mut self) {
        if let Some((gpu, _)) = &mut self.session.gpu {
            gpu.trim(vulkan::Trim::Hidden);
        }
        self.session.trim_at = None;
        self.session.renderer.forget();
        self.surface_pixels = None;
    }
}
/// Chemin GPU de la fenêtre `window` : Vulkan sur le contexte partagé `gpu` (absent en mode
/// logiciel ou sans GPU Vulkan : Cairo sert alors toutes les images), si le GPU peut
/// présenter sur la fenêtre.
pub(super) fn gpu(
    options: &WindowOptions,
    gpu: Option<&Rc<vulkan::Gpu>>,
    connection: &Connection,
    window: u32,
) -> Option<(vulkan::Renderer, vulkan::Surface)> {
    let gpu = gpu?;
    let created = vulkan::Renderer::on(gpu).and_then(|renderer| {
        let native = vulkan::Native::Xcb {
            connection: connection.as_ptr(),
            window,
        };
        let target = renderer.surface(native, options.transparent)?;
        Ok((renderer, target))
    });
    match created {
        Ok((renderer, target)) => {
            if options.profile {
                eprintln!(
                    "karbon-ui: Vulkan sur {} (X11), présentation {}",
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
/// Contexte Vulkan partagé par les fenêtres de la boucle (voir le backend Wayland).
pub(in crate::platform::linux::x11) fn shared_gpu(
    slot: &mut Option<Option<Rc<vulkan::Gpu>>>,
    options: &WindowOptions,
) -> Option<Rc<vulkan::Gpu>> {
    if options.renderer == RenderMode::Software {
        return None;
    }
    slot.get_or_insert_with(|| match vulkan::Gpu::new(false) {
        Ok(gpu) => Some(Rc::new(gpu)),
        Err(error) => {
            if options.renderer == RenderMode::Gpu || options.profile {
                eprintln!("karbon-ui: Vulkan indisponible, rendu Cairo : {error}");
            }
            None
        }
    })
    .clone()
}
