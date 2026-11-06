//! Vulkan, chargé au lancement (`dlopen` de `libvulkan.so.1`, le chargeur de Khronos) :
//! aucun paquet de développement n'est nécessaire pour compiler, et l'absence de Vulkan
//! n'empêche pas l'application de démarrer (le renderer Cairo prend le relais).
//!
//! Les fonctions sont résolues par `vkGetInstanceProcAddr` et `vkGetDeviceProcAddr` en
//! tables (point d'entrée, instance, surface Wayland, appareil, chaîne d'échange) ; les
//! types sont transcrits des en-têtes (`device.rs`, `present.rs`, `resources.rs`,
//! `pipeline.rs`, `commands.rs`, `consts.rs`).
#![allow(non_snake_case)]
// Tables complètes pour toutes les étapes du renderer (chaîne d'échange, surfaces,
// envois de textures) : certaines fonctions ne servent pas encore.
#![allow(dead_code)]
mod commands;
mod consts;
mod device;
mod pipeline;
mod present;
mod resources;

pub(in crate::platform::linux) use commands::*;
pub(in crate::platform::linux) use consts::*;
pub(in crate::platform::linux) use device::*;
pub(in crate::platform::linux) use pipeline::*;
pub(in crate::platform::linux) use present::*;
pub(in crate::platform::linux) use resources::*;
pub(in crate::platform::linux) use std::ffi::{c_char, c_void};

