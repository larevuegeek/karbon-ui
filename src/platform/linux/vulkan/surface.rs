//! Surface d'une fenêtre (Wayland : `VK_KHR_wayland_surface`, X11 : `VK_KHR_xcb_surface`) et
//! sa chaîne d'échange (`VK_KHR_swapchain`) : images BGRA 8 bits au format des cibles, à la
//! taille en pixels de la fenêtre (le compositeur Wayland la laisse au client,
//! fractionnaire comprise : la destination du viewport est posée par la fenêtre), refaite
//! quand cette taille change ou que la chaîne est périmée.
//!
//! Rythme : sous Wayland, la fenêtre n'envoie une image qu'après le rappel de trame de la
//! précédente (aucune image pour une fenêtre cachée) ; le mode « mailbox » ne bloque donc
//! jamais, et à défaut « fifo » (toujours disponible) ne bloque pas davantage. X11 n'a pas
//! de rappel de trame : « fifo » y cale les images sur le balayage de l'écran.
//! Synchronisation : un sémaphore d'acquisition attendu par la soumission (le travail de
//! l'image précédente est terminé avant la suivante), et un sémaphore de fin de rendu par
//! image de la chaîne, attendu par la présentation (libre dès que l'image est de nouveau
//! acquise).
use super::{
    FORMAT,
    device::{Gpu, check},
    memory::Image,
};
use crate::platform::linux::ffi::vulkan::*;
use std::{ffi::c_void, rc::Rc};

/// Attente au plus d'une image libre de la chaîne : au-delà, l'image est reprise plus
/// tard.
const ACQUIRE_TIMEOUT: u64 = 50_000_000;

