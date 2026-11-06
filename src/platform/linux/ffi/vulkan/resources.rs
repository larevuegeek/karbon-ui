//! Structures Vulkan : Images, vues, échantillonneurs, tampons et mémoire.
//! Transcrites de `vulkan_core.h` et `vulkan_wayland.h` (Vulkan 1.3) ; les assertions
//! de taille et d'alignement viennent du compilateur C sur les mêmes en-têtes.
#![allow(non_snake_case, dead_code)]
use super::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub imageType: i32,
    pub format: i32,
    pub extent: VkExtent3D,
    pub mipLevels: u32,
    pub arrayLayers: u32,
    pub samples: u32,
    pub tiling: i32,
    pub usage: u32,
    pub sharingMode: i32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
    pub initialLayout: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkComponentMapping {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageSubresourceRange {
    pub aspectMask: u32,
    pub baseMipLevel: u32,
    pub levelCount: u32,
    pub baseArrayLayer: u32,
    pub layerCount: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageSubresourceLayers {
    pub aspectMask: u32,
    pub mipLevel: u32,
    pub baseArrayLayer: u32,
    pub layerCount: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageViewCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub image: u64,
    pub viewType: i32,
    pub format: i32,
    pub components: VkComponentMapping,
    pub subresourceRange: VkImageSubresourceRange,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryRequirements {
    pub size: u64,
    pub alignment: u64,
    pub memoryTypeBits: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryAllocateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub allocationSize: u64,
    pub memoryTypeIndex: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSamplerCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub magFilter: i32,
    pub minFilter: i32,
    pub mipmapMode: i32,
    pub addressModeU: i32,
    pub addressModeV: i32,
    pub addressModeW: i32,
    pub mipLodBias: f32,
    pub anisotropyEnable: u32,
    pub maxAnisotropy: f32,
    pub compareEnable: u32,
    pub compareOp: i32,
    pub minLod: f32,
    pub maxLod: f32,
    pub borderColor: i32,
    pub unnormalizedCoordinates: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkBufferCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub size: u64,
    pub usage: u32,
    pub sharingMode: i32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
}
const _: () = assert!(size_of::<VkImageCreateInfo>() == 88 && align_of::<VkImageCreateInfo>() == 8);
const _: () =
    assert!(size_of::<VkComponentMapping>() == 16 && align_of::<VkComponentMapping>() == 4);
const _: () = assert!(
    size_of::<VkImageSubresourceRange>() == 20 && align_of::<VkImageSubresourceRange>() == 4
);
const _: () = assert!(
    size_of::<VkImageSubresourceLayers>() == 16 && align_of::<VkImageSubresourceLayers>() == 4
);
const _: () =
    assert!(size_of::<VkImageViewCreateInfo>() == 80 && align_of::<VkImageViewCreateInfo>() == 8);
const _: () =
    assert!(size_of::<VkMemoryRequirements>() == 24 && align_of::<VkMemoryRequirements>() == 8);
const _: () =
    assert!(size_of::<VkMemoryAllocateInfo>() == 32 && align_of::<VkMemoryAllocateInfo>() == 8);
const _: () =
    assert!(size_of::<VkSamplerCreateInfo>() == 80 && align_of::<VkSamplerCreateInfo>() == 8);
const _: () =
    assert!(size_of::<VkBufferCreateInfo>() == 56 && align_of::<VkBufferCreateInfo>() == 8);