/// Table de fonctions résolues par nom ; `None` si l'une manque.
macro_rules! table {
    ($name:ident { $( $f:ident ( $( $a:ident : $t:ty ),* $(,)? ) $( -> $r:ty )? ; )* }) => {
        pub(in crate::platform::linux) struct $name {
            $( pub $f: unsafe extern "system" fn( $( $t ),* ) $( -> $r )?, )*
        }
        impl $name {
            /// Résout chaque fonction par `load` (adresse, ou nul si absente).
            ///
            /// # Safety
            ///
            /// `load` rend l'adresse de la fonction Vulkan de ce nom, de la signature
            /// déclarée ici.
            pub unsafe fn load(mut load: impl FnMut(&std::ffi::CStr) -> *const c_void) -> Option<Self> {
                Some(Self { $( $f: {
                    let name = std::ffi::CStr::from_bytes_with_nul(concat!(stringify!($f), "\0").as_bytes()).ok()?;
                    let address = load(name);
                    if address.is_null() {
                        return None;
                    }
                    // SAFETY : adresse de la fonction de ce nom, de cette signature.
                    unsafe { std::mem::transmute::<*const c_void, unsafe extern "system" fn( $( $t ),* ) $( -> $r )?>(address) }
                }, )* })
            }
        }
    };
}
table! {
    Entry {
        vkCreateInstance(pCreateInfo: *const VkInstanceCreateInfo, pAllocator: *const VkAllocationCallbacks, pInstance: *mut *mut c_void) -> i32;
        vkEnumerateInstanceExtensionProperties(pLayerName: *const c_char, pPropertyCount: *mut u32, pProperties: *mut VkExtensionProperties) -> i32;
        vkEnumerateInstanceLayerProperties(pPropertyCount: *mut u32, pProperties: *mut VkLayerProperties) -> i32;
        vkEnumerateInstanceVersion(pApiVersion: *mut u32) -> i32;
    }
}
table! {
    InstanceFns {
        vkDestroyInstance(instance: *mut c_void, pAllocator: *const VkAllocationCallbacks);
        vkEnumeratePhysicalDevices(instance: *mut c_void, pPhysicalDeviceCount: *mut u32, pPhysicalDevices: *mut *mut c_void) -> i32;
        vkGetPhysicalDeviceProperties(physicalDevice: *mut c_void, pProperties: *mut VkPhysicalDeviceProperties);
        vkGetPhysicalDeviceQueueFamilyProperties(physicalDevice: *mut c_void, pQueueFamilyPropertyCount: *mut u32, pQueueFamilyProperties: *mut VkQueueFamilyProperties);
        vkGetPhysicalDeviceMemoryProperties(physicalDevice: *mut c_void, pMemoryProperties: *mut VkPhysicalDeviceMemoryProperties);
        vkGetPhysicalDeviceFeatures2(physicalDevice: *mut c_void, pFeatures: *mut VkPhysicalDeviceFeatures2);
        vkEnumerateDeviceExtensionProperties(physicalDevice: *mut c_void, pLayerName: *const c_char, pPropertyCount: *mut u32, pProperties: *mut VkExtensionProperties) -> i32;
        vkCreateDevice(physicalDevice: *mut c_void, pCreateInfo: *const VkDeviceCreateInfo, pAllocator: *const VkAllocationCallbacks, pDevice: *mut *mut c_void) -> i32;
        vkGetDeviceProcAddr(device: *mut c_void, pName: *const c_char) -> *const c_void;
    }
}
table! {
    SurfaceFns {
        vkDestroySurfaceKHR(instance: *mut c_void, surface: u64, pAllocator: *const VkAllocationCallbacks);
        vkGetPhysicalDeviceSurfaceSupportKHR(physicalDevice: *mut c_void, queueFamilyIndex: u32, surface: u64, pSupported: *mut u32) -> i32;
        vkGetPhysicalDeviceSurfaceCapabilitiesKHR(physicalDevice: *mut c_void, surface: u64, pSurfaceCapabilities: *mut VkSurfaceCapabilitiesKHR) -> i32;
        vkGetPhysicalDeviceSurfaceFormatsKHR(physicalDevice: *mut c_void, surface: u64, pSurfaceFormatCount: *mut u32, pSurfaceFormats: *mut VkSurfaceFormatKHR) -> i32;
        vkGetPhysicalDeviceSurfacePresentModesKHR(physicalDevice: *mut c_void, surface: u64, pPresentModeCount: *mut u32, pPresentModes: *mut i32) -> i32;
    }
}
table! {
    WaylandSurfaceFns {
        vkCreateWaylandSurfaceKHR(instance: *mut c_void, pCreateInfo: *const VkWaylandSurfaceCreateInfoKHR, pAllocator: *const VkAllocationCallbacks, pSurface: *mut u64) -> i32;
    }
}
table! {
    XcbSurfaceFns {
        vkCreateXcbSurfaceKHR(instance: *mut c_void, pCreateInfo: *const VkXcbSurfaceCreateInfoKHR, pAllocator: *const VkAllocationCallbacks, pSurface: *mut u64) -> i32;
    }
}
table! {
    DeviceFns {
        vkDestroyDevice(device: *mut c_void, pAllocator: *const VkAllocationCallbacks);
        vkGetDeviceQueue(device: *mut c_void, queueFamilyIndex: u32, queueIndex: u32, pQueue: *mut *mut c_void);
        vkDeviceWaitIdle(device: *mut c_void) -> i32;
        vkQueueSubmit(queue: *mut c_void, submitCount: u32, pSubmits: *const VkSubmitInfo, fence: u64) -> i32;
        vkQueueWaitIdle(queue: *mut c_void) -> i32;
        vkCreateImage(device: *mut c_void, pCreateInfo: *const VkImageCreateInfo, pAllocator: *const VkAllocationCallbacks, pImage: *mut u64) -> i32;
        vkDestroyImage(device: *mut c_void, image: u64, pAllocator: *const VkAllocationCallbacks);
        vkGetImageMemoryRequirements(device: *mut c_void, image: u64, pMemoryRequirements: *mut VkMemoryRequirements);
        vkBindImageMemory(device: *mut c_void, image: u64, memory: u64, memoryOffset: u64) -> i32;
        vkCreateImageView(device: *mut c_void, pCreateInfo: *const VkImageViewCreateInfo, pAllocator: *const VkAllocationCallbacks, pView: *mut u64) -> i32;
        vkDestroyImageView(device: *mut c_void, imageView: u64, pAllocator: *const VkAllocationCallbacks);
        vkCreateBuffer(device: *mut c_void, pCreateInfo: *const VkBufferCreateInfo, pAllocator: *const VkAllocationCallbacks, pBuffer: *mut u64) -> i32;
        vkDestroyBuffer(device: *mut c_void, buffer: u64, pAllocator: *const VkAllocationCallbacks);
        vkGetBufferMemoryRequirements(device: *mut c_void, buffer: u64, pMemoryRequirements: *mut VkMemoryRequirements);
        vkBindBufferMemory(device: *mut c_void, buffer: u64, memory: u64, memoryOffset: u64) -> i32;
        vkAllocateMemory(device: *mut c_void, pAllocateInfo: *const VkMemoryAllocateInfo, pAllocator: *const VkAllocationCallbacks, pMemory: *mut u64) -> i32;
        vkFreeMemory(device: *mut c_void, memory: u64, pAllocator: *const VkAllocationCallbacks);
        vkMapMemory(device: *mut c_void, memory: u64, offset: u64, size: u64, flags: u32, ppData: *mut *mut c_void) -> i32;
        vkUnmapMemory(device: *mut c_void, memory: u64);
        vkCreateSampler(device: *mut c_void, pCreateInfo: *const VkSamplerCreateInfo, pAllocator: *const VkAllocationCallbacks, pSampler: *mut u64) -> i32;
        vkDestroySampler(device: *mut c_void, sampler: u64, pAllocator: *const VkAllocationCallbacks);
        vkCreateShaderModule(device: *mut c_void, pCreateInfo: *const VkShaderModuleCreateInfo, pAllocator: *const VkAllocationCallbacks, pShaderModule: *mut u64) -> i32;
        vkDestroyShaderModule(device: *mut c_void, shaderModule: u64, pAllocator: *const VkAllocationCallbacks);
        vkCreateDescriptorSetLayout(device: *mut c_void, pCreateInfo: *const VkDescriptorSetLayoutCreateInfo, pAllocator: *const VkAllocationCallbacks, pSetLayout: *mut u64) -> i32;
        vkDestroyDescriptorSetLayout(device: *mut c_void, descriptorSetLayout: u64, pAllocator: *const VkAllocationCallbacks);
        vkCreatePipelineLayout(device: *mut c_void, pCreateInfo: *const VkPipelineLayoutCreateInfo, pAllocator: *const VkAllocationCallbacks, pPipelineLayout: *mut u64) -> i32;
        vkDestroyPipelineLayout(device: *mut c_void, pipelineLayout: u64, pAllocator: *const VkAllocationCallbacks);
        vkCreateGraphicsPipelines(device: *mut c_void, pipelineCache: u64, createInfoCount: u32, pCreateInfos: *const VkGraphicsPipelineCreateInfo, pAllocator: *const VkAllocationCallbacks, pPipelines: *mut u64) -> i32;
        vkDestroyPipeline(device: *mut c_void, pipeline: u64, pAllocator: *const VkAllocationCallbacks);
        vkCreateCommandPool(device: *mut c_void, pCreateInfo: *const VkCommandPoolCreateInfo, pAllocator: *const VkAllocationCallbacks, pCommandPool: *mut u64) -> i32;
        vkDestroyCommandPool(device: *mut c_void, commandPool: u64, pAllocator: *const VkAllocationCallbacks);
        vkAllocateCommandBuffers(device: *mut c_void, pAllocateInfo: *const VkCommandBufferAllocateInfo, pCommandBuffers: *mut *mut c_void) -> i32;
        vkResetCommandBuffer(commandBuffer: *mut c_void, flags: u32) -> i32;
        vkBeginCommandBuffer(commandBuffer: *mut c_void, pBeginInfo: *const VkCommandBufferBeginInfo) -> i32;
        vkEndCommandBuffer(commandBuffer: *mut c_void) -> i32;
        vkCmdBeginRendering(commandBuffer: *mut c_void, pRenderingInfo: *const VkRenderingInfo);
        vkCmdEndRendering(commandBuffer: *mut c_void);
        vkCmdBindPipeline(commandBuffer: *mut c_void, pipelineBindPoint: i32, pipeline: u64);
        vkCmdSetViewport(commandBuffer: *mut c_void, firstViewport: u32, viewportCount: u32, pViewports: *const VkViewport);
        vkCmdSetScissor(commandBuffer: *mut c_void, firstScissor: u32, scissorCount: u32, pScissors: *const VkRect2D);
        vkCmdPushConstants(commandBuffer: *mut c_void, layout: u64, stageFlags: u32, offset: u32, size: u32, pValues: *const c_void);
        vkCmdDraw(commandBuffer: *mut c_void, vertexCount: u32, instanceCount: u32, firstVertex: u32, firstInstance: u32);
        vkCmdPipelineBarrier2(commandBuffer: *mut c_void, pDependencyInfo: *const VkDependencyInfo);
        vkCmdCopyBufferToImage(commandBuffer: *mut c_void, srcBuffer: u64, dstImage: u64, dstImageLayout: i32, regionCount: u32, pRegions: *const VkBufferImageCopy);
        vkCmdCopyImageToBuffer(commandBuffer: *mut c_void, srcImage: u64, srcImageLayout: i32, dstBuffer: u64, regionCount: u32, pRegions: *const VkBufferImageCopy);
        vkCmdBlitImage(commandBuffer: *mut c_void, srcImage: u64, srcImageLayout: i32, dstImage: u64, dstImageLayout: i32, regionCount: u32, pRegions: *const VkImageBlit, filter: i32);
        vkCmdClearColorImage(commandBuffer: *mut c_void, image: u64, imageLayout: i32, pColor: *const VkClearColorValue, rangeCount: u32, pRanges: *const VkImageSubresourceRange);
        vkCmdPushDescriptorSetKHR(commandBuffer: *mut c_void, pipelineBindPoint: i32, layout: u64, set: u32, descriptorWriteCount: u32, pDescriptorWrites: *const VkWriteDescriptorSet);
        vkCreateFence(device: *mut c_void, pCreateInfo: *const VkFenceCreateInfo, pAllocator: *const VkAllocationCallbacks, pFence: *mut u64) -> i32;
        vkDestroyFence(device: *mut c_void, fence: u64, pAllocator: *const VkAllocationCallbacks);
        vkWaitForFences(device: *mut c_void, fenceCount: u32, pFences: *const u64, waitAll: u32, timeout: u64) -> i32;
        vkResetFences(device: *mut c_void, fenceCount: u32, pFences: *const u64) -> i32;
        vkCreateSemaphore(device: *mut c_void, pCreateInfo: *const VkSemaphoreCreateInfo, pAllocator: *const VkAllocationCallbacks, pSemaphore: *mut u64) -> i32;
        vkDestroySemaphore(device: *mut c_void, semaphore: u64, pAllocator: *const VkAllocationCallbacks);
    }
}
table! {
    SwapchainFns {
        vkCreateSwapchainKHR(device: *mut c_void, pCreateInfo: *const VkSwapchainCreateInfoKHR, pAllocator: *const VkAllocationCallbacks, pSwapchain: *mut u64) -> i32;
        vkDestroySwapchainKHR(device: *mut c_void, swapchain: u64, pAllocator: *const VkAllocationCallbacks);
        vkGetSwapchainImagesKHR(device: *mut c_void, swapchain: u64, pSwapchainImageCount: *mut u32, pSwapchainImages: *mut u64) -> i32;
        vkAcquireNextImageKHR(device: *mut c_void, swapchain: u64, timeout: u64, semaphore: u64, fence: u64, pImageIndex: *mut u32) -> i32;
        vkQueuePresentKHR(queue: *mut c_void, pPresentInfo: *const VkPresentInfoKHR) -> i32;
    }
}

