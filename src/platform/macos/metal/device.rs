//! Périphérique Metal, file de commandes, bibliothèque de shaders compilée au lancement
//! depuis la source MSL embarquée, pipeline et modes de stockage.
//!
//! Choix du GPU : sur un Mac à deux GPU (Mac Intel à graphiques commutables), le GPU
//! économe (`isLowPower`) est préféré et `MTLCreateSystemDefaultDevice`, qui forcerait
//! le passage au GPU dédié, n'est pas appelé ; le système garde la main sur la
//! commutation (une application empaquetée déclare `NSSupportsAutomaticGraphicsSwitching`
//! dans son `Info.plist`). Avec un seul GPU (Apple Silicon), c'est le périphérique par
//! défaut.
//!
//! Stockage des ressources remplies par le CPU (`Storage`) : tampons partagés en mémoire
//! unifiée (Apple Silicon, GPU Intel intégrés), gérés (`Managed`, copie signalée par
//! `didModifyRange`) sur un GPU dédié ; textures partagées seulement sur un GPU Apple
//! (macOS refuse les textures `Shared` sur les GPU Intel et AMD), gérées sinon.
//! `KARBON_METAL_STORAGE=managed` force le stockage `Managed` partout, relecture comprise
//! (synchronisée par un blit `synchronizeResource`), pour exercer sur un Mac Apple
//! Silicon le chemin d'un GPU dédié (tests, voir `docs/tests.md`).
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlendFactor, MTLBuffer, MTLCommandQueue, MTLDevice, MTLLibrary, MTLPixelFormat,
    MTLRenderPipelineColorAttachmentDescriptor, MTLRenderPipelineDescriptor,
    MTLRenderPipelineState, MTLResourceOptions, MTLStorageMode, MTLTexture, MTLTextureDescriptor,
    MTLTextureUsage,
};

pub(super) type Device = ProtocolObject<dyn MTLDevice>;
pub(super) type Texture = Retained<ProtocolObject<dyn MTLTexture>>;
pub(super) type Buffer = Retained<ProtocolObject<dyn MTLBuffer>>;

/// Source MSL : quads, puis passages de filtre.
const SOURCE: &str = concat!(
    include_str!("shaders.metal"),
    include_str!("filter/filters.metal")
);
/// Format des cibles : BGRA 8 bits non linéaire, comme les bitmaps Core Graphics (le
/// mélange se fait dans l'espace sRGB encodé, à l'identique).
pub(super) const FORMAT: MTLPixelFormat = MTLPixelFormat::BGRA8Unorm;

