//! Pipeline des quads instanciés : shaders SPIR-V embarqués (`shaders/`, compilés par
//! `shaders/build.sh`), disposition à descripteurs poussés (instances, arrêts des
//! dégradés, maillons des découpes, texture), constantes de push du repère de la cible,
//! mélange prémultiplié « source-over », rendu dynamique vers le format de la cible.
use super::device::{Gpu, check};
use crate::platform::linux::ffi::vulkan::*;
use std::rc::Rc;

const VERTEX: &[u8] = include_bytes!("shaders/quad.vert.spv");
const FRAGMENT: &[u8] = include_bytes!("shaders/quad.frag.spv");
/// Taille des constantes de push : `Frame` (coin et taille de la cible).
pub(super) const FRAME_BYTES: u32 = 16;

/// Module de shader d'un binaire SPIR-V (mots de 32 bits petit-boutistes).
pub(super) fn module(gpu: &Gpu, spirv: &[u8]) -> crate::Result<u64> {
    let words = spirv
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect::<Vec<_>>();
    let info = VkShaderModuleCreateInfo {
        sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        codeSize: words.len() * 4,
        pCode: words.as_ptr(),
    };
    let mut module = 0;
    // SAFETY : appareil valide ; code lu pendant l'appel.
    check(
        unsafe { (gpu.fns.vkCreateShaderModule)(gpu.device, &info, std::ptr::null(), &mut module) },
        "vkCreateShaderModule",
    )?;
    Ok(module)
}
pub(super) struct Pipeline {
    pub set_layout: u64,
    pub layout: u64,
    pub pipeline: u64,
    /// Échantillonneur bilinéaire, bords prolongés.
    pub sampler: u64,
    gpu: Rc<Gpu>,
}
impl Pipeline {
    /// Pipeline pour une cible au format `format`.
    pub fn new(gpu: &Rc<Gpu>, format: i32) -> crate::Result<Self> {
        let mut this = Self {
            set_layout: 0,
            layout: 0,
            pipeline: 0,
            sampler: 0,
            gpu: Rc::clone(gpu),
        };
        let both = VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT;
        let binding = |index: u32, kind: i32, stages: u32| VkDescriptorSetLayoutBinding {
            binding: index,
            descriptorType: kind,
            descriptorCount: 1,
            stageFlags: stages,
            pImmutableSamplers: std::ptr::null(),
        };
        let bindings = [
            binding(0, VK_DESCRIPTOR_TYPE_STORAGE_BUFFER, both),
            binding(
                1,
                VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
                VK_SHADER_STAGE_FRAGMENT_BIT,
            ),
            binding(
                2,
                VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
                VK_SHADER_STAGE_FRAGMENT_BIT,
            ),
            binding(
                3,
                VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                VK_SHADER_STAGE_FRAGMENT_BIT,
            ),
        ];
        let set = VkDescriptorSetLayoutCreateInfo {
            sType: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: VK_DESCRIPTOR_SET_LAYOUT_CREATE_PUSH_DESCRIPTOR_BIT_KHR,
            bindingCount: bindings.len() as u32,
            pBindings: bindings.as_ptr(),
        };
        let device = gpu.device;
        // SAFETY : descriptions locales ; objets détruits par `Drop` (nuls ignorés).
        unsafe {
            check(
                (gpu.fns.vkCreateDescriptorSetLayout)(
                    device,
                    &set,
                    std::ptr::null(),
                    &mut this.set_layout,
                ),
                "vkCreateDescriptorSetLayout",
            )?;
            let push = VkPushConstantRange {
                stageFlags: both,
                offset: 0,
                size: FRAME_BYTES,
            };
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
                    device,
                    &layout,
                    std::ptr::null(),
                    &mut this.layout,
                ),
                "vkCreatePipelineLayout",
            )?;
            let sampler = VkSamplerCreateInfo {
                sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                magFilter: VK_FILTER_LINEAR,
                minFilter: VK_FILTER_LINEAR,
                mipmapMode: VK_SAMPLER_MIPMAP_MODE_LINEAR,
                addressModeU: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                addressModeV: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                addressModeW: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                mipLodBias: 0.,
                anisotropyEnable: 0,
                maxAnisotropy: 1.,
                compareEnable: 0,
                compareOp: 0,
                minLod: 0.,
                maxLod: 16.,
                borderColor: 0,
                unnormalizedCoordinates: 0,
            };
            check(
                (gpu.fns.vkCreateSampler)(device, &sampler, std::ptr::null(), &mut this.sampler),
                "vkCreateSampler",
            )?;
            this.pipeline = graphics(gpu, this.layout, (VERTEX, FRAGMENT), format, true)?;
        }
        Ok(this)
    }
}
/// Pipeline graphique de `layout` (bande de triangles sans sommets, fenêtre et ciseau
/// dynamiques) vers une cible au format `format` ; `blend` : mélange prémultiplié
/// « source-over », sinon écriture directe.
pub(super) fn graphics(
    gpu: &Gpu,
    layout: u64,
    (vertex, fragment): (&[u8], &[u8]),
    format: i32,
    blend: bool,
) -> crate::Result<u64> {
    let vertex = module(gpu, vertex)?;
    let fragment = match module(gpu, fragment) {
        Ok(fragment) => fragment,
        Err(error) => {
            // SAFETY : module créé ci-dessus, employé par aucun pipeline.
            unsafe { (gpu.fns.vkDestroyShaderModule)(gpu.device, vertex, std::ptr::null()) };
            return Err(error);
        }
    };
    let stage = |stage: u32, module: u64| VkPipelineShaderStageCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        stage,
        module,
        pName: c"main".as_ptr(),
        pSpecializationInfo: std::ptr::null(),
    };
    let stages = [
        stage(VK_SHADER_STAGE_VERTEX_BIT, vertex),
        stage(VK_SHADER_STAGE_FRAGMENT_BIT, fragment),
    ];
    // SAFETY : structures « create info » entièrement remplies ci-dessous ou
    // nulles (champs non utilisés).
    let mut vertex_input: VkPipelineVertexInputStateCreateInfo = unsafe { std::mem::zeroed() };
    vertex_input.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
    let mut assembly: VkPipelineInputAssemblyStateCreateInfo = unsafe { std::mem::zeroed() };
    assembly.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
    assembly.topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_STRIP;
    let mut viewport: VkPipelineViewportStateCreateInfo = unsafe { std::mem::zeroed() };
    viewport.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
    viewport.viewportCount = 1;
    viewport.scissorCount = 1;
    let mut raster: VkPipelineRasterizationStateCreateInfo = unsafe { std::mem::zeroed() };
    raster.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
    raster.polygonMode = VK_POLYGON_MODE_FILL;
    raster.cullMode = VK_CULL_MODE_NONE;
    raster.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE;
    raster.lineWidth = 1.;
    let mut multisample: VkPipelineMultisampleStateCreateInfo = unsafe { std::mem::zeroed() };
    multisample.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
    multisample.rasterizationSamples = VK_SAMPLE_COUNT_1_BIT;
    // Couleurs prémultipliées : source + destination × (1 − alpha source).
    let attachment = VkPipelineColorBlendAttachmentState {
        blendEnable: u32::from(blend),
        srcColorBlendFactor: VK_BLEND_FACTOR_ONE,
        dstColorBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
        colorBlendOp: VK_BLEND_OP_ADD,
        srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
        dstAlphaBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
        alphaBlendOp: VK_BLEND_OP_ADD,
        colorWriteMask: VK_COLOR_COMPONENT_R_BIT
            | VK_COLOR_COMPONENT_G_BIT
            | VK_COLOR_COMPONENT_B_BIT
            | VK_COLOR_COMPONENT_A_BIT,
    };
    let mut blend: VkPipelineColorBlendStateCreateInfo = unsafe { std::mem::zeroed() };
    blend.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
    blend.attachmentCount = 1;
    blend.pAttachments = &attachment;
    let dynamics = [VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR];
    let mut dynamic: VkPipelineDynamicStateCreateInfo = unsafe { std::mem::zeroed() };
    dynamic.sType = VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
    dynamic.dynamicStateCount = dynamics.len() as u32;
    dynamic.pDynamicStates = dynamics.as_ptr();
    let mut rendering: VkPipelineRenderingCreateInfo = unsafe { std::mem::zeroed() };
    rendering.sType = VK_STRUCTURE_TYPE_PIPELINE_RENDERING_CREATE_INFO;
    rendering.colorAttachmentCount = 1;
    rendering.pColorAttachmentFormats = &format;
    let mut info: VkGraphicsPipelineCreateInfo = unsafe { std::mem::zeroed() };
    info.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
    info.pNext = (&raw const rendering).cast();
    info.stageCount = stages.len() as u32;
    info.pStages = stages.as_ptr();
    info.pVertexInputState = &vertex_input;
    info.pInputAssemblyState = &assembly;
    info.pViewportState = &viewport;
    info.pRasterizationState = &raster;
    info.pMultisampleState = &multisample;
    info.pColorBlendState = &blend;
    info.pDynamicState = &dynamic;
    info.layout = layout;
    info.basePipelineIndex = -1;
    let mut pipeline = 0;
    // SAFETY : descriptions locales valides pendant l'appel ; modules détruits après.
    let result = unsafe {
        let result = (gpu.fns.vkCreateGraphicsPipelines)(
            gpu.device,
            0,
            1,
            &info,
            std::ptr::null(),
            &mut pipeline,
        );
        (gpu.fns.vkDestroyShaderModule)(gpu.device, vertex, std::ptr::null());
        (gpu.fns.vkDestroyShaderModule)(gpu.device, fragment, std::ptr::null());
        result
    };
    check(result, "vkCreateGraphicsPipelines")?;
    Ok(pipeline)
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        let gpu = &self.gpu;
        // SAFETY : objets possédés, plus utilisés par le GPU ; les nuls sont ignorés.
        unsafe {
            (gpu.fns.vkDestroyPipeline)(gpu.device, self.pipeline, std::ptr::null());
            (gpu.fns.vkDestroySampler)(gpu.device, self.sampler, std::ptr::null());
            (gpu.fns.vkDestroyPipelineLayout)(gpu.device, self.layout, std::ptr::null());
            (gpu.fns.vkDestroyDescriptorSetLayout)(gpu.device, self.set_layout, std::ptr::null());
        }
    }
}