type GetInstanceProcAddr = unsafe extern "system" fn(*mut c_void, *const c_char) -> *const c_void;
/// Bibliothèque `libvulkan.so.1` ouverte, et `vkGetInstanceProcAddr`.
pub(in crate::platform::linux) struct Library {
    handle: *mut c_void,
    pub get_instance_proc_addr: GetInstanceProcAddr,
}
impl Library {
    /// Ouvre le chargeur Vulkan ; `None` s'il n'est pas installé.
    pub fn open() -> Option<Self> {
        // SAFETY : nom statique ; bibliothèque refermée par `Drop`.
        let handle = unsafe {
            libc::dlopen(
                c"libvulkan.so.1".as_ptr(),
                libc::RTLD_NOW | libc::RTLD_LOCAL,
            )
        };
        if handle.is_null() {
            return None;
        }
        // SAFETY : symbole du chargeur Vulkan, de cette signature.
        let symbol = unsafe { libc::dlsym(handle, c"vkGetInstanceProcAddr".as_ptr()) };
        if symbol.is_null() {
            // SAFETY : bibliothèque ouverte ci-dessus.
            unsafe { libc::dlclose(handle) };
            return None;
        }
        Some(Self {
            handle,
            // SAFETY : `vkGetInstanceProcAddr` a cette signature.
            get_instance_proc_addr: unsafe {
                std::mem::transmute::<*mut c_void, GetInstanceProcAddr>(symbol)
            },
        })
    }
    /// Fonctions de `instance` (nulle : fonctions globales).
    pub fn resolve(&self, instance: *mut c_void, name: &std::ffi::CStr) -> *const c_void {
        // SAFETY : instance valide ou nulle ; nom terminé par un octet nul.
        unsafe { (self.get_instance_proc_addr)(instance, name.as_ptr()) }
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY : bibliothèque ouverte par `open`, refermée une fois (après toutes les
        // ressources Vulkan, détruites avant).
        unsafe { libc::dlclose(self.handle) };
    }
}
/// Version de l'API (`VK_MAKE_API_VERSION(0, major, minor, 0)`).
pub(in crate::platform::linux) const fn version(major: u32, minor: u32) -> u32 {
    (major << 22) | (minor << 12)
}
