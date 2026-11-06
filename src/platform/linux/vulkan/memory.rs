//! Ressources du GPU : tampons en mémoire visible du processeur (projetés, cohérents) et
//! images en mémoire du GPU avec leur vue. Chaque ressource garde le contexte (`Rc<Gpu>`)
//! et le détruit après elle, jamais avant.
use super::device::{Gpu, check};
use crate::platform::linux::ffi::vulkan::*;
use std::{ffi::c_void, rc::Rc};

fn allocate(gpu: &Gpu, requirements: VkMemoryRequirements, flags: u32) -> crate::Result<u64> {
    let kind =
        gpu.memory_type(requirements.memoryTypeBits, flags)
            .ok_or(crate::Error::Unsupported {
                reason: "no suitable Vulkan memory type",
            })?;
    let info = VkMemoryAllocateInfo {
        sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        pNext: std::ptr::null(),
        allocationSize: requirements.size,
        memoryTypeIndex: kind,
    };
    let mut memory = 0;
    // SAFETY : appareil valide ; description locale.
    check(
        unsafe { (gpu.fns.vkAllocateMemory)(gpu.device, &info, std::ptr::null(), &mut memory) },
        "vkAllocateMemory",
    )?;
    Ok(memory)
}
/// Tampon projeté en mémoire visible et cohérente (données envoyées à chaque image,
/// relecture).
pub(super) struct Buffer {
    pub raw: u64,
    memory: u64,
    mapped: *mut u8,
    pub size: u64,
    gpu: Rc<Gpu>,
}
impl Buffer {
    pub fn new(gpu: &Rc<Gpu>, size: u64, usage: u32) -> crate::Result<Self> {
        let size = size.max(256);
        let info = VkBufferCreateInfo {
            sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            size,
            usage,
            sharingMode: VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
        };
        let mut raw = 0;
        // SAFETY : appareil valide ; description locale.
        check(
            unsafe { (gpu.fns.vkCreateBuffer)(gpu.device, &info, std::ptr::null(), &mut raw) },
            "vkCreateBuffer",
        )?;
        let mut requirements: VkMemoryRequirements = unsafe { std::mem::zeroed() };
        // SAFETY : tampon valide ; sortie locale.
        unsafe { (gpu.fns.vkGetBufferMemoryRequirements)(gpu.device, raw, &mut requirements) };
        let flags = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
        let memory = match allocate(gpu, requirements, flags) {
            Ok(memory) => memory,
            Err(error) => {
                // SAFETY : tampon sans mémoire, détruit une fois.
                unsafe { (gpu.fns.vkDestroyBuffer)(gpu.device, raw, std::ptr::null()) };
                return Err(error);
            }
        };
        let mut mapped = std::ptr::null_mut::<c_void>();
        // SAFETY : mémoire et tampon valides ; projection de toute la mémoire.
        let bound = unsafe {
            check(
                (gpu.fns.vkBindBufferMemory)(gpu.device, raw, memory, 0),
                "vkBindBufferMemory",
            )
            .and_then(|()| {
                check(
                    (gpu.fns.vkMapMemory)(gpu.device, memory, 0, u64::MAX, 0, &mut mapped),
                    "vkMapMemory",
                )
            })
        };
        let buffer = Self {
            raw,
            memory,
            mapped: mapped.cast(),
            size,
            gpu: Rc::clone(gpu),
        };
        bound.map(|()| buffer)
    }
    /// Écrit `data` à l'octet `offset` du tampon (assez grand).
    pub fn write_at<T: Copy>(&mut self, offset: usize, data: &[T]) {
        let bytes = std::mem::size_of_val(data);
        assert!((offset + bytes) as u64 <= self.size, "buffer too small");
        // SAFETY : projection de `size` octets ; données de `bytes` octets, dans les bornes.
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr().cast::<u8>(),
                self.mapped.add(offset),
                bytes,
            )
        };
    }
    /// Octets du tampon (relecture après une copie terminée).
    pub fn read(&self, len: usize) -> &[u8] {
        // SAFETY : projection de `size` octets, lue après la fin du travail du GPU.
        unsafe { std::slice::from_raw_parts(self.mapped, len.min(self.size as usize)) }
    }
}
impl Drop for Buffer {
    fn drop(&mut self) {
        let gpu = &self.gpu;
        // SAFETY : ressources possédées, plus utilisées par le GPU (attendu par
        // l'appelant) ; la mémoire projetée est libérée avec elles.
        unsafe {
            (gpu.fns.vkDestroyBuffer)(gpu.device, self.raw, std::ptr::null());
            (gpu.fns.vkFreeMemory)(gpu.device, self.memory, std::ptr::null());
        }
    }
}
/// Image 2D en mémoire du GPU, avec sa vue et sa disposition courante ; ou image d'une
/// chaîne d'échange (empruntée : seule sa vue lui appartient).
pub(super) struct Image {
    pub raw: u64,
    /// Mémoire possédée (0 : image empruntée).
    memory: u64,
    pub view: u64,
    pub width: usize,
    pub height: usize,
    /// Niveaux de mipmap.
    pub levels: u32,
    /// Disposition actuelle de tous les niveaux (suivie par les barrières enregistrées).
    pub layout: i32,
    gpu: Rc<Gpu>,
}
impl Image {
    pub fn new(
        gpu: &Rc<Gpu>,
        width: usize,
        height: usize,
        format: i32,
        usage: u32,
    ) -> crate::Result<Self> {
        Self::with_levels(gpu, (width, height), format, usage, 1)
    }
    /// Image de `levels` niveaux de mipmap (vue sur tous les niveaux).
    pub fn with_levels(
        gpu: &Rc<Gpu>,
        (width, height): (usize, usize),
        format: i32,
        usage: u32,
        levels: u32,
    ) -> crate::Result<Self> {
        let max = gpu.max_image_edge();
        if width == 0 || height == 0 || width > max || height > max {
            return Err(crate::Error::invalid_input("invalid image size"));
        }
        let info = VkImageCreateInfo {
            sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            imageType: VK_IMAGE_TYPE_2D,
            format,
            extent: VkExtent3D {
                width: width as u32,
                height: height as u32,
                depth: 1,
            },
            mipLevels: levels,
            arrayLayers: 1,
            samples: VK_SAMPLE_COUNT_1_BIT,
            tiling: VK_IMAGE_TILING_OPTIMAL,
            usage,
            sharingMode: VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
        };
        let mut raw = 0;
        // SAFETY : appareil valide ; description locale.
        check(
            unsafe { (gpu.fns.vkCreateImage)(gpu.device, &info, std::ptr::null(), &mut raw) },
            "vkCreateImage",
        )?;
        let mut requirements: VkMemoryRequirements = unsafe { std::mem::zeroed() };
        // SAFETY : image valide ; sortie locale.
        unsafe { (gpu.fns.vkGetImageMemoryRequirements)(gpu.device, raw, &mut requirements) };
        let memory = match allocate(gpu, requirements, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT) {
            Ok(memory) => memory,
            Err(error) => {
                // SAFETY : image sans mémoire, détruite une fois.
                unsafe { (gpu.fns.vkDestroyImage)(gpu.device, raw, std::ptr::null()) };
                return Err(error);
            }
        };
        let mut image = Self {
            raw,
            memory,
            view: 0,
            width,
            height,
            levels,
            layout: VK_IMAGE_LAYOUT_UNDEFINED,
            gpu: Rc::clone(gpu),
        };
        // SAFETY : image et mémoire valides.
        check(
            unsafe { (gpu.fns.vkBindImageMemory)(gpu.device, raw, memory, 0) },
            "vkBindImageMemory",
        )?;
        image.view = view(gpu, raw, format)?;
        Ok(image)
    }
    /// Image `raw` d'une chaîne d'échange, de `width` × `height` pixels au format
    /// `format` : sa vue est créée, l'image reste à la chaîne.
    pub fn borrowed(
        gpu: &Rc<Gpu>,
        raw: u64,
        (width, height): (usize, usize),
        format: i32,
    ) -> crate::Result<Self> {
        Ok(Self {
            raw,
            memory: 0,
            view: view(gpu, raw, format)?,
            width,
            height,
            levels: 1,
            layout: VK_IMAGE_LAYOUT_UNDEFINED,
            gpu: Rc::clone(gpu),
        })
    }
}
/// Vue 2D de tous les niveaux de `raw`.
fn view(gpu: &Gpu, raw: u64, format: i32) -> crate::Result<u64> {
    let view = VkImageViewCreateInfo {
        sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        image: raw,
        viewType: VK_IMAGE_VIEW_TYPE_2D,
        format,
        components: VkComponentMapping {
            r: VK_COMPONENT_SWIZZLE_IDENTITY,
            g: VK_COMPONENT_SWIZZLE_IDENTITY,
            b: VK_COMPONENT_SWIZZLE_IDENTITY,
            a: VK_COMPONENT_SWIZZLE_IDENTITY,
        },
        subresourceRange: color_range(),
    };
    let mut raw_view = 0;
    // SAFETY : description locale ; vue détruite avec l'image.
    check(
        unsafe { (gpu.fns.vkCreateImageView)(gpu.device, &view, std::ptr::null(), &mut raw_view) },
        "vkCreateImageView",
    )?;
    Ok(raw_view)
}
impl Drop for Image {
    fn drop(&mut self) {
        let gpu = &self.gpu;
        // SAFETY : ressources possédées, plus utilisées par le GPU (attendu par
        // l'appelant) ; une vue nulle est ignorée par Vulkan. Une image empruntée reste
        // à sa chaîne d'échange.
        unsafe {
            (gpu.fns.vkDestroyImageView)(gpu.device, self.view, std::ptr::null());
            if self.memory != 0 {
                (gpu.fns.vkDestroyImage)(gpu.device, self.raw, std::ptr::null());
                (gpu.fns.vkFreeMemory)(gpu.device, self.memory, std::ptr::null());
            }
        }
    }
}
/// Plage de la couleur : tous les niveaux de mipmap, couche unique.
pub(super) fn color_range() -> VkImageSubresourceRange {
    VkImageSubresourceRange {
        aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
        baseMipLevel: 0,
        levelCount: VK_REMAINING_MIP_LEVELS,
        baseArrayLayer: 0,
        layerCount: 1,
    }
}
