//! Renderer GPU Vulkan de Linux : infrastructure (étape 1) et traduction de toutes les
//! scènes (étape 2) ; sert les rendus hors écran en mode `Gpu`, les fenêtres à l'étape 3.
//!
//! Même conception que le renderer Metal de macOS : chaque élément est un quad instancié
//! (`platform/shared/gpu.rs`) dont le fragment calcule la couverture par distance signée
//! ou lit une texture ; shaders GLSL portés de `macos/metal/shaders.metal`, compilés en
//! SPIR-V versionné (`shaders/build.sh`). Vulkan 1.3 (rendu dynamique, synchronization2)
//! et descripteurs poussés, chargé au lancement : sans Vulkan, Cairo dessine.
//!
//! | Module | Rôle |
//! | --- | --- |
//! | `device.rs` | instance, choix du GPU, appareil logique et file |
//! | `memory.rs` | tampons projetés et images du GPU |
//! | `pipeline.rs` | shaders SPIR-V, disposition des descripteurs, pipeline graphique |
//! | `commands.rs` | tampon de commandes, barrières, passage de rendu des lots |
//! | `resources.rs` | ressources gardées d'une image à l'autre et leurs envois |
//! | `atlas.rs` | glyphes rastérisés par Cairo, pages et copies vers le GPU |
//! | `textures.rs` | textures des images et des rastérisations, mipmaps |
//! | `staging.rs` | tampon intermédiaire des envois |
//! | `encode/` | scène → couches, lots et étapes (copie de `macos/metal/encode/`) |
//! | `filter/` | passages des filtres CSS (copie adaptée de `macos/metal/filter/`) |
//! | `submit.rs` | exécution des étapes d'une image, textures des couches |
//! | `surface.rs` | surface Wayland et chaîne d'échange d'une fenêtre |
//! | `present.rs` | image d'une fenêtre : traduction, rendu, présentation |
//! | `shaders/` | sources GLSL et SPIR-V compilé |
mod atlas;
mod commands;
mod device;
mod encode;
mod filter;
mod memory;
mod pipeline;
mod present;
mod resources;
mod staging;
mod submit;
mod surface;
#[cfg(test)]
mod tests;
mod textures;

use super::graphics::Renderer as Cairo;
use crate::platform::{
    linux::ffi::vulkan::*,
    shared::gpu::{ClipNode, Instance, Stop},
};
use crate::ui::scene::Scene;
use commands::{ATTACHMENT, Commands, transition};
pub(super) use device::Gpu;
use encode::Builder;
use memory::{Buffer, Image};
use pipeline::Pipeline;
pub(super) use present::Presented;
use resources::Resources;
use std::rc::Rc;
pub(super) use surface::{Native, Surface};

/// Ce que `Renderer::trim` libère.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Trim {
    /// Repos (une seconde après la dernière image) : réserves et textures inutilisées.
    Idle,
    /// Fenêtre réduite ou masquée (`suspended`) : tout.
    Hidden,
}
/// Format des cibles : BGRA 8 bits non linéaire, celui des pixels du moteur.
const FORMAT: i32 = VK_FORMAT_B8G8R8A8_UNORM;

