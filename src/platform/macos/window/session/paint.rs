//! Présentation d'une image : chemin Metal (`CAMetalLayer`, image entière) quand la scène
//! le permet, sinon chemin Core Graphics (redessin partiel, demi-résolution pendant les
//! mouvements) dont l'image devient le contenu du calque de la vue. Le passage d'un chemin
//! à l'autre change le calque visible et son contenu dans la même transaction.
use super::{Bitmap, Instant, Session, View, metal};
use crate::window::WindowOptions;
use objc2_quartz_core::CATransaction;

/// Délai après la dernière image avant d'alléger les ressources GPU (`metal::Trim::Idle`).
const TRIM_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

/// Chemin de dessin d'une image.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Path {
    /// Aucune image encore : le chemin GPU est attendu s'il existe.
    None,
    Metal,
    CoreGraphics,
}
impl Session {
    /// Prépare l'image et la présente comme contenu du calque de la vue.
    pub fn paint(&mut self) -> crate::Result<()> {
        self.dirty = false;
        // Fenêtre dans le Dock ou entièrement couverte : rien n'est dessiné (les
        // ressources libérées par `hidden` ne sont pas recréées) ; l'image est refaite à
        // la sortie du Dock ou quand la fenêtre redevient visible (M5 de la revue).
        if self.window.isMiniaturized() || self.covered() {
            self.presented = None;
            return Ok(());
        }
        let start = Instant::now();
        self.prepare()?;
        let prepared = start.elapsed();
        let (_, width, height, scale) = self.context();
        let key = (width, height, scale.to_bits(), self.runtime.stats().paints);
        if self.presented == Some(key) {
            return Ok(());
        }
        if self.paint_metal(key, start, prepared)? {
            return Ok(());
        }
        let now = Instant::now();
        // Mouvement qui change toute la fenêtre (repli d'un panneau, changement de page,
        // défilement) : image en demi-résolution, agrandie par Core Animation ; l'image
        // nette suit l'arrêt du mouvement. Les petites zones restent redessinées nettes.
        let moving = self
            .runtime
            .next_wake()
            .is_some_and(|wake| wake <= now + std::time::Duration::from_millis(40))
            || self.scrolling_until.is_some_and(|until| until > now);
        let low = (scale / 2.).max(1.);
        // Inventaire tenu à chaque image, pour juger la suivante.
        let full = self
            .renderer
            .full_change(self.runtime.scene(), scale, (width, height))?;
        let motion = moving && low < scale && full;
        let (image, shown) = if motion {
            let pixels = |v: usize| ((v as f32 * low / scale).round() as usize).max(1);
            let (w, h) = (pixels(width), pixels(height));
            if self
                .motion
                .as_ref()
                .is_none_or(|s| (s.width, s.height) != (w, h))
            {
                self.motion = Some(Bitmap::new(w, h)?);
            }
            let surface = self.motion.as_ref().expect("surface created above");
            self.renderer.draw(surface, self.runtime.scene(), low)?;
            self.renderer.take_refine();
            self.refine_at = Some(now + std::time::Duration::from_millis(120));
            (surface.image()?, low)
        } else {
            if self
                .surface
                .as_ref()
                .is_none_or(|s| (s.width, s.height) != (width, height))
            {
                self.surface = Some(Bitmap::new(width, height)?);
            }
            let surface = self.surface.as_ref().expect("surface created above");
            // Seule la zone modifiée depuis l'image précédente est redessinée.
            if !self
                .renderer
                .draw_changes(surface, self.runtime.scene(), scale)?
            {
                self.presented = Some(key);
                return Ok(());
            }
            if self.renderer.take_refine() {
                self.refine_at = Some(now + std::time::Duration::from_millis(150));
            }
            (surface.image()?, scale)
        };
        let drawn = start.elapsed();
        if let Some(layer) = self.view.layer() {
            CATransaction::begin();
            CATransaction::setDisableActions(true);
            // SAFETY : CGImage est un objet Objective-C (pont sans coût) ; le calque le
            // retient.
            unsafe {
                let object = &*(&*image as *const objc2_core_graphics::CGImage)
                    .cast::<objc2::runtime::AnyObject>();
                layer.setContents(Some(object));
            }
            layer.setContentsScale(f64::from(shown));
            // Retour depuis Metal : le calque GPU est masqué dans la même transaction.
            if let Some((_, surface)) = &self.gpu {
                surface.show(false);
            }
            CATransaction::commit();
        }
        self.path = Path::CoreGraphics;
        self.presented = Some(key);
        if self.gpu.is_some() {
            self.trim_at = Some(Instant::now() + TRIM_DELAY);
        }
        self.notify_accessibility();
        if self.options.profile {
            let zone = if motion {
                "mouvement (demi-résolution)".to_string()
            } else {
                self.renderer
                    .last_region
                    .as_ref()
                    .map_or("complète".to_string(), |zones| {
                        let area: i32 = zones.iter().map(|r| (r[2] - r[0]) * (r[3] - r[1])).sum();
                        format!("{} rect., {} kpx", zones.len(), area / 1000)
                    })
            };
            eprintln!(
                "karbon-ui: image {width}×{height}, chemin Core Graphics, zone {zone} : préparation {:.1} ms, dessin {:.1} ms, total {:.1} ms",
                prepared.as_secs_f64() * 1e3,
                (drawn - prepared).as_secs_f64() * 1e3,
                start.elapsed().as_secs_f64() * 1e3,
            );
        }
        self.frame_presented()
    }
    /// Présente l'image par Metal si la scène le permet ; `false` : elle passe par Core
    /// Graphics (scène non prise en charge, pas de drawable, ou erreur du GPU, qui
    /// désactive alors le chemin Metal de la fenêtre).
    fn paint_metal(
        &mut self,
        key: (usize, usize, u32, u64),
        start: Instant,
        prepared: std::time::Duration,
    ) -> crate::Result<bool> {
        let (_, width, height, scale) = self.context();
        let Some((gpu, surface)) = &mut self.gpu else {
            return Ok(false);
        };
        let scene = self.runtime.scene();
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        surface.fit(self.view.bounds(), f64::from(scale), (width, height));
        let presented = gpu.present(&mut self.renderer, scene, surface, (width, height), scale);
        let shown = matches!(presented, Ok(metal::Presented::Shown));
        if shown {
            // Le calque Metal recouvre la vue ; l'image Core Graphics est libérée.
            surface.show(true);
            if let Some(layer) = self.view.layer() {
                // SAFETY : aucun contenu ; le calque libère l'image précédente.
                unsafe { layer.setContents(None) };
            }
        }
        CATransaction::commit();
        if let Err(error) = presented {
            // Message hors diagnostic : en anglais, comme l'API (M13 de la revue).
            eprintln!("karbon-ui: Metal disabled for this window: {error}");
            if let Some((_, surface)) = self.gpu.take() {
                surface.show(false);
                surface.layer.removeFromSuperlayer();
            }
            return Ok(false);
        }
        if let Ok(metal::Presented::Fallback(reason)) = presented
            && self.options.profile
        {
            eprintln!("karbon-ui: image par Core Graphics : {reason}");
        }
        if !shown {
            return Ok(false);
        }
        // Chemin Metal : surfaces et caches propres à Core Graphics libérés (l'image
        // suivante par Core Graphics sera dessinée en entier) ; ressources GPU allégées
        // une seconde après la dernière image.
        self.renderer.release();
        self.surface = None;
        self.motion = None;
        self.trim_at = Some(Instant::now() + TRIM_DELAY);
        self.path = Path::Metal;
        self.presented = Some(key);
        self.notify_accessibility();
        if self.options.profile {
            let (gpu_time, [build, encode, wait]) = self
                .gpu
                .as_ref()
                .map_or((None, [0.; 3]), |(gpu, _)| (gpu.gpu_time, gpu.timings));
            let gpu_time = gpu_time.map_or(String::new(), |ms| {
                format!(", GPU de l'image précédente {ms:.1} ms")
            });
            eprintln!(
                "karbon-ui: image {width}×{height}, chemin Metal : préparation {:.1} ms, dessin {:.1} ms, total {:.1} ms{gpu_time}, traduction {build:.2} ms, encodage {encode:.2} ms, attente {wait:.2} ms",
                prepared.as_secs_f64() * 1e3,
                (start.elapsed() - prepared).as_secs_f64() * 1e3,
                start.elapsed().as_secs_f64() * 1e3,
            );
            if let Some((gpu, _)) = &self.gpu {
                let memory = gpu.memory();
                if self.memory_line != memory {
                    eprintln!("karbon-ui: mémoire Metal : {memory}");
                    self.memory_line = memory;
                }
            }
        }
        if let Some((gpu, _)) = &mut self.gpu {
            self.compare.run(
                gpu,
                &mut self.renderer,
                self.runtime.scene(),
                (width, height),
                scale,
            );
        }
        self.frame_presented()?;
        Ok(true)
    }
}
/// Chemin GPU de la fenêtre : Metal, sauf en mode logiciel ou sans GPU Metal (le chemin
/// Core Graphics sert alors toutes les images).
pub(super) fn gpu(
    options: &WindowOptions,
    view: &View,
) -> Option<(metal::Renderer, metal::Surface)> {
    if options.renderer == crate::window::RenderMode::Software {
        return None;
    }
    let layer = view.layer()?;
    let created = metal::Renderer::new().and_then(|renderer| {
        let surface = renderer.surface(&layer, !options.transparent)?;
        Ok((renderer, surface))
    });
    match created {
        Ok(gpu) => Some(gpu),
        Err(error) => {
            if options.renderer == crate::window::RenderMode::Gpu || options.profile {
                eprintln!("karbon-ui: Metal unavailable, rendering with Core Graphics: {error}");
            }
            None
        }
    }
}
