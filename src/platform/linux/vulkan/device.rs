//! Instance, choix du GPU et appareil logique.
//!
//! Vulkan 1.3 est demandé (rendu dynamique, synchronization2) avec les descripteurs
//! poussés (`VK_KHR_push_descriptor`). Le GPU intégré est préféré (plus économe, comme
//! sous macOS), puis le dédié ; un rendu Vulkan sur processeur (llvmpipe) n'est retenu
//! que sur demande (tests), Cairo étant plus rapide pour les fenêtres.
//! `KARBON_VULKAN_DEVICE=<texte>` impose le GPU dont le nom contient ce texte ;
//! `KARBON_VULKAN_VALIDATION` active la couche de validation de Khronos.
use crate::platform::linux::ffi::vulkan::*;
use std::ffi::{CStr, c_void};

/// Échec d'un appel Vulkan.
pub(super) fn check(result: i32, operation: &'static str) -> crate::Result<()> {
    match result {
        VK_SUCCESS => Ok(()),
        VK_ERROR_DEVICE_LOST => Err(crate::Error::DeviceLost {
            operation,
            code: result,
        }),
        _ => Err(crate::Error::Platform {
            operation,
            code: result,
        }),
    }
}
fn unsupported(reason: &'static str) -> crate::Error {
    crate::Error::Unsupported { reason }
}
/// Liste rendue par un appel Vulkan en deux temps (nombre, puis éléments).
pub(super) fn list<T: Copy>(mut call: impl FnMut(*mut u32, *mut T) -> i32) -> Vec<T> {
    let mut count = 0;
    if call(&mut count, std::ptr::null_mut()) != VK_SUCCESS || count == 0 {
        return Vec::new();
    }
    let mut items = Vec::with_capacity(count as usize);
    let result = call(&mut count, items.as_mut_ptr());
    if result != VK_SUCCESS && result != VK_INCOMPLETE {
        return Vec::new();
    }
    // SAFETY : `count` éléments écrits par Vulkan.
    unsafe { items.set_len(count as usize) };
    items
}
fn extension_names(properties: &[VkExtensionProperties]) -> Vec<String> {
    properties
        .iter()
        .map(|p| {
            // SAFETY : nom terminé par un octet nul (taille fixe de Vulkan).
            unsafe { CStr::from_ptr(p.extensionName.as_ptr()) }
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}
/// Contexte Vulkan : instance, GPU, appareil et file graphique.
pub(in crate::platform::linux) struct Gpu {
    pub device: *mut c_void,
    pub fns: DeviceFns,
    pub swapchain: Option<SwapchainFns>,
    pub queue: *mut c_void,
    pub family: u32,
    pub physical: *mut c_void,
    pub instance: *mut c_void,
    pub instance_fns: InstanceFns,
    pub surface: Option<SurfaceFns>,
    pub wayland_surface: Option<WaylandSurfaceFns>,
    pub xcb_surface: Option<XcbSurfaceFns>,
    pub memory: VkPhysicalDeviceMemoryProperties,
    /// Limites du GPU (taille maximale des images, des tampons…), consultées avant de
    /// créer une ressource : les dépasser est un comportement indéfini.
    pub limits: VkPhysicalDeviceLimits,
    /// Nom du GPU (diagnostic, `--profile`).
    pub name: String,
    /// Chargeur Vulkan, ouvert tant que le contexte existe.
    _library: Library,
}
impl Gpu {
    /// Contexte sur le GPU préféré ; `allow_cpu` admet un rendu Vulkan sur processeur.
    pub fn new(allow_cpu: bool) -> crate::Result<Self> {
        let library =
            Library::open().ok_or_else(|| unsupported("no Vulkan loader (libvulkan.so.1)"))?;
        // SAFETY : fonctions globales résolues avec une instance nulle.
        let entry = unsafe { Entry::load(|name| library.resolve(std::ptr::null_mut(), name)) }
            .ok_or_else(|| unsupported("Vulkan loader without Vulkan 1.1 entry points"))?;
        let mut api = 0;
        // SAFETY : sortie locale.
        unsafe { (entry.vkEnumerateInstanceVersion)(&mut api) };
        if api < version(1, 3) {
            return Err(unsupported("Vulkan 1.3 is not available"));
        }
        // SAFETY : sorties allouées par `list`.
        let available = extension_names(&list(|n, p| unsafe {
            (entry.vkEnumerateInstanceExtensionProperties)(std::ptr::null(), n, p)
        }));
        // Surfaces de fenêtres : l'extension commune, et celles de Wayland et d'XCB
        // disponibles.
        let has = |name: &CStr| available.iter().any(|a| a.as_bytes() == name.to_bytes());
        let platforms: Vec<&CStr> = [c"VK_KHR_wayland_surface", c"VK_KHR_xcb_surface"]
            .into_iter()
            .filter(|e| has(e))
            .collect();
        let presentable = has(c"VK_KHR_surface") && !platforms.is_empty();
        let extensions: Vec<*const c_char> = if presentable {
            std::iter::once(c"VK_KHR_surface")
                .chain(platforms.iter().copied())
                .map(CStr::as_ptr)
                .collect()
        } else {
            Vec::new()
        };
        let validation = std::env::var_os("KARBON_VULKAN_VALIDATION").is_some()
            && {
                // Demander une couche absente ferait échouer l'instance (et passer la fenêtre
                // à Cairo sans explication).
                // SAFETY : sorties allouées par `list` ; noms terminés par un octet nul.
                let layers =
                    list(|n, p| unsafe { (entry.vkEnumerateInstanceLayerProperties)(n, p) });
                let present = layers
                .iter()
                .any(|l| unsafe { CStr::from_ptr(l.layerName.as_ptr()) } == c"VK_LAYER_KHRONOS_validation");
                if !present {
                    eprintln!(
                        "karbon-ui: KARBON_VULKAN_VALIDATION: VK_LAYER_KHRONOS_validation is not installed"
                    );
                }
                present
            };
        let layers = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
        let app = VkApplicationInfo {
            sType: VK_STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: std::ptr::null(),
            pApplicationName: c"karbon-ui".as_ptr(),
            applicationVersion: 0,
            pEngineName: c"karbon-ui".as_ptr(),
            engineVersion: 0,
            apiVersion: version(1, 3),
        };
        let info = VkInstanceCreateInfo {
            sType: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            pApplicationInfo: &app,
            enabledLayerCount: u32::from(validation),
            ppEnabledLayerNames: layers.as_ptr(),
            enabledExtensionCount: extensions.len() as u32,
            ppEnabledExtensionNames: extensions.as_ptr(),
        };
        let mut instance = std::ptr::null_mut();
        // SAFETY : descriptions locales valides pendant l'appel.
        check(
            unsafe { (entry.vkCreateInstance)(&info, std::ptr::null(), &mut instance) },
            "vkCreateInstance",
        )?;
        // SAFETY : instance valide ; fonctions de l'instance. Un chargeur sans les
        // fonctions de base de Vulkan 1.3 n'existe pas : l'instance n'est alors pas
        // détruite (rien pour le faire).
        let instance_fns = unsafe { InstanceFns::load(|name| library.resolve(instance, name)) }
            .ok_or_else(|| unsupported("incomplete Vulkan instance"))?;
        // SAFETY : idem ; extensions activées ci-dessus.
        let surface = presentable
            .then(|| unsafe { SurfaceFns::load(|name| library.resolve(instance, name)) })
            .flatten();
        // SAFETY : idem ; tables nulles si l'extension manque.
        let windows = (
            unsafe { WaylandSurfaceFns::load(|name| library.resolve(instance, name)) }
                .filter(|_| presentable),
            unsafe { XcbSurfaceFns::load(|name| library.resolve(instance, name)) }
                .filter(|_| presentable),
        );
        // L'instance est détruite par `device` s'il échoue.
        let mut gpu = Self::device(library, instance, instance_fns, surface, allow_cpu)?;
        (gpu.wayland_surface, gpu.xcb_surface) = windows;
        Ok(gpu)
    }
    /// Choisit le GPU et crée l'appareil logique.
    fn device(
        library: Library,
        instance: *mut c_void,
        fns: InstanceFns,
        surface: Option<SurfaceFns>,
        allow_cpu: bool,
    ) -> crate::Result<Self> {
        let destroy = |fns: &InstanceFns| {
            // SAFETY : instance valide, sans appareil créé.
            unsafe { (fns.vkDestroyInstance)(instance, std::ptr::null()) }
        };
        // SAFETY : instance valide.
        let physicals = list(|n, p| unsafe { (fns.vkEnumeratePhysicalDevices)(instance, n, p) });
        let forced = std::env::var("KARBON_VULKAN_DEVICE").ok();
        // Rang, GPU, propriétés, famille de la file graphique, chaîne d'échange possible.
        let mut best: Option<(u32, *mut c_void, VkPhysicalDeviceProperties, u32, bool)> = None;
        for physical in physicals {
            // SAFETY : sortie locale de la bonne taille (assertions de `device.rs`).
            let mut props: VkPhysicalDeviceProperties = unsafe { std::mem::zeroed() };
            unsafe { (fns.vkGetPhysicalDeviceProperties)(physical, &mut props) };
            // SAFETY : nom terminé par un octet nul.
            let name = unsafe { CStr::from_ptr(props.deviceName.as_ptr()) }.to_string_lossy();
            if props.apiVersion < version(1, 3) {
                continue;
            }
            let rank = match forced.as_deref() {
                Some(f) if !name.contains(f) => continue,
                Some(_) => 0,
                None => match props.deviceType {
                    VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU => 0,
                    VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU => 1,
                    VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU => 2,
                    VK_PHYSICAL_DEVICE_TYPE_CPU if allow_cpu => 3,
                    _ => continue,
                },
            };
            // SAFETY : GPU valide.
            let extensions = extension_names(&list(|n, p| unsafe {
                (fns.vkEnumerateDeviceExtensionProperties)(physical, std::ptr::null(), n, p)
            }));
            if !extensions.iter().any(|e| e == "VK_KHR_push_descriptor") {
                continue;
            }
            // SAFETY : GPU valide.
            let families = list(|n, p| {
                unsafe { (fns.vkGetPhysicalDeviceQueueFamilyProperties)(physical, n, p) };
                VK_SUCCESS
            });
            let Some(family) = families
                .iter()
                .position(|f: &VkQueueFamilyProperties| f.queueFlags & VK_QUEUE_GRAPHICS_BIT != 0)
            else {
                continue;
            };
            let swapchain = extensions.iter().any(|e| e == "VK_KHR_swapchain");
            if best.as_ref().is_none_or(|(r, ..)| rank < *r) {
                best = Some((rank, physical, props, family as u32, swapchain));
            }
        }
        let Some((_, physical, props, family, swapchain)) = best else {
            destroy(&fns);
            return Err(unsupported("no Vulkan 1.3 GPU with VK_KHR_push_descriptor"));
        };
        let mut features13: VkPhysicalDeviceVulkan13Features = unsafe { std::mem::zeroed() };
        features13.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_3_FEATURES;
        features13.dynamicRendering = 1;
        features13.synchronization2 = 1;
        let priority = 1f32;
        let queue = VkDeviceQueueCreateInfo {
            sType: VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            queueFamilyIndex: family,
            queueCount: 1,
            pQueuePriorities: &priority,
        };
        let mut extensions = vec![c"VK_KHR_push_descriptor".as_ptr()];
        if swapchain && surface.is_some() {
            extensions.push(c"VK_KHR_swapchain".as_ptr());
        }
        let info = VkDeviceCreateInfo {
            sType: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
            pNext: (&raw const features13).cast(),
            flags: 0,
            queueCreateInfoCount: 1,
            pQueueCreateInfos: &queue,
            enabledLayerCount: 0,
            ppEnabledLayerNames: std::ptr::null(),
            enabledExtensionCount: extensions.len() as u32,
            ppEnabledExtensionNames: extensions.as_ptr(),
            pEnabledFeatures: std::ptr::null(),
        };
        let mut device = std::ptr::null_mut();
        // SAFETY : descriptions locales valides pendant l'appel.
        if let Err(error) = check(
            unsafe { (fns.vkCreateDevice)(physical, &info, std::ptr::null(), &mut device) },
            "vkCreateDevice",
        ) {
            destroy(&fns);
            return Err(error);
        }
        // SAFETY : appareil valide ; fonctions résolues par `vkGetDeviceProcAddr`.
        let resolve = |name: &CStr| unsafe { (fns.vkGetDeviceProcAddr)(device, name.as_ptr()) };
        // SAFETY : idem.
        let Some(device_fns) = (unsafe { DeviceFns::load(resolve) }) else {
            // L'appareil a été créé : il est détruit avant l'instance, par la seule
            // fonction que tout appareil possède.
            let destroy_device = resolve(c"vkDestroyDevice");
            if !destroy_device.is_null() {
                // SAFETY : fonction de l'appareil, de cette signature ; appareil valide,
                // sans objet enfant.
                unsafe {
                    let destroy_device: unsafe extern "C" fn(
                        *mut c_void,
                        *const VkAllocationCallbacks,
                    ) = std::mem::transmute(destroy_device);
                    destroy_device(device, std::ptr::null());
                }
            }
            destroy(&fns);
            return Err(unsupported("incomplete Vulkan device"));
        };
        // SAFETY : idem ; extension activée ci-dessus.
        let swapchain_fns = (extensions.len() > 1)
            .then(|| unsafe { SwapchainFns::load(resolve) })
            .flatten();
        let mut queue_handle = std::ptr::null_mut();
        // SAFETY : file 0 de la famille demandée.
        unsafe { (device_fns.vkGetDeviceQueue)(device, family, 0, &mut queue_handle) };
        let mut memory: VkPhysicalDeviceMemoryProperties = unsafe { std::mem::zeroed() };
        // SAFETY : sortie locale.
        unsafe { (fns.vkGetPhysicalDeviceMemoryProperties)(physical, &mut memory) };
        // SAFETY : nom terminé par un octet nul.
        let name = unsafe { CStr::from_ptr(props.deviceName.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        Ok(Self {
            device,
            fns: device_fns,
            swapchain: swapchain_fns,
            queue: queue_handle,
            family,
            physical,
            instance,
            instance_fns: fns,
            surface,
            wayland_surface: None,
            xcb_surface: None,
            memory,
            limits: props.limits,
            name,
            _library: library,
        })
    }
    /// Plus grand côté (pixels) d'une image qui peut être créée, échantillonnée et
    /// servir de cible de rendu sur ce GPU (4096 au moins d'après la norme).
    pub fn max_image_edge(&self) -> usize {
        self.limits
            .maxImageDimension2D
            .min(self.limits.maxFramebufferWidth)
            .min(self.limits.maxFramebufferHeight) as usize
    }
    /// Type de mémoire compatible avec `bits` et possédant `flags`.
    pub fn memory_type(&self, bits: u32, flags: u32) -> Option<u32> {
        (0..self.memory.memoryTypeCount).find(|&i| {
            bits & (1 << i) != 0
                && self.memory.memoryTypes[i as usize].propertyFlags & flags == flags
        })
    }
}
impl Drop for Gpu {
    fn drop(&mut self) {
        // SAFETY : appareil inactif (attendu), sans objet enfant restant (les ressources
        // du renderer sont détruites avant lui) ; puis instance.
        unsafe {
            (self.fns.vkDeviceWaitIdle)(self.device);
            (self.fns.vkDestroyDevice)(self.device, std::ptr::null());
            (self.instance_fns.vkDestroyInstance)(self.instance, std::ptr::null());
        }
    }
}