pub(super) struct Gpu {
    pub device: Retained<Device>,
    pub queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pub pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    /// Passages de filtre (flou, matrices de couleur, ombre).
    pub filters: super::filter::Pipelines,
    pub storage: Storage,
}
/// Modes de stockage des ressources écrites ou relues par le CPU (voir le module).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Storage {
    /// Tampons d'instances : `Managed` (sinon `Shared`).
    pub buffers_managed: bool,
    /// Textures remplies par le CPU : `Managed` (sinon `Shared`).
    pub textures_managed: bool,
    /// Relecture par un tampon `Managed` synchronisé par blit (sinon `Shared`).
    pub readback_managed: bool,
}
impl Storage {
    /// Tout en `Managed` : chemin d'un GPU dédié.
    pub const MANAGED: Self = Self {
        buffers_managed: true,
        textures_managed: true,
        readback_managed: true,
    };
    /// Modes retenus pour `device`, ou `Managed` partout sous `KARBON_METAL_STORAGE=managed`.
    pub fn of(device: &Device) -> Self {
        if std::env::var("KARBON_METAL_STORAGE").is_ok_and(|v| v == "managed") {
            return Self::MANAGED;
        }
        let apple = device.supportsFamily(objc2_metal::MTLGPUFamily::Apple1);
        Self {
            buffers_managed: !device.hasUnifiedMemory(),
            textures_managed: !apple,
            readback_managed: false,
        }
    }
}
fn failure(operation: &'static str) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: 0x8000_4005_u32 as i32,
    }
}
/// Périphérique retenu (voir la documentation du module), ou `None` sans GPU Metal.
pub(super) fn system_device() -> Option<Retained<Device>> {
    let all = objc2_metal::MTLCopyAllDevices().to_vec();
    if all.len() > 1
        && let Some(low) = all
            .iter()
            .find(|d| d.isLowPower() && !d.isHeadless() && !d.isRemovable())
    {
        return Some(low.clone());
    }
    objc2_metal::MTLCreateSystemDefaultDevice()
}
impl Gpu {
    /// GPU du système ; `storage` remplace les modes de stockage choisis (tests).
    pub fn new(storage: Option<Storage>) -> crate::Result<Self> {
        let device = system_device().ok_or(crate::Error::Unsupported {
            reason: "no Metal device",
        })?;
        let queue = device
            .newCommandQueue()
            .ok_or_else(|| failure("newCommandQueue"))?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SOURCE), None)
            .map_err(|error| {
                eprintln!(
                    "karbon-ui: shaders Metal : {}",
                    error.localizedDescription()
                );
                failure("newLibraryWithSource")
            })?;
        let function = |name: &str| {
            library
                .newFunctionWithName(&NSString::from_str(name))
                .ok_or_else(|| failure("newFunctionWithName"))
        };
        let descriptor = MTLRenderPipelineDescriptor::new();
        let (vertex, fragment) = (function("quad_vertex")?, function("quad_fragment")?);
        descriptor.setVertexFunction(Some(&vertex));
        descriptor.setFragmentFunction(Some(&fragment));
        // SAFETY : l'attachement 0 existe toujours.
        let color: Retained<MTLRenderPipelineColorAttachmentDescriptor> =
            unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        color.setPixelFormat(FORMAT);
        color.setBlendingEnabled(true);
        color.setSourceRGBBlendFactor(MTLBlendFactor::One);
        color.setSourceAlphaBlendFactor(MTLBlendFactor::One);
        color.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        color.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        let pipeline = device
            .newRenderPipelineStateWithDescriptor_error(&descriptor)
            .map_err(|error| {
                eprintln!(
                    "karbon-ui: pipeline Metal : {}",
                    error.localizedDescription()
                );
                failure("newRenderPipelineState")
            })?;
        let filters = super::filter::Pipelines::new(&device, &library)?;
        let storage = storage.unwrap_or_else(|| Storage::of(&device));
        Ok(Self {
            device,
            queue,
            pipeline,
            filters,
            storage,
        })
    }
    /// Tampon écrit par le CPU à chaque image : partagé en mémoire unifiée, géré
    /// (`Managed`, copie synchronisée par `didModifyRange`) sur un GPU dédié.
    pub fn upload_buffer(&self, bytes: usize) -> crate::Result<Buffer> {
        let mode = if self.storage.buffers_managed {
            MTLResourceOptions::StorageModeManaged
        } else {
            MTLResourceOptions::StorageModeShared
        };
        self.device
            .newBufferWithLength_options(
                bytes.max(256),
                mode | MTLResourceOptions::CPUCacheModeWriteCombined,
            )
            .ok_or_else(|| failure("newBufferWithLength"))
    }
    /// Signale au pilote les `bytes` premiers octets écrits dans `buffer` (tampon géré).
    pub fn modified(&self, buffer: &Buffer, bytes: usize) {
        if self.storage.buffers_managed && bytes > 0 {
            buffer.didModifyRange(objc2_foundation::NSRange::new(0, bytes));
        }
    }
    /// Tampon lu par le CPU (relecture d'une texture rendue) : partagé, ou géré et
    /// synchronisé par `read_back` (`Storage::readback_managed`).
    pub fn readback_buffer(&self, bytes: usize) -> crate::Result<Buffer> {
        let mode = if self.storage.readback_managed {
            MTLResourceOptions::StorageModeManaged
        } else {
            MTLResourceOptions::StorageModeShared
        };
        self.device
            .newBufferWithLength_options(bytes.max(256), mode)
            .ok_or_else(|| failure("newBufferWithLength"))
    }
    /// Texture remplie par le CPU (`replaceRegion`) : partagée sur un GPU Apple, gérée
    /// sinon (le pilote la recopie vers la mémoire du GPU).
    pub fn upload_texture(
        &self,
        format: MTLPixelFormat,
        width: usize,
        height: usize,
        mipmapped: bool,
    ) -> crate::Result<Texture> {
        let mode = if self.storage.textures_managed {
            MTLStorageMode::Managed
        } else {
            MTLStorageMode::Shared
        };
        self.texture(
            format,
            width,
            height,
            mipmapped,
            mode,
            MTLTextureUsage::ShaderRead,
        )
    }
    /// Cible de rendu privée au GPU (groupes à opacité, rendus relus).
    pub fn target_texture(&self, width: usize, height: usize) -> crate::Result<Texture> {
        self.render_texture(FORMAT, width, height)
    }
    /// Cible de rendu privée au GPU au format `format` (couches, filtres).
    pub fn render_texture(
        &self,
        format: MTLPixelFormat,
        width: usize,
        height: usize,
    ) -> crate::Result<Texture> {
        self.texture(
            format,
            width,
            height,
            false,
            MTLStorageMode::Private,
            MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget,
        )
    }
    fn texture(
        &self,
        format: MTLPixelFormat,
        width: usize,
        height: usize,
        mipmapped: bool,
        mode: MTLStorageMode,
        usage: MTLTextureUsage,
    ) -> crate::Result<Texture> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid texture size"));
        }
        // SAFETY : dimensions vérifiées ci-dessus.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                format, width, height, mipmapped,
            )
        };
        descriptor.setStorageMode(mode);
        descriptor.setUsage(usage);
        self.device
            .newTextureWithDescriptor(&descriptor)
            .ok_or_else(|| failure("newTextureWithDescriptor"))
    }
}
/// Copie `bytes` à `offset` dans `buffer` (assez grand) ; l'appelant signale ensuite
/// l'écriture (`Gpu::modified`).
pub(super) fn write(buffer: &Buffer, offset: usize, bytes: &[u8]) {
    assert!(offset + bytes.len() <= buffer.length());
    // SAFETY : tampon CPU de `length()` octets ≥ `offset + bytes.len()` (vérifié) ;
    // aucune lecture GPU en cours sur cet emplacement (anneau de tampons, `submit.rs`).
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            buffer.contents().as_ptr().cast::<u8>().add(offset),
            bytes.len(),
        );
    }
}
/// Remplit la région (`x`, `y`, `width`, `height`) du niveau 0 de `texture`.
pub(super) fn replace(
    texture: &Texture,
    (x, y, width, height): (usize, usize, usize, usize),
    pixels: &[u8],
    row_bytes: usize,
) {
    debug_assert!(pixels.len() >= row_bytes * height);
    let region = objc2_metal::MTLRegion {
        origin: objc2_metal::MTLOrigin { x, y, z: 0 },
        size: objc2_metal::MTLSize {
            width,
            height,
            depth: 1,
        },
    };
    if let Some(bytes) = std::ptr::NonNull::new(pixels.as_ptr().cast_mut()) {
        // SAFETY : `pixels` couvre `height` lignes de `row_bytes` octets, lues pendant
        // l'appel ; la région est dans la texture (allouée par l'appelant).
        unsafe {
            texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region,
                0,
                bytes.cast(),
                row_bytes,
            )
        };
    }
}