fn unsupported(reason: &'static str) -> crate::Error {
    crate::Error::Unsupported { reason }
}
/// Fenêtre native d'une surface.
#[derive(Clone, Copy)]
pub(in crate::platform::linux) enum Native {
    Wayland {
        display: *mut c_void,
        surface: *mut c_void,
    },
    Xcb {
        connection: *mut c_void,
        window: u32,
    },
}
pub(in crate::platform::linux) struct Surface {
    raw: u64,
    swapchain: u64,
    pub(super) images: Vec<Image>,
    /// Fin du rendu de chaque image, attendue par sa présentation.
    pub(super) rendered: Vec<u64>,
    /// Image acquise, attendue par la soumission.
    pub(super) acquired: u64,
    extent: (usize, usize),
    alpha: u32,
    mode: i32,
    /// La chaîne ne correspond plus à la surface : refaite avant l'image suivante.
    stale: bool,
    gpu: Rc<Gpu>,
}
impl Surface {
    /// Surface Vulkan de la fenêtre `native` ; `transparent` : l'alpha des images est
    /// conservé par le compositeur.
    pub(super) fn new(gpu: &Rc<Gpu>, native: Native, transparent: bool) -> crate::Result<Self> {
        let (Some(fns), Some(_)) = (&gpu.surface, &gpu.swapchain) else {
            return Err(unsupported("Vulkan presentation unavailable"));
        };
        let mut this = Self {
            raw: 0,
            swapchain: 0,
            images: Vec::new(),
            rendered: Vec::new(),
            acquired: 0,
            extent: (0, 0),
            alpha: 0,
            mode: VK_PRESENT_MODE_FIFO_KHR,
            stale: true,
            gpu: Rc::clone(gpu),
        };
        // SAFETY : connexion et fenêtre natives vivantes plus longtemps que la surface
        // Vulkan (détruite par `Drop` avant elles) ; sorties locales.
        unsafe {
            match native {
                Native::Wayland { display, surface } => {
                    let Some(create) = &gpu.wayland_surface else {
                        return Err(unsupported("no Vulkan Wayland surfaces"));
                    };
                    let info = VkWaylandSurfaceCreateInfoKHR {
                        sType: VK_STRUCTURE_TYPE_WAYLAND_SURFACE_CREATE_INFO_KHR,
                        pNext: std::ptr::null(),
                        flags: 0,
                        display,
                        surface,
                    };
                    check(
                        (create.vkCreateWaylandSurfaceKHR)(
                            gpu.instance,
                            &info,
                            std::ptr::null(),
                            &mut this.raw,
                        ),
                        "vkCreateWaylandSurfaceKHR",
                    )?;
                }
                Native::Xcb { connection, window } => {
                    let Some(create) = &gpu.xcb_surface else {
                        return Err(unsupported("no Vulkan XCB surfaces"));
                    };
                    let info = VkXcbSurfaceCreateInfoKHR {
                        sType: VK_STRUCTURE_TYPE_XCB_SURFACE_CREATE_INFO_KHR,
                        pNext: std::ptr::null(),
                        flags: 0,
                        connection,
                        window,
                    };
                    check(
                        (create.vkCreateXcbSurfaceKHR)(
                            gpu.instance,
                            &info,
                            std::ptr::null(),
                            &mut this.raw,
                        ),
                        "vkCreateXcbSurfaceKHR",
                    )?;
                }
            }
            let mut supported = 0;
            check(
                (fns.vkGetPhysicalDeviceSurfaceSupportKHR)(
                    gpu.physical,
                    gpu.family,
                    this.raw,
                    &mut supported,
                ),
                "vkGetPhysicalDeviceSurfaceSupportKHR",
            )?;
            if supported == 0 {
                return Err(unsupported("Vulkan queue cannot present to the surface"));
            }
            let formats = super::device::list(|n, p| {
                (fns.vkGetPhysicalDeviceSurfaceFormatsKHR)(gpu.physical, this.raw, n, p)
            });
            if !formats
                .iter()
                .any(|f| f.format == FORMAT && f.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR)
            {
                return Err(unsupported("no BGRA8 surface format"));
            }
            let modes = super::device::list(|n, p| {
                (fns.vkGetPhysicalDeviceSurfacePresentModesKHR)(gpu.physical, this.raw, n, p)
            });
            if matches!(native, Native::Wayland { .. })
                && modes.contains(&VK_PRESENT_MODE_MAILBOX_KHR)
            {
                this.mode = VK_PRESENT_MODE_MAILBOX_KHR;
            }
            let caps = this.capabilities()?;
            let supported = caps.supportedCompositeAlpha;
            this.alpha = [
                (!transparent, VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR),
                (true, VK_COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR),
                (true, VK_COMPOSITE_ALPHA_INHERIT_BIT_KHR),
            ]
            .into_iter()
            .find(|&(wanted, bit)| wanted && supported & bit != 0)
            .map(|(_, bit)| bit)
            .ok_or(unsupported("no usable composite alpha"))?;
            let semaphore = VkSemaphoreCreateInfo {
                sType: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
            };
            check(
                (gpu.fns.vkCreateSemaphore)(
                    gpu.device,
                    &semaphore,
                    std::ptr::null(),
                    &mut this.acquired,
                ),
                "vkCreateSemaphore",
            )?;
        }
        Ok(this)
    }
    /// Mode de présentation retenu (diagnostic).
    pub(in crate::platform::linux) fn mode(&self) -> &'static str {
        if self.mode == VK_PRESENT_MODE_MAILBOX_KHR {
            "mailbox"
        } else {
            "fifo"
        }
    }
    fn capabilities(&self) -> crate::Result<VkSurfaceCapabilitiesKHR> {
        let fns = self.gpu.surface.as_ref().expect("surface functions");
        // SAFETY : sortie locale entièrement écrite par Vulkan.
        let mut caps: VkSurfaceCapabilitiesKHR = unsafe { std::mem::zeroed() };
        // SAFETY : GPU et surface valides.
        check(
            unsafe {
                (fns.vkGetPhysicalDeviceSurfaceCapabilitiesKHR)(
                    self.gpu.physical,
                    self.raw,
                    &mut caps,
                )
            },
            "vkGetPhysicalDeviceSurfaceCapabilitiesKHR",
        )?;
        Ok(caps)
    }
    /// Refait la chaîne à `size` pixels (le GPU est au repos : l'image précédente est
    /// terminée).
    fn configure(&mut self, (width, height): (usize, usize)) -> crate::Result<()> {
        let gpu = Rc::clone(&self.gpu);
        let fns = gpu.swapchain.as_ref().expect("swapchain functions");
        let caps = self.capabilities()?;
        let mut count = caps.minImageCount.max(3);
        if caps.maxImageCount > 0 {
            count = count.min(caps.maxImageCount);
        }
        let clamp =
            |v: usize, min: u32, max: u32| (v as u32).clamp(min.max(1), max.max(min.max(1)));
        let extent = VkExtent2D {
            width: clamp(width, caps.minImageExtent.width, caps.maxImageExtent.width),
            height: clamp(
                height,
                caps.minImageExtent.height,
                caps.maxImageExtent.height,
            ),
        };
        let info = VkSwapchainCreateInfoKHR {
            sType: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
            pNext: std::ptr::null(),
            flags: 0,
            surface: self.raw,
            minImageCount: count,
            imageFormat: FORMAT,
            imageColorSpace: VK_COLOR_SPACE_SRGB_NONLINEAR_KHR,
            imageExtent: extent,
            imageArrayLayers: 1,
            imageUsage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
            imageSharingMode: VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            preTransform: caps.currentTransform,
            compositeAlpha: self.alpha,
            presentMode: self.mode,
            clipped: 1,
            oldSwapchain: self.swapchain,
        };
        let mut swapchain = 0;
        // SAFETY : appareil et surface valides ; l'ancienne chaîne est retirée ensuite.
        unsafe {
            check((gpu.fns.vkDeviceWaitIdle)(gpu.device), "vkDeviceWaitIdle")?;
            check(
                (fns.vkCreateSwapchainKHR)(gpu.device, &info, std::ptr::null(), &mut swapchain),
                "vkCreateSwapchainKHR",
            )?;
            self.images.clear();
            (fns.vkDestroySwapchainKHR)(gpu.device, self.swapchain, std::ptr::null());
            self.swapchain = swapchain;
            let raws = super::device::list(|n, p| {
                (fns.vkGetSwapchainImagesKHR)(gpu.device, swapchain, n, p)
            });
            let size = (extent.width as usize, extent.height as usize);
            for raw in raws {
                self.images.push(Image::borrowed(&gpu, raw, size, FORMAT)?);
            }
            let semaphore = VkSemaphoreCreateInfo {
                sType: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
            };
            while self.rendered.len() < self.images.len() {
                let mut raw = 0;
                check(
                    (gpu.fns.vkCreateSemaphore)(gpu.device, &semaphore, std::ptr::null(), &mut raw),
                    "vkCreateSemaphore",
                )?;
                self.rendered.push(raw);
            }
        }
        self.extent = (width, height);
        self.stale = false;
        Ok(())
    }
    /// Image libre de la chaîne pour une image de `size` pixels (chaîne refaite si
    /// besoin) ; `None` si aucune ne se libère à temps.
    pub(super) fn acquire(&mut self, size: (usize, usize)) -> crate::Result<Option<usize>> {
        for _ in 0..2 {
            if self.stale || self.extent != size || self.swapchain == 0 {
                self.configure(size)?;
            }
            let fns = self.gpu.swapchain.as_ref().expect("swapchain functions");
            let mut index = 0;
            // SAFETY : chaîne valide ; sémaphore non signalé (soumission précédente
            // terminée).
            let result = unsafe {
                (fns.vkAcquireNextImageKHR)(
                    self.gpu.device,
                    self.swapchain,
                    ACQUIRE_TIMEOUT,
                    self.acquired,
                    0,
                    &mut index,
                )
            };
            match result {
                VK_SUCCESS => return Ok(Some(index as usize)),
                VK_SUBOPTIMAL_KHR => {
                    self.stale = true;
                    return Ok(Some(index as usize));
                }
                VK_TIMEOUT | VK_NOT_READY => return Ok(None),
                VK_ERROR_OUT_OF_DATE_KHR => self.stale = true,
                other => check(other, "vkAcquireNextImageKHR")?,
            }
        }
        Ok(None)
    }
    /// Présente l'image `index` une fois son rendu terminé.
    pub(super) fn present(&mut self, index: usize) -> crate::Result<()> {
        let fns = self.gpu.swapchain.as_ref().expect("swapchain functions");
        let image = index as u32;
        let info = VkPresentInfoKHR {
            sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
            pNext: std::ptr::null(),
            waitSemaphoreCount: 1,
            pWaitSemaphores: &self.rendered[index],
            swapchainCount: 1,
            pSwapchains: &self.swapchain,
            pImageIndices: &image,
            pResults: std::ptr::null_mut(),
        };
        // SAFETY : image acquise et soumise ; sémaphore signalé par la soumission.
        match unsafe { (fns.vkQueuePresentKHR)(self.gpu.queue, &info) } {
            VK_SUCCESS => Ok(()),
            VK_SUBOPTIMAL_KHR | VK_ERROR_OUT_OF_DATE_KHR => {
                self.stale = true;
                Ok(())
            }
            other => check(other, "vkQueuePresentKHR"),
        }
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        let gpu = Rc::clone(&self.gpu);
        // SAFETY : GPU au repos ; objets possédés, détruits une fois (nuls ignorés).
        unsafe {
            (gpu.fns.vkDeviceWaitIdle)(gpu.device);
            self.images.clear();
            for &semaphore in self.rendered.iter().chain([&self.acquired]) {
                (gpu.fns.vkDestroySemaphore)(gpu.device, semaphore, std::ptr::null());
            }
            if let Some(fns) = &gpu.swapchain {
                (fns.vkDestroySwapchainKHR)(gpu.device, self.swapchain, std::ptr::null());
            }
            if let Some(fns) = &gpu.surface {
                (fns.vkDestroySurfaceKHR)(gpu.instance, self.raw, std::ptr::null());
            }
        }
    }
}
