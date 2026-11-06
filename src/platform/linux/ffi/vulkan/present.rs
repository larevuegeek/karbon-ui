//! Structures Vulkan : surfaces Wayland et XCB, chaîne d'échange (`VK_KHR_wayland_surface`,
//! `VK_KHR_xcb_surface`, `VK_KHR_swapchain`).
//! Transcrites de `vulkan_core.h`, `vulkan_wayland.h` et `vulkan_xcb.h` (Vulkan 1.3) ; les assertions
//! de taille et d'alignement viennent du compilateur C sur les mêmes en-têtes.
#![allow(non_snake_case, dead_code)]
use super::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkWaylandSurfaceCreateInfoKHR {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub display: *mut c_void,
    pub surface: *mut c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkXcbSurfaceCreateInfoKHR {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub connection: *mut c_void,
    pub window: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSurfaceCapabilitiesKHR {
    pub minImageCount: u32,
    pub maxImageCount: u32,
    pub currentExtent: VkExtent2D,
    pub minImageExtent: VkExtent2D,
    pub maxImageExtent: VkExtent2D,
    pub maxImageArrayLayers: u32,
    pub supportedTransforms: u32,
    pub currentTransform: u32,
    pub supportedCompositeAlpha: u32,
    pub supportedUsageFlags: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSurfaceFormatKHR {
    pub format: i32,
    pub colorSpace: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkSwapchainCreateInfoKHR {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub surface: u64,
    pub minImageCount: u32,
    pub imageFormat: i32,
    pub imageColorSpace: i32,
    pub imageExtent: VkExtent2D,
    pub imageArrayLayers: u32,
    pub imageUsage: u32,
    pub imageSharingMode: i32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
    pub preTransform: u32,
    pub compositeAlpha: u32,
    pub presentMode: i32,
    pub clipped: u32,
    pub oldSwapchain: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPresentInfoKHR {
    pub sType: i32,
    pub pNext: *const c_void,
    pub waitSemaphoreCount: u32,
    pub pWaitSemaphores: *const u64,
    pub swapchainCount: u32,
    pub pSwapchains: *const u64,
    pub pImageIndices: *const u32,
    pub pResults: *mut i32,
}
const _: () = assert!(
    size_of::<VkWaylandSurfaceCreateInfoKHR>() == 40
        && align_of::<VkWaylandSurfaceCreateInfoKHR>() == 8
);
const _: () = assert!(
    size_of::<VkXcbSurfaceCreateInfoKHR>() == 40 && align_of::<VkXcbSurfaceCreateInfoKHR>() == 8
);
const _: () = assert!(
    size_of::<VkSurfaceCapabilitiesKHR>() == 52 && align_of::<VkSurfaceCapabilitiesKHR>() == 4
);
const _: () =
    assert!(size_of::<VkSurfaceFormatKHR>() == 8 && align_of::<VkSurfaceFormatKHR>() == 4);
const _: () = assert!(
    size_of::<VkSwapchainCreateInfoKHR>() == 104 && align_of::<VkSwapchainCreateInfoKHR>() == 8
);
const _: () = assert!(size_of::<VkPresentInfoKHR>() == 64 && align_of::<VkPresentInfoKHR>() == 8);
