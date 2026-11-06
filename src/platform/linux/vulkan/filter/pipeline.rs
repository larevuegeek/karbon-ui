//! Pipeline des passages de filtre (`shaders/filter.vert`, `shaders/filter.frag` : cible
//! RGBA 16 bits flottants, sans mélange) et enregistrement d'un passage.
use super::Kernel;
use crate::platform::linux::{
    ffi::vulkan::*,
    vulkan::{
        device::{Gpu, check},
        memory::{Buffer, Image},
        pipeline::graphics,
    },
};
use std::{ffi::c_void, rc::Rc};

/// Format des cibles des filtres.
pub(in crate::platform::linux::vulkan) const FORMAT: i32 = VK_FORMAT_R16G16B16A16_SFLOAT;
const VERTEX: &[u8] = include_bytes!("../shaders/filter.vert.spv");
const FRAGMENT: &[u8] = include_bytes!("../shaders/filter.frag.spv");
/// Constantes de push : rang des paramètres, calcul.
const PUSH_BYTES: u32 = 8;

pub(in crate::platform::linux::vulkan) struct Filters {
    set_layout: u64,
    layout: u64,
    pipeline: u64,
    gpu: Rc<Gpu>,
}
impl Filters {
    pub fn new(gpu: &Rc<Gpu>) -> crate::Result<Self> {
        let mut this = Self {
            set_layout: 0,
            layout: 0,
            pipeline: 0,
            gpu: Rc::clone(gpu),
        };
        let binding = |index: u32, kind: i32| VkDescriptorSetLayoutBinding {
            binding: index,
            descriptorType: kind,
            descriptorCount: 1,
            stageFlags: VK_SHADER_STAGE_FRAGMENT_BIT,
            pImmutableSamplers: std::ptr::null(),
        };
        let bindings = [
            binding(0, VK_DESCRIPTOR_TYPE_STORAGE_BUFFER),
            binding(1, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
            binding(2, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
        ];
        let set = VkDescriptorSetLayoutCreateInfo {
            sType: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: VK_DESCRIPTOR_SET_LAYOUT_CREATE_PUSH_DESCRIPTOR_BIT_KHR,
            bindingCount: bindings.len() as u32,
            pBindings: bindings.as_ptr(),
        };
        let push = VkPushConstantRange {
            stageFlags: VK_SHADER_STAGE_FRAGMENT_BIT,
            offset: 0,
            size: PUSH_BYTES,
        };
        // SAFETY : descriptions locales ; objets détruits par `Drop` (nuls ignorés).
        unsafe {
            check(
                (gpu.fns.vkCreateDescriptorSetLayout)(
                    gpu.device,
                    &set,
                    std::ptr::null(),
                    &mut this.set_layout,
                ),
                "vkCreateDescriptorSetLayout",
            )?;
            let layout = VkPipelineLayoutCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                setLayoutCount: 1,
                pSetLayouts: &this.set_layout,
                pushConstantRangeCount: 1,
                pPushConstantRanges: &push,
            };
            check(
                (gpu.fns.vkCreatePipelineLayout)(
                    gpu.device,
                    &layout,
                    std::ptr::null(),
                    &mut this.layout,
                ),
                "vkCreatePipelineLayout",
            )?;
        }
        this.pipeline = graphics(gpu, this.layout, (VERTEX, FRAGMENT), FORMAT, false)?;
        Ok(this)
    }
}
impl Drop for Filters {
    fn drop(&mut self) {
        let gpu = &self.gpu;
        // SAFETY : objets possédés, plus utilisés par le GPU ; les nuls sont ignorés.
        unsafe {
            (gpu.fns.vkDestroyPipeline)(gpu.device, self.pipeline, std::ptr::null());
            (gpu.fns.vkDestroyPipelineLayout)(gpu.device, self.layout, std::ptr::null());
            (gpu.fns.vkDestroyDescriptorSetLayout)(gpu.device, self.set_layout, std::ptr::null());
        }
    }
}
/// Passage à enregistrer : cible (disposition « attachement »), zone utile, calcul,
/// paramètres (rang `index` du tampon `params`) et vues lues (source, dessous).
pub(in crate::platform::linux::vulkan) struct Record<'a> {
    pub target: &'a Image,
    pub size: [usize; 2],
    pub kernel: Kernel,
    pub index: usize,
    pub params: &'a Buffer,
    pub views: [u64; 2],
    pub sampler: u64,
}
/// Enregistre `pass` : la cible est effacée en entier (au-delà de la zone utile, elle
/// reste transparente pour les échantillonnages de la recomposition), puis la zone utile
/// est calculée.
pub(in crate::platform::linux::vulkan) fn record(
    gpu: &Gpu,
    command: *mut c_void,
    pipeline: &Filters,
    pass: Record<'_>,
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
            color: VkClearColorValue { float32: [0.; 4] },
        },
    };
    // SAFETY : informations de rendu entièrement remplies (pas de profondeur).
    let mut rendering: VkRenderingInfo = unsafe { std::mem::zeroed() };
    rendering.sType = VK_STRUCTURE_TYPE_RENDERING_INFO;
    rendering.renderArea = VkRect2D {
        offset: VkOffset2D { x: 0, y: 0 },
        extent: VkExtent2D { width, height },
    };
    rendering.layerCount = 1;
    rendering.colorAttachmentCount = 1;
    rendering.pColorAttachments = &attachment;
    let (w, h) = (pass.size[0] as u32, pass.size[1] as u32);
    let viewport = VkViewport {
        x: 0.,
        y: 0.,
        width: w as f32,
        height: h as f32,
        minDepth: 0.,
        maxDepth: 1.,
    };
    let scissor = VkRect2D {
        offset: VkOffset2D { x: 0, y: 0 },
        extent: VkExtent2D {
            width: w,
            height: h,
        },
    };
    let buffer = VkDescriptorBufferInfo {
        buffer: pass.params.raw,
        offset: 0,
        range: pass.params.size,
    };
    let images = pass.views.map(|view| VkDescriptorImageInfo {
        sampler: pass.sampler,
        imageView: view,
        imageLayout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
    });
    let write = |binding: u32, kind: i32| {
        // SAFETY : écriture de descripteur entièrement remplie ci-dessous.
        let mut w: VkWriteDescriptorSet = unsafe { std::mem::zeroed() };
        w.sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET;
        w.dstBinding = binding;
        w.descriptorCount = 1;
        w.descriptorType = kind;
        w
    };
    let mut writes = [
        write(0, VK_DESCRIPTOR_TYPE_STORAGE_BUFFER),
        write(1, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
        write(2, VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
    ];
    writes[0].pBufferInfo = &buffer;
    writes[1].pImageInfo = &images[0];
    writes[2].pImageInfo = &images[1];
    let push = [pass.index as u32, pass.kernel as u32];
    // SAFETY : tampon en cours d'enregistrement ; objets valides pendant l'exécution.
    unsafe {
        (gpu.fns.vkCmdBeginRendering)(command, &rendering);
        (gpu.fns.vkCmdBindPipeline)(command, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline.pipeline);
        (gpu.fns.vkCmdSetViewport)(command, 0, 1, &viewport);
        (gpu.fns.vkCmdSetScissor)(command, 0, 1, &scissor);
        (gpu.fns.vkCmdPushConstants)(
            command,
            pipeline.layout,
            VK_SHADER_STAGE_FRAGMENT_BIT,
            0,
            PUSH_BYTES,
            push.as_ptr().cast(),
        );
        (gpu.fns.vkCmdPushDescriptorSetKHR)(
            command,
            VK_PIPELINE_BIND_POINT_GRAPHICS,
            pipeline.layout,
            0,
            writes.len() as u32,
            writes.as_ptr(),
        );
        (gpu.fns.vkCmdDraw)(command, 4, 1, 0, 0);
        (gpu.fns.vkCmdEndRendering)(command);
    }
}
