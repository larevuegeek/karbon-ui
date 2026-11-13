//! Pipelines des passages de filtre (un par fonction de `filters.metal`, cible RGBA 16
//! bits flottants, sans mélange) et encodage d'un passage.
use super::{Kernel, Params, Pass, Source};
use crate::platform::macos::metal::{device::Texture, submit::CommandBuffer};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLDevice, MTLLibrary, MTLLoadAction,
    MTLPixelFormat, MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLStoreAction, MTLViewport,
};

/// Format des textures intermédiaires des filtres.
pub(in crate::platform::macos::metal) const FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA16Float;

type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;
pub(in crate::platform::macos::metal) struct Pipelines {
    kernels: [Pipeline; 6],
}
fn failure(operation: &'static str) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: 0x8000_4005_u32 as i32,
    }
}
impl Pipelines {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        library: &ProtocolObject<dyn MTLLibrary>,
    ) -> crate::Result<Self> {
        let function = |name: &str| {
            library
                .newFunctionWithName(&NSString::from_str(name))
                .ok_or_else(|| failure("newFunctionWithName"))
        };
        let vertex = function("filter_vertex")?;
        let pipeline = |name: &str| -> crate::Result<Pipeline> {
            let descriptor = MTLRenderPipelineDescriptor::new();
            descriptor.setVertexFunction(Some(&vertex));
            let fragment = function(name)?;
            descriptor.setFragmentFunction(Some(&fragment));
            // SAFETY : l'attachement 0 existe toujours.
            let color = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
            color.setPixelFormat(FORMAT);
            device
                .newRenderPipelineStateWithDescriptor_error(&descriptor)
                .map_err(|error| {
                    eprintln!(
                        "karbon-ui: Metal pipeline {name}: {}",
                        error.localizedDescription()
                    );
                    failure("newRenderPipelineState")
                })
        };
        Ok(Self {
            kernels: [
                pipeline("filter_downsample")?,
                pipeline("filter_blur")?,
                pipeline("filter_upsample")?,
                pipeline("filter_matrix")?,
                pipeline("filter_shade")?,
                pipeline("filter_merge")?,
            ],
        })
    }
    fn get(&self, kernel: Kernel) -> &Pipeline {
        &self.kernels[kernel as usize]
    }
}
/// Encode `pass` dans `cb` ; `layers` donne la texture de chaque couche de l'image.
pub(in crate::platform::macos::metal) fn encode(
    cb: &CommandBuffer,
    pipelines: &Pipelines,
    pass: &Pass,
    layers: &[Texture],
) -> crate::Result<()> {
    let descriptor = MTLRenderPassDescriptor::renderPassDescriptor();
    // SAFETY : l'attachement 0 existe toujours.
    let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
    attachment.setTexture(Some(&pass.target));
    // Effacée en entier : au-delà de la zone utile, la texture reste transparente (bords
    // échantillonnés par la recomposition).
    attachment.setLoadAction(MTLLoadAction::Clear);
    attachment.setStoreAction(MTLStoreAction::Store);
    attachment.setClearColor(MTLClearColor {
        red: 0.,
        green: 0.,
        blue: 0.,
        alpha: 0.,
    });
    let encoder = cb
        .renderCommandEncoderWithDescriptor(&descriptor)
        .ok_or_else(|| failure("renderCommandEncoderWithDescriptor"))?;
    encoder.setRenderPipelineState(pipelines.get(pass.kernel));
    encoder.setViewport(MTLViewport {
        originX: 0.,
        originY: 0.,
        width: pass.size[0] as f64,
        height: pass.size[1] as f64,
        znear: 0.,
        zfar: 1.,
    });
    let source = match &pass.source {
        Source::Layer(index) => &layers[*index],
        Source::Texture(texture) => texture,
    };
    let params: &Params = &pass.params;
    let bytes = std::ptr::NonNull::from(params).cast();
    // SAFETY : `params` est copié à l'appel (≤ 4 Ko, `repr(C)` miroir du shader) ; les
    // textures vivent jusqu'à la fin du tampon de commandes (retenues par Metal).
    unsafe {
        encoder.setFragmentBytes_length_atIndex(bytes, std::mem::size_of::<Params>(), 0);
        encoder.setFragmentTexture_atIndex(Some(source), 0);
        if let Some(below) = &pass.below {
            encoder.setFragmentTexture_atIndex(Some(below), 1);
        }
        encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::TriangleStrip, 0, 4);
    }
    encoder.endEncoding();
    Ok(())
}
