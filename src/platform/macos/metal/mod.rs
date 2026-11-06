//! Renderer GPU Metal des fenêtres macOS (Apple Silicon et Mac Intel).
//!
//! Chaque image est redessinée en entière : la scène est traduite en couches
//! d'instances (`encode/`), un quad par élément, dont le fragment calcule la couverture
//! par distance signée (rectangles à coins indépendants, traits, segments) ou lit une
//! texture (images, atlas de glyphes, icônes et SVG rastérisés une fois par Core
//! Graphics). Les découpes rectangulaires deviennent scissor + couverture, les découpes
//! arrondies ou tournées et les masques en dégradé une chaîne évaluée par le shader, les
//! groupes à opacité et les groupes filtrés des textures intermédiaires bornées à leur
//! contenu ; filtres et `backdrop-filter` passent par des passages de filtre (`filter/`).
//!
//! Le renderer Core Graphics (`graphics/`) reste le repli (mode logiciel, pas de GPU,
//! erreur du GPU, image sans drawable, scène refusée par `Error::Unsupported`, aucune
//! aujourd'hui) et garde le rendu hors écran de référence (aperçus, exports, mesures). Il
//! prête aussi au chemin Metal ses mises en page Core Text et sa rastérisation.
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `device.rs` | périphérique, file, shaders compilés au lancement, pipeline, stockage |
//! | `shaders.metal` | source MSL embarquée |
//! | `instance.rs` | structures partagées avec les shaders |
//! | `encode/` | scène → couches, lots d'instances et étapes (formes, texte, images, raster, effets) |
//! | `filter/` | filtres CSS en passages GPU (flou, ombre, matrices de couleur) |
//! | `compare.rs` | diagnostic `KARBON_METAL_COMPARE` (Metal contre Core Graphics) |
//! | `atlas.rs` | atlas de glyphes Core Text |
//! | `resources.rs` | textures gardées d'une image à l'autre |
//! | `submit.rs` | étapes de l'image, anneau de tampons, relecture |
//! | `surface.rs` | `CAMetalLayer` de la fenêtre |
mod atlas;
mod compare;
mod device;
mod encode;
mod filter;
mod instance;
mod resources;
mod submit;
mod surface;
#[cfg(test)]
mod tests;

use super::graphics::Renderer as CgRenderer;
use crate::ui::scene::Scene;
pub(super) use compare::Comparison;
pub(super) use resources::Trim;
pub(super) use surface::Surface;

/// Issue d'une présentation.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Presented {
    /// Image présentée par Metal.
    Shown,
    /// Aucun drawable disponible : l'image passe par Core Graphics.
    NoDrawable,
    /// Image que Metal ne rend pas à l'identique (raison) : elle passe par Core Graphics.
    Fallback(&'static str),
}

