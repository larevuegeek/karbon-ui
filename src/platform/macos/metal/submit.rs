//! Encodage d'une image construite (`encode/`) dans un tampon de commandes Metal : calcul
//! des mipmaps des nouvelles images, puis les étapes dans leur ordre : passages de rendu
//! des couches (une couche avant la recomposition qui la lit, la cible finale en dernier)
//! et passages de filtre.
//!
//! Les données par image (instances, arrêts, découpes) vont dans un anneau de trois
//! jeux de tampons ; un jeu n'est réécrit qu'une fois le GPU sorti de l'image qui l'a lu.
use super::{
    device::{Buffer, Gpu, Texture, write},
    encode::{Bind, Built, Layer, Step},
    instance::Frame,
    resources::Resources,
};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_metal::{
    MTLBlitCommandEncoder, MTLBuffer, MTLClearColor, MTLCommandBuffer, MTLCommandEncoder,
    MTLCommandQueue, MTLLoadAction, MTLPrimitiveType, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLScissorRect, MTLStoreAction,
};

pub(super) type CommandBuffer = Retained<ProtocolObject<dyn MTLCommandBuffer>>;

#[derive(Default)]
struct Slot {
    buffers: [Option<Buffer>; 3],
    /// Dernier tampon de commandes qui a lu ce jeu.
    reader: Option<CommandBuffer>,
}
#[derive(Default)]
pub(super) struct Ring {
    slots: [Slot; 3],
    next: usize,
    /// Dernier tampon de commandes soumis.
    pub last: Option<CommandBuffer>,
}
impl Ring {
    /// Octets des tampons de l'anneau.
    pub fn bytes(&self) -> usize {
        self.slots
            .iter()
            .flat_map(|s| s.buffers.iter().flatten())
            .map(|b| b.length())
            .sum()
    }
    /// Libère les tampons (recréés à la prochaine image ; le GPU retient ceux qu'une image
    /// en vol lit encore).
    pub fn trim(&mut self) {
        for slot in &mut self.slots {
            slot.buffers = Default::default();
            // Un tampon de commandes terminé retient encore ses cibles (drawable compris).
            if slot
                .reader
                .as_ref()
                .is_some_and(|cb| cb.status() == objc2_metal::MTLCommandBufferStatus::Completed)
            {
                slot.reader = None;
            }
        }
    }
    /// Attend la fin de tout le travail soumis (avant de vider l'atlas).
    pub fn idle(&mut self) {
        if let Some(last) = self.last.take() {
            last.waitUntilCompleted();
        }
    }
}
fn bytes<T>(items: &[T]) -> &[u8] {
    // SAFETY : types `repr(C)` de nombres, sans remplissage (voir `instance.rs`).
    unsafe { std::slice::from_raw_parts(items.as_ptr().cast(), std::mem::size_of_val(items)) }
}
fn failure(operation: &'static str) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: 0x8000_4005_u32 as i32,
    }
}
/// Écrit les morceaux `parts` bout à bout dans le tampon `index` du jeu, agrandi au
/// besoin (sans copie intermédiaire).
fn upload(gpu: &Gpu, slot: &mut Slot, index: usize, parts: &[&[u8]]) -> crate::Result<Buffer> {
    let total = parts.iter().map(|p| p.len()).sum::<usize>();
    let buffer = match &slot.buffers[index] {
        Some(b) if b.length() >= total => b.clone(),
        _ => {
            let b = gpu.upload_buffer(total.next_power_of_two())?;
            slot.buffers[index] = Some(b.clone());
            b
        }
    };
    let mut offset = 0;
    for part in parts {
        write(&buffer, offset, part);
        offset += part.len();
    }
    gpu.modified(&buffer, total);
    Ok(buffer)
}
/// Encode `built` vers `target` (couche 0) dans un nouveau tampon de commandes, non soumis.
pub(super) fn encode(
    gpu: &Gpu,
    ring: &mut Ring,
    resources: &mut Resources,
    built: &Built,
    target: &Texture,
) -> crate::Result<CommandBuffer> {
    let slot_index = ring.next;
    ring.next = (ring.next + 1) % ring.slots.len();
    let slot = &mut ring.slots[slot_index];
    if let Some(reader) = slot.reader.take() {
        reader.waitUntilCompleted();
    }
    let mut offsets = Vec::with_capacity(built.layers.len());
    let mut count = 0;
    for layer in &built.layers {
        offsets.push(count);
        count += layer.instances.len();
    }
    let parts = built
        .layers
        .iter()
        .map(|l| bytes(&l.instances))
        .collect::<Vec<_>>();
    let instance_buffer = upload(gpu, slot, 0, &parts)?;
    let stop_buffer = upload(gpu, slot, 1, &[bytes(&built.stops)])?;
    let clip_buffer = upload(gpu, slot, 2, &[bytes(&built.clips)])?;
    let textures = built
        .layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            if index == 0 {
                Ok(target.clone())
            } else {
                resources.target(gpu, layer.texture_size, super::device::FORMAT)
            }
        })
        .collect::<crate::Result<Vec<_>>>()?;
    let cb = gpu
        .queue
        .commandBuffer()
        .ok_or_else(|| failure("commandBuffer"))?;
    if !built.mipmaps.is_empty() {
        let blit = cb
            .blitCommandEncoder()
            .ok_or_else(|| failure("blitCommandEncoder"))?;
        for texture in &built.mipmaps {
            blit.generateMipmapsForTexture(texture);
        }
        blit.endEncoding();
    }
    let buffers = [&instance_buffer, &stop_buffer, &clip_buffer];
    for step in &built.steps {
        match step {
            Step::Draw(layer) => {
                let pass = Pass {
                    built,
                    layer: &built.layers[*layer],
                    target: &textures[*layer],
                    textures: &textures,
                    offset: offsets[*layer],
                };
                pass.encode(gpu, resources, &cb, buffers)?;
            }
            Step::Filter(pass) => super::filter::encode(&cb, &gpu.filters, pass, &textures)?,
        }
    }
    slot.reader = Some(cb.clone());
    ring.last = Some(cb.clone());
    Ok(cb)
}
/// Passage de rendu d'une couche.
struct Pass<'a> {
    built: &'a Built,
    layer: &'a Layer,
    target: &'a Texture,
    /// Textures des couches (recompositions).
    textures: &'a [Texture],
    /// Première instance de la couche dans le tampon.
    offset: usize,
}
impl Pass<'_> {
    fn encode(
        &self,
        gpu: &Gpu,
        resources: &Resources,
        cb: &CommandBuffer,
        [instances, stops, clips]: [&Buffer; 3],
    ) -> crate::Result<()> {
        let layer = self.layer;
        let pass = MTLRenderPassDescriptor::renderPassDescriptor();
        // SAFETY : l'attachement 0 existe toujours.
        let attachment = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setTexture(Some(self.target));
        attachment.setLoadAction(MTLLoadAction::Clear);
        attachment.setStoreAction(MTLStoreAction::Store);
        let [red, green, blue, alpha] = layer.clear;
        attachment.setClearColor(MTLClearColor {
            red,
            green,
            blue,
            alpha,
        });
        let encoder = cb
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or_else(|| failure("renderCommandEncoderWithDescriptor"))?;
        encoder.setRenderPipelineState(&gpu.pipeline);
        let frame = Frame {
            origin: [layer.origin[0] as f32, layer.origin[1] as f32],
            size: [layer.texture_size[0] as f32, layer.texture_size[1] as f32],
        };
        let frame_ptr = std::ptr::NonNull::from(&frame).cast();
        let frame_len = std::mem::size_of::<Frame>();
        // SAFETY : tampons vivants jusqu'à la fin du tampon de commandes (retenus par
        // Metal) ; indices conformes aux arguments des shaders ; `frame` copié à l'appel.
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(instances), 0, 0);
            encoder.setVertexBytes_length_atIndex(frame_ptr, frame_len, 1);
            encoder.setFragmentBuffer_offset_atIndex(Some(instances), 0, 0);
            encoder.setFragmentBuffer_offset_atIndex(Some(stops), 0, 1);
            encoder.setFragmentBuffer_offset_atIndex(Some(clips), 0, 2);
            encoder.setFragmentBytes_length_atIndex(frame_ptr, frame_len, 3);
        }
        for batch in &layer.batches {
            let [x, y, width, height] = batch.scissor;
            if width == 0 || height == 0 {
                continue;
            }
            let texture = match batch.bind {
                Bind::None => &resources.blank,
                Bind::Texture(i) => &self.built.textures[i],
                Bind::Layer(i) => &self.textures[i],
            };
            encoder.setScissorRect(MTLScissorRect {
                x,
                y,
                width,
                height,
            });
            // SAFETY : la texture vit au moins jusqu'à la fin du tampon de commandes ;
            // les instances `start..start + count` de la couche sont dans le tampon.
            unsafe {
                encoder.setFragmentTexture_atIndex(Some(texture), 0);
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount_baseInstance(
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                    batch.count,
                    self.offset + batch.start,
                );
            }
        }
        encoder.endEncoding();
        Ok(())
    }
}
/// Copie `texture` (BGRA, `width` × `height`) dans un tampon lisible et attend le GPU.
pub(super) fn read_back(
    gpu: &Gpu,
    cb: &CommandBuffer,
    texture: &Texture,
    (width, height): (usize, usize),
) -> crate::Result<Vec<u8>> {
    let buffer = gpu.readback_buffer(width * height * 4)?;
    let blit = cb
        .blitCommandEncoder()
        .ok_or_else(|| failure("blitCommandEncoder"))?;
    // SAFETY : région dans la texture ; tampon de `width * height * 4` octets.
    unsafe {
        blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage(
            texture,
            0,
            0,
            objc2_metal::MTLOrigin { x: 0, y: 0, z: 0 },
            objc2_metal::MTLSize {
                width,
                height,
                depth: 1,
            },
            &buffer,
            0,
            width * 4,
            width * height * 4,
        );
    }
    if gpu.storage.readback_managed {
        // Tampon géré : la copie du GPU revient dans la mémoire du CPU.
        blit.synchronizeResource(ProtocolObject::from_ref(&*buffer));
    }
    blit.endEncoding();
    cb.commit();
    cb.waitUntilCompleted();
    if let Some(error) = cb.error() {
        eprintln!("karbon-ui: Metal : {}", error.localizedDescription());
        return Err(failure("MTLCommandBuffer"));
    }
    let mut pixels = vec![0u8; width * height * 4];
    // SAFETY : tampon partagé de `width * height * 4` octets, écrit par le GPU terminé.
    unsafe {
        std::ptr::copy_nonoverlapping(
            buffer.contents().as_ptr().cast::<u8>(),
            pixels.as_mut_ptr(),
            pixels.len(),
        );
    }
    Ok(pixels)
}