pub(super) struct Renderer {
    commands: Commands,
    pipeline: Pipeline,
    filters: filter::FilterPipeline,
    /// Tampons des instances, des arrêts, des maillons et des paramètres des filtres de
    /// l'image en cours.
    buffers: [Buffer; 4],
    /// Texture 1 × 1 liée aux lots sans texture.
    neutral: Image,
    /// Textures des couches intermédiaires et des cibles de filtre, réutilisées d'une
    /// image à l'autre.
    pool: Vec<Image>,
    targets: Vec<Image>,
    resources: Resources,
    /// Image présentée dont le GPU n'a peut-être pas fini (`present.rs`) : textures des
    /// couches et des filtres à rendre aux réserves, image traduite.
    in_flight: Option<(Vec<Image>, encode::Built)>,
    /// Durées de la dernière image présentée (`--profile`), en millisecondes :
    /// traduction, enregistrement, soumission et présentation, attente de l'image
    /// précédente.
    pub(super) timings: [f64; 4],
    gpu: Rc<Gpu>,
}
impl Renderer {
    /// Libellé du chemin Vulkan, transmis à `ViewContext::backend`.
    pub(super) const LABEL: &'static str = "GPU · Vulkan";
    /// Renderer sur le GPU préféré ; `allow_cpu` admet un rendu Vulkan sur processeur
    /// (tests). Erreur sans Vulkan 1.3 ou sans descripteurs poussés.
    pub(super) fn new(allow_cpu: bool) -> crate::Result<Self> {
        Self::on(&Rc::new(Gpu::new(allow_cpu)?))
    }
    /// Renderer sur le contexte `gpu`, partagé avec d'autres renderers (fenêtres d'une
    /// même boucle : une instance et un appareil pour toutes).
    pub(super) fn on(gpu: &Rc<Gpu>) -> crate::Result<Self> {
        let gpu = Rc::clone(gpu);
        let storage = VK_BUFFER_USAGE_STORAGE_BUFFER_BIT;
        let buffers = [
            Buffer::new(&gpu, 64 * std::mem::size_of::<Instance>() as u64, storage)?,
            Buffer::new(&gpu, 64 * std::mem::size_of::<Stop>() as u64, storage)?,
            Buffer::new(&gpu, 64 * std::mem::size_of::<ClipNode>() as u64, storage)?,
            Buffer::new(
                &gpu,
                4 * std::mem::size_of::<filter::Params>() as u64,
                storage,
            )?,
        ];
        let mut neutral = Image::new(&gpu, 1, 1, FORMAT, VK_IMAGE_USAGE_SAMPLED_BIT)?;
        let commands = Commands::new(&gpu)?;
        commands.begin()?;
        transition(
            &gpu,
            commands.buffer,
            &mut neutral,
            VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            (VK_PIPELINE_STAGE_2_NONE, VK_ACCESS_2_NONE),
            (
                VK_PIPELINE_STAGE_2_FRAGMENT_SHADER_BIT,
                VK_ACCESS_2_SHADER_SAMPLED_READ_BIT,
            ),
        );
        commands.submit_wait()?;
        Ok(Self {
            pipeline: Pipeline::new(&gpu, FORMAT)?,
            filters: filter::FilterPipeline::new(&gpu)?,
            commands,
            buffers,
            neutral,
            pool: Vec::new(),
            targets: Vec::new(),
            resources: Resources::new(&gpu),
            in_flight: None,
            timings: [0.; 4],
            gpu,
        })
    }
    /// Libère ce qui peut l'être entre deux images (le GPU a terminé la précédente) :
    /// réserves des couches et des cibles de filtre, tampon intermédiaire, textures que
    /// la dernière image n'a pas employées ; `Trim::Hidden` : atlas, textures et groupes
    /// filtrés aussi. La prochaine image recrée ce dont elle a besoin.
    pub(super) fn trim(&mut self, what: Trim) {
        // L'image en vol est terminée avant que ses ressources ne soient libérées.
        if self.finish().is_err() {
            return;
        }
        self.pool.clear();
        self.targets.clear();
        self.resources.trim(what == Trim::Hidden);
    }
    /// Nom du GPU utilisé.
    pub(super) fn device_name(&self) -> &str {
        &self.gpu.name
    }
    /// Traduit `scene` pour une cible de `size` pixels à `scale` ; `cairo` (le renderer
    /// Cairo de l'appelant) met le texte en page et prépare les images. Les ressources
    /// sont ouvertes pour l'image (`Resources::begin`) : l'appelant les referme
    /// (`Resources::end`) une fois l'image terminée, ou abandonnée.
    fn build(
        &mut self,
        cairo: &mut Cairo,
        scene: &Scene,
        size: (usize, usize),
        scale: f32,
    ) -> crate::Result<encode::Built> {
        scene.validate()?;
        self.resources.begin();
        cairo.begin_frame(scene)?;
        let mut builder = Builder::new(&mut self.resources, cairo, scene, size, scale);
        let run = builder.run(&scene.commands);
        let built = builder.finish();
        cairo.end_frame();
        run.map(|()| built)
    }
    /// Dessine `scene` à `scale` sur une image hors écran de `width` × `height` et relit
    /// ses pixels BGRA prémultipliés.
    pub(super) fn render(
        &mut self,
        cairo: &mut Cairo,
        scene: &Scene,
        width: usize,
        height: usize,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        self.finish()?;
        let result = self
            .build(cairo, scene, (width, height), scale)
            .and_then(|mut built| self.draw(&mut built, width, height));
        self.resources.end();
        result
    }
    /// Dessine `instances` (un lot, sans texture) sur une image hors écran effacée en
    /// `clear` (prémultiplié), et relit ses pixels (tests du shader).
    #[cfg(test)]
    pub(super) fn render_instances(
        &mut self,
        width: usize,
        height: usize,
        clear: [f32; 4],
        instances: &[Instance],
        stops: &[Stop],
        clips: &[ClipNode],
    ) -> crate::Result<Vec<u8>> {
        let mut built = encode::Built::single((width, height), clear, instances, stops, clips);
        self.draw(&mut built, width, height)
    }
    /// Exécute `built` vers une image hors écran, puis la relit.
    fn draw(
        &mut self,
        built: &mut encode::Built,
        width: usize,
        height: usize,
    ) -> crate::Result<Vec<u8>> {
        let usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT;
        let mut target = Image::new(&self.gpu, width, height, FORMAT, usage)?;
        let readback = Buffer::new(
            &self.gpu,
            (width * height * 4) as u64,
            VK_BUFFER_USAGE_TRANSFER_DST_BIT,
        )?;
        self.commands.begin()?;
        let images = match self.encode(built, &mut target) {
            Ok(images) => images,
            Err(error) => {
                // Rien n'est soumis : les copies enregistrées n'auront pas lieu.
                self.resources.reset();
                return Err(error);
            }
        };
        let gpu = &self.gpu;
        let command = self.commands.buffer;
        transition(
            gpu,
            command,
            &mut target,
            VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            ATTACHMENT,
            (VK_PIPELINE_STAGE_2_COPY_BIT, VK_ACCESS_2_TRANSFER_READ_BIT),
        );
        let region = VkBufferImageCopy {
            bufferOffset: 0,
            bufferRowLength: 0,
            bufferImageHeight: 0,
            imageSubresource: VkImageSubresourceLayers {
                aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            imageOffset: VkOffset3D { x: 0, y: 0, z: 0 },
            imageExtent: VkExtent3D {
                width: width as u32,
                height: height as u32,
                depth: 1,
            },
        };
        // SAFETY : tampon en cours d'enregistrement ; image en disposition de source ;
        // dépendance entièrement remplie (autres listes vides).
        unsafe {
            (gpu.fns.vkCmdCopyImageToBuffer)(
                command,
                target.raw,
                VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                readback.raw,
                1,
                &region,
            );
            // La copie est rendue visible à l'hôte avant la relecture.
            let barrier = VkMemoryBarrier2 {
                sType: VK_STRUCTURE_TYPE_MEMORY_BARRIER_2,
                pNext: std::ptr::null(),
                srcStageMask: VK_PIPELINE_STAGE_2_COPY_BIT,
                srcAccessMask: VK_ACCESS_2_TRANSFER_WRITE_BIT,
                dstStageMask: VK_PIPELINE_STAGE_2_HOST_BIT,
                dstAccessMask: VK_ACCESS_2_HOST_READ_BIT,
            };
            let mut dependency: VkDependencyInfo = std::mem::zeroed();
            dependency.sType = VK_STRUCTURE_TYPE_DEPENDENCY_INFO;
            dependency.memoryBarrierCount = 1;
            dependency.pMemoryBarriers = &barrier;
            (gpu.fns.vkCmdPipelineBarrier2)(command, &dependency);
        };
        let submitted = self.commands.submit_wait();
        self.recycle(images, built);
        submitted?;
        Ok(readback.read(width * height * 4).to_vec())
    }
}
