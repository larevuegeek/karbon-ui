//! Enregistrement et soumission : tampon de commandes et sa barrière d'attente, barrières
//! de disposition des images (synchronization2), et passage de rendu d'une liste de lots
//! (cible effacée, puis un appel par lot : ciseau, texture poussée, instances).
use super::{
    device::{Gpu, check},
    memory::{Buffer, Image, color_range},
    pipeline::Pipeline,
};
use crate::platform::linux::ffi::vulkan::*;
use std::{ffi::c_void, rc::Rc};

/// Lot d'instances partageant une texture et un ciseau.
#[derive(Clone, Copy, Debug)]
pub(super) struct Batch {
    /// Vue de la texture lue, ou 0 (texture neutre).
    pub texture: u64,
    /// Ciseau en pixels de la cible : x, y, largeur, hauteur.
    pub scissor: [u32; 4],
    pub start: u32,
    pub count: u32,
}
/// Passage de rendu vers une image.
pub(super) struct Pass<'a> {
    pub target: &'a mut Image,
    /// Coin de la cible dans les pixels de la fenêtre.
    pub origin: [f32; 2],
    /// Couleur d'effacement prémultipliée.
    pub clear: [f32; 4],
    pub batches: &'a [Batch],
}
/// Tampon de commandes réutilisable, et la barrière qui signale sa fin.
pub(super) struct Commands {
    pool: u64,
    pub buffer: *mut c_void,
    fence: u64,
    gpu: Rc<Gpu>,
}
impl Commands {
    pub fn new(gpu: &Rc<Gpu>) -> crate::Result<Self> {
        let mut this = Self {
            pool: 0,
            buffer: std::ptr::null_mut(),
            fence: 0,
            gpu: Rc::clone(gpu),
        };
        let pool = VkCommandPoolCreateInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: gpu.family,
        };
        let fence = VkFenceCreateInfo {
            sType: VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
        };
        // SAFETY : descriptions locales ; objets détruits par `Drop`.
        unsafe {
            check(
                (gpu.fns.vkCreateCommandPool)(gpu.device, &pool, std::ptr::null(), &mut this.pool),
                "vkCreateCommandPool",
            )?;
            let allocate = VkCommandBufferAllocateInfo {
                sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                pNext: std::ptr::null(),
                commandPool: this.pool,
                level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
                commandBufferCount: 1,
            };
            check(
                (gpu.fns.vkAllocateCommandBuffers)(gpu.device, &allocate, &mut this.buffer),
                "vkAllocateCommandBuffers",
            )?;
            check(
                (gpu.fns.vkCreateFence)(gpu.device, &fence, std::ptr::null(), &mut this.fence),
                "vkCreateFence",
            )?;
        }
        Ok(this)
    }
    pub fn begin(&self) -> crate::Result<()> {
        let info = VkCommandBufferBeginInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            pNext: std::ptr::null(),
            flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
            pInheritanceInfo: std::ptr::null(),
        };
        // SAFETY : tampon valide, pas en cours d'exécution (attendu par `submit_wait`).
        unsafe {
            check(
                (self.gpu.fns.vkResetCommandBuffer)(self.buffer, 0),
                "vkResetCommandBuffer",
            )?;
            check(
                (self.gpu.fns.vkBeginCommandBuffer)(self.buffer, &info),
                "vkBeginCommandBuffer",
            )
        }
    }
    /// Termine l'enregistrement, soumet et attend la fin du travail.
    pub fn submit_wait(&self) -> crate::Result<()> {
        self.submit(None)?;
        self.wait()
    }
    /// Termine l'enregistrement et soumet, sans attendre ; `semaphores` : attendu avant
    /// l'écriture des couleurs (image acquise), signalé à la fin (présentation).
    pub fn submit(&self, semaphores: Option<(u64, u64)>) -> crate::Result<()> {
        let gpu = &self.gpu;
        let stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
        let (wait, signal) = semaphores.unzip();
        let info = VkSubmitInfo {
            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            pNext: std::ptr::null(),
            waitSemaphoreCount: u32::from(wait.is_some()),
            pWaitSemaphores: wait.as_ref().map_or(std::ptr::null(), |w| w),
            pWaitDstStageMask: &stage,
            commandBufferCount: 1,
            pCommandBuffers: &self.buffer,
            signalSemaphoreCount: u32::from(signal.is_some()),
            pSignalSemaphores: signal.as_ref().map_or(std::ptr::null(), |s| s),
        };
        // SAFETY : tampon enregistré ; barrière non signalée ; sémaphores valides.
        unsafe {
            check(
                (gpu.fns.vkEndCommandBuffer)(self.buffer),
                "vkEndCommandBuffer",
            )?;
            check(
                (gpu.fns.vkQueueSubmit)(gpu.queue, 1, &info, self.fence),
                "vkQueueSubmit",
            )
        }
    }
    /// Attend la fin du travail soumis (bornée à cinq secondes), puis réarme la barrière.
    pub fn wait(&self) -> crate::Result<()> {
        let gpu = &self.gpu;
        // SAFETY : barrière signalée par la soumission précédente.
        unsafe {
            let waited = (gpu.fns.vkWaitForFences)(gpu.device, 1, &self.fence, 1, 5_000_000_000);
            check(waited, "vkWaitForFences")?;
            check(
                (gpu.fns.vkResetFences)(gpu.device, 1, &self.fence),
                "vkResetFences",
            )
        }
    }
}
impl Drop for Commands {
    fn drop(&mut self) {
        let gpu = &self.gpu;
        // SAFETY : travail terminé (attendu) ; le tampon est libéré avec son pool.
        unsafe {
            (gpu.fns.vkDeviceWaitIdle)(gpu.device);
            (gpu.fns.vkDestroyFence)(gpu.device, self.fence, std::ptr::null());
            (gpu.fns.vkDestroyCommandPool)(gpu.device, self.pool, std::ptr::null());
        }
    }
}
/// Passe `image` à la disposition `layout`, après les écritures de `after` et avant les
/// accès de `before` (étapes et accès de synchronization2).
pub(super) fn transition(
    gpu: &Gpu,
    command: *mut c_void,
    image: &mut Image,
    layout: i32,
    after: (u64, u64),
    before: (u64, u64),
) {
    let barrier = VkImageMemoryBarrier2 {
        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER_2,
        pNext: std::ptr::null(),
        srcStageMask: after.0,
        srcAccessMask: after.1,
        dstStageMask: before.0,
        dstAccessMask: before.1,
        oldLayout: image.layout,
        newLayout: layout,
        srcQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED,
        dstQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED,
        image: image.raw,
        subresourceRange: color_range(),
    };
    // SAFETY : structure de dépendance entièrement remplie (autres listes vides).
    let mut dependency: VkDependencyInfo = unsafe { std::mem::zeroed() };
    dependency.sType = VK_STRUCTURE_TYPE_DEPENDENCY_INFO;
    dependency.imageMemoryBarrierCount = 1;
    dependency.pImageMemoryBarriers = &barrier;
    // SAFETY : tampon en cours d'enregistrement ; image valide.
    unsafe { (gpu.fns.vkCmdPipelineBarrier2)(command, &dependency) };
    image.layout = layout;
}
/// Passe les niveaux `base .. base + count` de l'image `raw` de `old` à `new`, après les
/// accès de `after` et avant ceux de `before` (calcul des mipmaps, où les niveaux n'ont
/// pas tous la même disposition).
#[allow(clippy::too_many_arguments)]
pub(super) fn transition_levels(
    gpu: &Gpu,
    command: *mut c_void,
    raw: u64,
    (base, count): (u32, u32),
    (old, new): (i32, i32),
    after: (u64, u64),
    before: (u64, u64),
) {
    let barrier = VkImageMemoryBarrier2 {
        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER_2,
        pNext: std::ptr::null(),
        srcStageMask: after.0,
        srcAccessMask: after.1,
        dstStageMask: before.0,
        dstAccessMask: before.1,
        oldLayout: old,
        newLayout: new,
        srcQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED,
        dstQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED,
        image: raw,
        subresourceRange: VkImageSubresourceRange {
            baseMipLevel: base,
            levelCount: count,
            ..color_range()
        },
    };
    // SAFETY : structure de dépendance entièrement remplie (autres listes vides).
    let mut dependency: VkDependencyInfo = unsafe { std::mem::zeroed() };
    dependency.sType = VK_STRUCTURE_TYPE_DEPENDENCY_INFO;
    dependency.imageMemoryBarrierCount = 1;
    dependency.pImageMemoryBarriers = &barrier;
    // SAFETY : tampon en cours d'enregistrement ; image valide.
    unsafe { (gpu.fns.vkCmdPipelineBarrier2)(command, &dependency) };
}
/// Étapes et accès d'une cible de rendu.
pub(super) const ATTACHMENT: (u64, u64) = (
    VK_PIPELINE_STAGE_2_COLOR_ATTACHMENT_OUTPUT_BIT,
    VK_ACCESS_2_COLOR_ATTACHMENT_WRITE_BIT | VK_ACCESS_2_COLOR_ATTACHMENT_READ_BIT,
);
/// Enregistre `pass` : la cible (disposition « attachement ») est effacée, puis chaque lot
/// lit les tampons d'instances, d'arrêts et de maillons, et sa texture.
#[allow(clippy::too_many_arguments)]
pub(super) fn record(
    gpu: &Gpu,
    command: *mut c_void,
    pipeline: &Pipeline,
    pass: Pass<'_>,
    [instances, stops, clips]: [&Buffer; 3],
    neutral: u64,
) {
    let (width, height) = (pass.target.width as u32, pass.target.height as u32);
    let attachment = VkRenderingAttachmentInfo {
        sType: VK_STRUCTURE_TYPE_RENDERING_ATTACHMENT_INFO,
        pNext: std::ptr::null(),
        imageView: pass.target.view,
        imageLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        resolveMode: 0,
        resolveImageView: 0,
        resolveImageLayout: VK_IMAGE_LAYOUT_UNDEFINED,
        loadOp: VK_ATTACHMENT_LOAD_OP_CLEAR,
        storeOp: VK_ATTACHMENT_STORE_OP_STORE,
        clearValue: VkClearValue {
            color: VkClearColorValue {
                float32: pass.clear,
            },
        },
    };
    let area = VkRect2D {
        offset: VkOffset2D { x: 0, y: 0 },
        extent: VkExtent2D { width, height },
    };
    // SAFETY : informations de rendu entièrement remplies (pas de profondeur).
    let mut rendering: VkRenderingInfo = unsafe { std::mem::zeroed() };
    rendering.sType = VK_STRUCTURE_TYPE_RENDERING_INFO;
    rendering.renderArea = area;
    rendering.layerCount = 1;
    rendering.colorAttachmentCount = 1;
    rendering.pColorAttachments = &attachment;
    let viewport = VkViewport {
        x: 0.,
        y: 0.,
        width: width as f32,
        height: height as f32,
        minDepth: 0.,
        maxDepth: 1.,
    };
    let frame = [pass.origin[0], pass.origin[1], width as f32, height as f32];
    let buffer = |b: &Buffer| VkDescriptorBufferInfo {
        buffer: b.raw,
        offset: 0,
        range: b.size,
    };
    let buffers = [buffer(instances), buffer(stops), buffer(clips)];
    let write = |binding: u32, kind: i32| {
        // SAFETY : écriture de descripteur entièrement remplie ci-dessous.
        let mut w: VkWriteDescriptorSet = unsafe { std::mem::zeroed() };
        w.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
        w.dstBinding = binding;
        w.descriptorCount = 1;
        w.descriptorType = kind;
        w
    };
    // SAFETY : tampon en cours d'enregistrement ; objets valides pendant l'exécution.
    unsafe {
        (gpu.fns.vkCmdBeginRendering)(command, &rendering);
        (gpu.fns.vkCmdBindPipeline)(command, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline.pipeline);
        (gpu.fns.vkCmdSetViewport)(command, 0, 1, &viewport);
        (gpu.fns.vkCmdPushConstants)(
            command,
            pipeline.layout,
            VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
            0,
            super::pipeline::FRAME_BYTES,
            frame.as_ptr().cast(),
        );
        let mut storage = [0, 1, 2].map(|b| write(b, VK_DESCRIPTOR_TYPE_STORAGE_BUFFER));
        for (w, info) in storage.iter_mut().zip(&buffers) {
            w.pBufferInfo = info;
        }
        (gpu.fns.vkCmdPushDescriptorSetKHR)(
            command,
            VK_PIPELINE_BIND_POINT_GRAPHICS,
            pipeline.layout,
            0,
            storage.len() as u32,
            storage.as_ptr(),
        );
        let mut bound = None;
        for batch in pass.batches {
            let view = if batch.texture == 0 {
                neutral
            } else {
                batch.texture
            };
            if bound != Some(view) {
                let image = VkDescriptorImageInfo {
                    sampler: pipeline.sampler,
                    imageView: view,
                    imageLayout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                };
                let mut w = write(3, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER);
                w.pImageInfo = &image;
                (gpu.fns.vkCmdPushDescriptorSetKHR)(
                    command,
                    VK_PIPELINE_BIND_POINT_GRAPHICS,
                    pipeline.layout,
                    0,
                    1,
                    &w,
                );
                bound = Some(view);
            }
            let [x, y, w, h] = batch.scissor;
            if w == 0 || h == 0 || batch.count == 0 {
                continue;
            }
            let scissor = VkRect2D {
                offset: VkOffset2D {
                    x: x as i32,
                    y: y as i32,
                },
                extent: VkExtent2D {
                    width: w,
                    height: h,
                },
            };
            (gpu.fns.vkCmdSetScissor)(command, 0, 1, &scissor);
            (gpu.fns.vkCmdDraw)(command, 4, batch.count, 0, batch.start);
        }
        (gpu.fns.vkCmdEndRendering)(command);
    }
}