pub(super) struct Renderer {
    gpu: device::Gpu,
    resources: resources::Resources,
    ring: submit::Ring,
    /// Temps GPU de l'image précédente, en millisecondes (`--profile`), si elle est
    /// terminée au moment de présenter la suivante.
    pub(super) gpu_time: Option<f64>,
    /// Image encodée précédente, recyclée par la suivante (`encode::Built::recycle`).
    spare: Option<encode::Built>,
    /// Temps CPU de la dernière présentation, en millisecondes (`--profile`) : traduction
    /// de la scène en instances, encodage des passages, attente du drawable et de la
    /// planification.
    pub(super) timings: [f64; 3],
}
impl Renderer {
    /// Libellé du chemin Metal, transmis à `ViewContext::backend`.
    pub(super) const LABEL: &'static str = "GPU · Metal";
    /// Renderer sur le GPU du système ; erreur s'il n'y a pas de périphérique Metal
    /// (machine virtuelle sans GPU) ou si les shaders ne compilent pas.
    pub(super) fn new() -> crate::Result<Self> {
        Self::with_storage(None)
    }
    /// Renderer dont les modes de stockage sont imposés (tests du chemin `Managed`).
    fn with_storage(storage: Option<device::Storage>) -> crate::Result<Self> {
        let gpu = device::Gpu::new(storage)?;
        let resources = resources::Resources::new(&gpu)?;
        Ok(Self {
            gpu,
            resources,
            ring: submit::Ring::default(),
            gpu_time: None,
            spare: None,
            timings: [0.; 3],
        })
    }
    /// `true` s'il existe un périphérique Metal.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn available() -> bool {
        device::system_device().is_some()
    }
    /// Mémoire GPU tenue par le renderer, pour `--profile`.
    pub(super) fn memory(&self) -> String {
        let [atlas, images, rasters, pool, filtered] = self.resources.usage();
        let mb = |b: usize| b as f64 / 1048576.;
        format!(
            "atlas {:.1} Mo, images {:.1} Mo, rastérisations {:.1} Mo, cibles {:.1} Mo, groupes filtrés {:.1} Mo, tampons {:.1} Mo",
            mb(atlas),
            mb(images),
            mb(rasters),
            mb(pool),
            mb(filtered),
            mb(self.ring.bytes())
        )
    }
    /// Garde les vecteurs de `built` pour l'image suivante.
    fn keep(&mut self, mut built: encode::Built) {
        built.recycle();
        self.spare = Some(built);
    }
    /// Libère les ressources GPU qui peuvent l'être entre deux dessins (voir
    /// `resources::Trim`).
    pub(super) fn trim(&mut self, what: Trim) {
        self.spare = None;
        self.resources.trim(what);
        self.ring.trim();
    }
    /// Calque d'une fenêtre, sous-calque (masqué) de `parent`.
    pub(super) fn surface(
        &self,
        parent: &objc2_quartz_core::CALayer,
        opaque: bool,
    ) -> crate::Result<Surface> {
        Surface::new(&self.gpu.device, parent, opaque)
    }
    fn build(
        &mut self,
        cg: &mut CgRenderer,
        scene: &Scene,
        size: (usize, usize),
        scale: f32,
    ) -> crate::Result<encode::Built> {
        scene.validate()?;
        if !(scale.is_finite() && scale > 0.) {
            return Err(crate::Error::invalid_input("invalid render scale"));
        }
        if self.resources.atlas.full() {
            // Le GPU ne doit plus lire les pages vidées.
            self.ring.idle();
            self.resources.atlas.clear();
        }
        self.resources.begin();
        let spare = self.spare.take().unwrap_or_default();
        let mut builder = encode::Builder::new(
            &self.gpu,
            &mut self.resources,
            cg,
            scene,
            size,
            scale,
            spare,
        );
        let result = builder.run(&scene.commands);
        let built = builder.finish();
        if result.is_err() {
            self.resources.end();
        }
        result.map(|()| built)
    }
    /// Rend `scene` dans une texture de `width` × `height` pixels relue : pixels BGRA
    /// prémultipliés, comme `graphics::Renderer::render` (tests et diagnostic de
    /// comparaison).
    pub(super) fn render(
        &mut self,
        cg: &mut CgRenderer,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid render size"));
        }
        let size = (width as usize, height as usize);
        let built = self.build(cg, scene, size, scale)?;
        let target = self.gpu.target_texture(size.0, size.1)?;
        let cb = submit::encode(
            &self.gpu,
            &mut self.ring,
            &mut self.resources,
            &built,
            &target,
        )?;
        let pixels = submit::read_back(&self.gpu, &cb, &target, size);
        self.resources.end();
        self.keep(built);
        pixels
    }
    /// Dessine `scene` sur un drawable de `surface` (taille déjà ajustée) et le présente
    /// dans la transaction Core Animation ouverte par l'appelant.
    pub(super) fn present(
        &mut self,
        cg: &mut CgRenderer,
        scene: &Scene,
        surface: &Surface,
        size: (usize, usize),
        scale: f32,
    ) -> crate::Result<Presented> {
        use objc2_metal::{MTLCommandBuffer, MTLCommandBufferStatus, MTLDrawable};
        use objc2_quartz_core::CAMetalDrawable;
        // Image précédente en erreur (GPU retiré ou réinitialisé, délai dépassé) : le
        // chemin Metal de la fenêtre est abandonné, l'appelant passe à Core Graphics.
        if let Some(last) = &self.ring.last
            && last.status() == MTLCommandBufferStatus::Error
        {
            if let Some(error) = last.error() {
                eprintln!("karbon-ui: Metal : {}", error.localizedDescription());
            }
            return Err(crate::Error::DeviceLost {
                operation: "MTLCommandBuffer",
                code: 0x8000_4005_u32 as i32,
            });
        }
        // Diagnostic : images confiées à Core Graphics tant que la variable existe (la
        // sonde native vérifie ainsi le passage d'un chemin à l'autre).
        if std::env::var_os("KARBON_METAL_FALLBACK").is_some() {
            return Ok(Presented::Fallback("KARBON_METAL_FALLBACK"));
        }
        let start = std::time::Instant::now();
        let ms = |since: std::time::Instant| since.elapsed().as_secs_f64() * 1e3;
        let built = match self.build(cg, scene, size, scale) {
            Err(crate::Error::Unsupported { reason }) => return Ok(Presented::Fallback(reason)),
            other => other?,
        };
        self.timings[0] = ms(start);
        self.gpu_time = self.ring.last.as_ref().and_then(|cb| {
            (cb.status() == MTLCommandBufferStatus::Completed)
                .then(|| (cb.GPUEndTime() - cb.GPUStartTime()) * 1e3)
        });
        let wait = std::time::Instant::now();
        let Some(drawable) = surface.next() else {
            self.resources.end();
            return Ok(Presented::NoDrawable);
        };
        self.timings[2] = ms(wait);
        let target = drawable.texture();
        let encoding = std::time::Instant::now();
        let cb = submit::encode(
            &self.gpu,
            &mut self.ring,
            &mut self.resources,
            &built,
            &target,
        )?;
        self.resources.end();
        cb.commit();
        self.keep(built);
        self.timings[1] = ms(encoding);
        // `presentsWithTransaction` : présenter une fois le travail planifié, dans la
        // transaction de l'appelant.
        let wait = std::time::Instant::now();
        cb.waitUntilScheduled();
        drawable.present();
        self.timings[2] += ms(wait);
        Ok(Presented::Shown)
    }
}
