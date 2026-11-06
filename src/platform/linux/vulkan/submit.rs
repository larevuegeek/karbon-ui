//! Exécution d'une image traduite (`encode/`) : instances de toutes les couches dans un
//! seul tampon (décalage par couche), arrêts, maillons et paramètres des filtres, envois
//! des glyphes et des textures nouvelles, textures des couches et des cibles de filtre
//! prises dans des réserves par taille, puis les étapes dans leur ordre. Une couche est
//! dessinée, passée en lecture, puis recomposée par sa couche parente ou lue par un
//! passage de filtre ; la cible finale est dessinée en dernier.
use super::{
    FORMAT, Renderer,
    commands::{ATTACHMENT, Batch, Pass, record, transition},
    encode::{Bind, Built, Step},
    filter::{self, Source},
    memory::{Buffer, Image},
};
use crate::platform::{linux::ffi::vulkan::*, shared::gpu::Instance};

/// Étapes et accès d'une texture lue par le shader.
const SAMPLED: (u64, u64) = (
    VK_PIPELINE_STAGE_2_FRAGMENT_SHADER_BIT,
    VK_ACCESS_2_SHADER_SAMPLED_READ_BIT,
);
/// Textures gardées dans chaque réserve.
const POOL: usize = 8;

/// Passe `image` en cible de rendu : après ses lectures précédentes ; neuve (ou image de
/// la chaîne d'échange), après l'étape d'écriture des couleurs, à laquelle la soumission
/// attend l'acquisition de l'image.
fn as_attachment(gpu: &super::device::Gpu, command: *mut std::ffi::c_void, image: &mut Image) {
    let after = if image.layout == VK_IMAGE_LAYOUT_UNDEFINED {
        (
            VK_PIPELINE_STAGE_2_COLOR_ATTACHMENT_OUTPUT_BIT,
            VK_ACCESS_2_NONE,
        )
    } else {
        SAMPLED
    };
    transition(
        gpu,
        command,
        image,
        VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        after,
        ATTACHMENT,
    );
}
/// Prend dans `pool` une texture de `size` pixels, ou en crée une au format `format`.
fn take(
    renderer: &Renderer,
    pool: &mut Vec<Image>,
    [width, height]: [usize; 2],
    format: i32,
) -> crate::Result<Image> {
    if let Some(index) = pool
        .iter()
        .position(|i| (i.width, i.height) == (width, height))
    {
        return Ok(pool.swap_remove(index));
    }
    let usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT;
    Image::new(&renderer.gpu, width, height, format, usage)
}
impl Renderer {
    /// Tampon `index` du jeu courant, assez grand pour `bytes` octets.
    fn buffer(&mut self, index: usize, bytes: usize) -> crate::Result<&mut Buffer> {
        if bytes as u64 > self.buffers[index].size {
            let size = (bytes as u64).next_power_of_two();
            self.buffers[index] = Buffer::new(&self.gpu, size, VK_BUFFER_USAGE_STORAGE_BUFFER_BIT)?;
        }
        Ok(&mut self.buffers[index])
    }
    /// Enregistre `built` vers `target` (couche 0, laissée en disposition « attachement »)
    /// dans le tampon de commandes en cours ; les images des cibles de filtre sont posées
    /// dans `built`. Renvoie les textures des couches, à rendre à la réserve une fois le
    /// travail terminé (`recycle`).
    pub(super) fn encode(
        &mut self,
        built: &mut Built,
        target: &mut Image,
    ) -> crate::Result<Vec<Image>> {
        let mut offsets = Vec::with_capacity(built.layers.len());
        let mut count = 0;
        for layer in &built.layers {
            offsets.push(count);
            count += layer.instances.len();
        }
        let size = std::mem::size_of::<Instance>();
        let instances = self.buffer(0, count.max(1) * size)?;
        for (layer, offset) in built.layers.iter().zip(&offsets) {
            instances.write_at(offset * size, &layer.instances);
        }
        let stops = self.buffer(1, std::mem::size_of_val(built.stops.as_slice()).max(1))?;
        stops.write_at(0, &built.stops);
        let clips = self.buffer(2, std::mem::size_of_val(built.clips.as_slice()).max(1))?;
        clips.write_at(0, &built.clips);
        let params = self.buffer(3, std::mem::size_of_val(built.params.as_slice()).max(1))?;
        params.write_at(0, &built.params);
        self.resources.upload(self.commands.buffer)?;
        let mut pool = std::mem::take(&mut self.pool);
        let mut images = Vec::with_capacity(built.layers.len());
        let mut filled = Ok(());
        for layer in built.layers.iter().skip(1) {
            match take(self, &mut pool, layer.texture_size, FORMAT) {
                Ok(image) => images.push(image),
                Err(error) => filled = Err(error),
            }
        }
        let mut targets = std::mem::take(&mut self.targets);
        for t in built.targets.iter_mut().filter(|t| t.image.is_none()) {
            match take(self, &mut targets, t.size, filter::FORMAT) {
                Ok(image) => t.image = Some(image),
                Err(error) => filled = Err(error),
            }
        }
        self.pool = pool;
        self.targets = targets;
        filled?;
        let gpu = &self.gpu;
        let command = self.commands.buffer;
        let view = |images: &[Image], built: &Built, source: Source| match source {
            Source::Layer(index) => images[index - 1].view,
            Source::Target(index) => built.targets[index].image.as_ref().map_or(0, |i| i.view),
        };
        for step in &built.steps {
            match step {
                Step::Draw(index) => {
                    let index = *index;
                    let layer = &built.layers[index];
                    let batches = layer
                        .batches
                        .iter()
                        .map(|b| Batch {
                            texture: match b.bind {
                                Bind::None => 0,
                                Bind::Texture(view) => view,
                                Bind::Layer(child) => images[child - 1].view,
                                Bind::Target(t) => view_of(built, t),
                            },
                            scissor: b.scissor.map(|v| v as u32),
                            start: (offsets[index] + b.start) as u32,
                            count: b.count as u32,
                        })
                        .collect::<Vec<_>>();
                    let image = if index == 0 {
                        &mut *target
                    } else {
                        &mut images[index - 1]
                    };
                    as_attachment(gpu, command, image);
                    record(
                        gpu,
                        command,
                        &self.pipeline,
                        Pass {
                            target: image,
                            origin: [layer.origin[0] as f32, layer.origin[1] as f32],
                            clear: layer.clear,
                            batches: &batches,
                        },
                        [&self.buffers[0], &self.buffers[1], &self.buffers[2]],
                        self.neutral.view,
                    );
                    if index != 0 {
                        transition(
                            gpu,
                            command,
                            &mut images[index - 1],
                            VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                            ATTACHMENT,
                            SAMPLED,
                        );
                    }
                }
                Step::Filter(pass) => {
                    let views = [
                        view(&images, built, pass.source),
                        pass.below
                            .map_or(self.neutral.view, |below| view(&images, built, below)),
                    ];
                    let image = built.targets[pass.target]
                        .image
                        .as_mut()
                        .expect("filter target");
                    as_attachment(gpu, command, image);
                    filter::record(
                        gpu,
                        command,
                        &self.filters,
                        filter::Record {
                            target: image,
                            size: pass.size,
                            kernel: pass.kernel,
                            index: pass.params,
                            params: &self.buffers[3],
                            views,
                            sampler: self.pipeline.sampler,
                        },
                    );
                    transition(
                        gpu,
                        command,
                        image,
                        VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                        ATTACHMENT,
                        SAMPLED,
                    );
                }
            }
        }
        Ok(images)
    }
    /// Après une image terminée : textures des couches et cibles de filtre rendues à leurs
    /// réserves (bornées), résultats des groupes filtrés gardés par les ressources.
    pub(super) fn recycle(&mut self, images: Vec<Image>, built: &mut Built) {
        for mut image in images {
            image.layout = VK_IMAGE_LAYOUT_UNDEFINED;
            self.pool.push(image);
        }
        for target in built.targets.drain(..) {
            let Some(mut image) = target.image else {
                continue;
            };
            match target.keep {
                Some(kept) => self.resources.store_filtered(image, kept),
                None => {
                    image.layout = VK_IMAGE_LAYOUT_UNDEFINED;
                    self.targets.push(image);
                }
            }
        }
        for pool in [&mut self.pool, &mut self.targets] {
            if pool.len() > POOL {
                let excess = pool.len() - POOL;
                pool.drain(..excess);
            }
        }
    }
}
/// Vue de la cible de filtre `index` (posée par `encode`).
fn view_of(built: &Built, index: usize) -> u64 {
    built.targets[index].image.as_ref().map_or(0, |i| i.view)
}
