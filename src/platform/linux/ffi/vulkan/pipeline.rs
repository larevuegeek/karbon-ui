//! Structures Vulkan : Shaders, descripteurs et pipeline graphique (rendu dynamique).
//! Transcrites de `vulkan_core.h` et `vulkan_wayland.h` (Vulkan 1.3) ; les assertions
//! de taille et d'alignement viennent du compilateur C sur les mêmes en-têtes.
#![allow(non_snake_case, dead_code)]
use super::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkShaderModuleCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub codeSize: usize,
    pub pCode: *const u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptorType: i32,
    pub descriptorCount: u32,
    pub stageFlags: u32,
    pub pImmutableSamplers: *const u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDescriptorSetLayoutCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub bindingCount: u32,
    pub pBindings: *const VkDescriptorSetLayoutBinding,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPushConstantRange {
    pub stageFlags: u32,
    pub offset: u32,
    pub size: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineLayoutCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub setLayoutCount: u32,
    pub pSetLayouts: *const u64,
    pub pushConstantRangeCount: u32,
    pub pPushConstantRanges: *const VkPushConstantRange,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSpecializationMapEntry {
    pub constantID: u32,
    pub offset: u32,
    pub size: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSpecializationInfo {
    pub mapEntryCount: u32,
    pub pMapEntries: *const VkSpecializationMapEntry,
    pub dataSize: usize,
    pub pData: *const c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineShaderStageCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub stage: u32,
    pub module: u64,
    pub pName: *const c_char,
    pub pSpecializationInfo: *const VkSpecializationInfo,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkVertexInputBindingDescription {
    pub binding: u32,
    pub stride: u32,
    pub inputRate: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkVertexInputAttributeDescription {
    pub location: u32,
    pub binding: u32,
    pub format: i32,
    pub offset: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineVertexInputStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub vertexBindingDescriptionCount: u32,
    pub pVertexBindingDescriptions: *const VkVertexInputBindingDescription,
    pub vertexAttributeDescriptionCount: u32,
    pub pVertexAttributeDescriptions: *const VkVertexInputAttributeDescription,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineInputAssemblyStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub topology: i32,
    pub primitiveRestartEnable: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineTessellationStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub patchControlPoints: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub minDepth: f32,
    pub maxDepth: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineViewportStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub viewportCount: u32,
    pub pViewports: *const VkViewport,
    pub scissorCount: u32,
    pub pScissors: *const VkRect2D,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineRasterizationStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub depthClampEnable: u32,
    pub rasterizerDiscardEnable: u32,
    pub polygonMode: i32,
    pub cullMode: u32,
    pub frontFace: i32,
    pub depthBiasEnable: u32,
    pub depthBiasConstantFactor: f32,
    pub depthBiasClamp: f32,
    pub depthBiasSlopeFactor: f32,
    pub lineWidth: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineMultisampleStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub rasterizationSamples: u32,
    pub sampleShadingEnable: u32,
    pub minSampleShading: f32,
    pub pSampleMask: *const u32,
    pub alphaToCoverageEnable: u32,
    pub alphaToOneEnable: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkStencilOpState {
    pub failOp: i32,
    pub passOp: i32,
    pub depthFailOp: i32,
    pub compareOp: i32,
    pub compareMask: u32,
    pub writeMask: u32,
    pub reference: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineDepthStencilStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub depthTestEnable: u32,
    pub depthWriteEnable: u32,
    pub depthCompareOp: i32,
    pub depthBoundsTestEnable: u32,
    pub stencilTestEnable: u32,
    pub front: VkStencilOpState,
    pub back: VkStencilOpState,
    pub minDepthBounds: f32,
    pub maxDepthBounds: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineColorBlendAttachmentState {
    pub blendEnable: u32,
    pub srcColorBlendFactor: i32,
    pub dstColorBlendFactor: i32,
    pub colorBlendOp: i32,
    pub srcAlphaBlendFactor: i32,
    pub dstAlphaBlendFactor: i32,
    pub alphaBlendOp: i32,
    pub colorWriteMask: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineColorBlendStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub logicOpEnable: u32,
    pub logicOp: i32,
    pub attachmentCount: u32,
    pub pAttachments: *const VkPipelineColorBlendAttachmentState,
    pub blendConstants: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineDynamicStateCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub dynamicStateCount: u32,
    pub pDynamicStates: *const i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPipelineRenderingCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub viewMask: u32,
    pub colorAttachmentCount: u32,
    pub pColorAttachmentFormats: *const i32,
    pub depthAttachmentFormat: i32,
    pub stencilAttachmentFormat: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkGraphicsPipelineCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub stageCount: u32,
    pub pStages: *const VkPipelineShaderStageCreateInfo,
    pub pVertexInputState: *const VkPipelineVertexInputStateCreateInfo,
    pub pInputAssemblyState: *const VkPipelineInputAssemblyStateCreateInfo,
    pub pTessellationState: *const VkPipelineTessellationStateCreateInfo,
    pub pViewportState: *const VkPipelineViewportStateCreateInfo,
    pub pRasterizationState: *const VkPipelineRasterizationStateCreateInfo,
    pub pMultisampleState: *const VkPipelineMultisampleStateCreateInfo,
    pub pDepthStencilState: *const VkPipelineDepthStencilStateCreateInfo,
    pub pColorBlendState: *const VkPipelineColorBlendStateCreateInfo,
    pub pDynamicState: *const VkPipelineDynamicStateCreateInfo,
    pub layout: u64,
    pub renderPass: u64,
    pub subpass: u32,
    pub basePipelineHandle: u64,
    pub basePipelineIndex: i32,
}
const _: () = assert!(
    size_of::<VkShaderModuleCreateInfo>() == 40 && align_of::<VkShaderModuleCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkDescriptorSetLayoutBinding>() == 24
        && align_of::<VkDescriptorSetLayoutBinding>() == 8
);
const _: () = assert!(
    size_of::<VkDescriptorSetLayoutCreateInfo>() == 32
        && align_of::<VkDescriptorSetLayoutCreateInfo>() == 8
);
const _: () =
    assert!(size_of::<VkPushConstantRange>() == 12 && align_of::<VkPushConstantRange>() == 4);
const _: () = assert!(
    size_of::<VkPipelineLayoutCreateInfo>() == 48 && align_of::<VkPipelineLayoutCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkSpecializationMapEntry>() == 16 && align_of::<VkSpecializationMapEntry>() == 8
);
const _: () =
    assert!(size_of::<VkSpecializationInfo>() == 32 && align_of::<VkSpecializationInfo>() == 8);
const _: () = assert!(
    size_of::<VkPipelineShaderStageCreateInfo>() == 48
        && align_of::<VkPipelineShaderStageCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkVertexInputBindingDescription>() == 12
        && align_of::<VkVertexInputBindingDescription>() == 4
);
const _: () = assert!(
    size_of::<VkVertexInputAttributeDescription>() == 16
        && align_of::<VkVertexInputAttributeDescription>() == 4
);
const _: () = assert!(
    size_of::<VkPipelineVertexInputStateCreateInfo>() == 48
        && align_of::<VkPipelineVertexInputStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineInputAssemblyStateCreateInfo>() == 32
        && align_of::<VkPipelineInputAssemblyStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineTessellationStateCreateInfo>() == 24
        && align_of::<VkPipelineTessellationStateCreateInfo>() == 8
);
const _: () = assert!(size_of::<VkViewport>() == 24 && align_of::<VkViewport>() == 4);
const _: () = assert!(
    size_of::<VkPipelineViewportStateCreateInfo>() == 48
        && align_of::<VkPipelineViewportStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineRasterizationStateCreateInfo>() == 64
        && align_of::<VkPipelineRasterizationStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineMultisampleStateCreateInfo>() == 48
        && align_of::<VkPipelineMultisampleStateCreateInfo>() == 8
);
const _: () = assert!(size_of::<VkStencilOpState>() == 28 && align_of::<VkStencilOpState>() == 4);
const _: () = assert!(
    size_of::<VkPipelineDepthStencilStateCreateInfo>() == 104
        && align_of::<VkPipelineDepthStencilStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineColorBlendAttachmentState>() == 32
        && align_of::<VkPipelineColorBlendAttachmentState>() == 4
);
const _: () = assert!(
    size_of::<VkPipelineColorBlendStateCreateInfo>() == 56
        && align_of::<VkPipelineColorBlendStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineDynamicStateCreateInfo>() == 32
        && align_of::<VkPipelineDynamicStateCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkPipelineRenderingCreateInfo>() == 40
        && align_of::<VkPipelineRenderingCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkGraphicsPipelineCreateInfo>() == 144
        && align_of::<VkGraphicsPipelineCreateInfo>() == 8
);
