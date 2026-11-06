//! Structures Vulkan : Tampons de commandes, rendu, barrières (synchronization2), copies, descripteurs poussés, soumission et synchronisation.
//! Transcrites de `vulkan_core.h` et `vulkan_wayland.h` (Vulkan 1.3) ; les assertions
//! de taille et d'alignement viennent du compilateur C sur les mêmes en-têtes.
#![allow(non_snake_case, dead_code)]
use super::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkCommandPoolCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub queueFamilyIndex: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkCommandBufferAllocateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub commandPool: u64,
    pub level: i32,
    pub commandBufferCount: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkCommandBufferInheritanceInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub renderPass: u64,
    pub subpass: u32,
    pub framebuffer: u64,
    pub occlusionQueryEnable: u32,
    pub queryFlags: u32,
    pub pipelineStatistics: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkCommandBufferBeginInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub pInheritanceInfo: *const VkCommandBufferInheritanceInfo,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union VkClearColorValue {
    pub float32: [f32; 4],
    pub int32: [i32; 4],
    pub uint32: [u32; 4],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkClearDepthStencilValue {
    pub depth: f32,
    pub stencil: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union VkClearValue {
    pub color: VkClearColorValue,
    pub depthStencil: VkClearDepthStencilValue,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkRenderingAttachmentInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub imageView: u64,
    pub imageLayout: i32,
    pub resolveMode: u32,
    pub resolveImageView: u64,
    pub resolveImageLayout: i32,
    pub loadOp: i32,
    pub storeOp: i32,
    pub clearValue: VkClearValue,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkRenderingInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub renderArea: VkRect2D,
    pub layerCount: u32,
    pub viewMask: u32,
    pub colorAttachmentCount: u32,
    pub pColorAttachments: *const VkRenderingAttachmentInfo,
    pub pDepthAttachment: *const VkRenderingAttachmentInfo,
    pub pStencilAttachment: *const VkRenderingAttachmentInfo,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryBarrier2 {
    pub sType: i32,
    pub pNext: *const c_void,
    pub srcStageMask: u64,
    pub srcAccessMask: u64,
    pub dstStageMask: u64,
    pub dstAccessMask: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkBufferMemoryBarrier2 {
    pub sType: i32,
    pub pNext: *const c_void,
    pub srcStageMask: u64,
    pub srcAccessMask: u64,
    pub dstStageMask: u64,
    pub dstAccessMask: u64,
    pub srcQueueFamilyIndex: u32,
    pub dstQueueFamilyIndex: u32,
    pub buffer: u64,
    pub offset: u64,
    pub size: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageMemoryBarrier2 {
    pub sType: i32,
    pub pNext: *const c_void,
    pub srcStageMask: u64,
    pub srcAccessMask: u64,
    pub dstStageMask: u64,
    pub dstAccessMask: u64,
    pub oldLayout: i32,
    pub newLayout: i32,
    pub srcQueueFamilyIndex: u32,
    pub dstQueueFamilyIndex: u32,
    pub image: u64,
    pub subresourceRange: VkImageSubresourceRange,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDependencyInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub dependencyFlags: u32,
    pub memoryBarrierCount: u32,
    pub pMemoryBarriers: *const VkMemoryBarrier2,
    pub bufferMemoryBarrierCount: u32,
    pub pBufferMemoryBarriers: *const VkBufferMemoryBarrier2,
    pub imageMemoryBarrierCount: u32,
    pub pImageMemoryBarriers: *const VkImageMemoryBarrier2,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkBufferImageCopy {
    pub bufferOffset: u64,
    pub bufferRowLength: u32,
    pub bufferImageHeight: u32,
    pub imageSubresource: VkImageSubresourceLayers,
    pub imageOffset: VkOffset3D,
    pub imageExtent: VkExtent3D,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageBlit {
    pub srcSubresource: VkImageSubresourceLayers,
    pub srcOffsets: [VkOffset3D; 2],
    pub dstSubresource: VkImageSubresourceLayers,
    pub dstOffsets: [VkOffset3D; 2],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDescriptorImageInfo {
    pub sampler: u64,
    pub imageView: u64,
    pub imageLayout: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDescriptorBufferInfo {
    pub buffer: u64,
    pub offset: u64,
    pub range: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkWriteDescriptorSet {
    pub sType: i32,
    pub pNext: *const c_void,
    pub dstSet: u64,
    pub dstBinding: u32,
    pub dstArrayElement: u32,
    pub descriptorCount: u32,
    pub descriptorType: i32,
    pub pImageInfo: *const VkDescriptorImageInfo,
    pub pBufferInfo: *const VkDescriptorBufferInfo,
    pub pTexelBufferView: *const u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSubmitInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub waitSemaphoreCount: u32,
    pub pWaitSemaphores: *const u64,
    pub pWaitDstStageMask: *const u32,
    pub commandBufferCount: u32,
    pub pCommandBuffers: *const *mut c_void,
    pub signalSemaphoreCount: u32,
    pub pSignalSemaphores: *const u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkFenceCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSemaphoreCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
}
const _: () = assert!(
    size_of::<VkCommandPoolCreateInfo>() == 24 && align_of::<VkCommandPoolCreateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkCommandBufferAllocateInfo>() == 32
        && align_of::<VkCommandBufferAllocateInfo>() == 8
);
const _: () = assert!(
    size_of::<VkCommandBufferInheritanceInfo>() == 56
        && align_of::<VkCommandBufferInheritanceInfo>() == 8
);
const _: () = assert!(
    size_of::<VkCommandBufferBeginInfo>() == 32 && align_of::<VkCommandBufferBeginInfo>() == 8
);
const _: () = assert!(size_of::<VkClearColorValue>() == 16 && align_of::<VkClearColorValue>() == 4);
const _: () = assert!(
    size_of::<VkClearDepthStencilValue>() == 8 && align_of::<VkClearDepthStencilValue>() == 4
);
const _: () = assert!(size_of::<VkClearValue>() == 16 && align_of::<VkClearValue>() == 4);
const _: () = assert!(
    size_of::<VkRenderingAttachmentInfo>() == 72 && align_of::<VkRenderingAttachmentInfo>() == 8
);
const _: () = assert!(size_of::<VkRenderingInfo>() == 72 && align_of::<VkRenderingInfo>() == 8);
const _: () = assert!(size_of::<VkMemoryBarrier2>() == 48 && align_of::<VkMemoryBarrier2>() == 8);
const _: () =
    assert!(size_of::<VkBufferMemoryBarrier2>() == 80 && align_of::<VkBufferMemoryBarrier2>() == 8);
const _: () =
    assert!(size_of::<VkImageMemoryBarrier2>() == 96 && align_of::<VkImageMemoryBarrier2>() == 8);
const _: () = assert!(size_of::<VkDependencyInfo>() == 64 && align_of::<VkDependencyInfo>() == 8);
const _: () = assert!(size_of::<VkBufferImageCopy>() == 56 && align_of::<VkBufferImageCopy>() == 8);
const _: () = assert!(size_of::<VkImageBlit>() == 80 && align_of::<VkImageBlit>() == 4);
const _: () =
    assert!(size_of::<VkDescriptorImageInfo>() == 24 && align_of::<VkDescriptorImageInfo>() == 8);
const _: () =
    assert!(size_of::<VkDescriptorBufferInfo>() == 24 && align_of::<VkDescriptorBufferInfo>() == 8);
const _: () =
    assert!(size_of::<VkWriteDescriptorSet>() == 64 && align_of::<VkWriteDescriptorSet>() == 8);
const _: () = assert!(size_of::<VkSubmitInfo>() == 72 && align_of::<VkSubmitInfo>() == 8);
const _: () = assert!(size_of::<VkFenceCreateInfo>() == 24 && align_of::<VkFenceCreateInfo>() == 8);
const _: () =
    assert!(size_of::<VkSemaphoreCreateInfo>() == 24 && align_of::<VkSemaphoreCreateInfo>() == 8);
