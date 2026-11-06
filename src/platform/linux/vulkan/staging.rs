//! Tampon intermédiaire des envois vers le GPU (glyphes de l'atlas, textures des images
//! et des rastérisations) : les pixels attendent dans la mémoire du processus pendant la
//! traduction de l'image, puis sont écrits d'un bloc dans un tampon projeté que les
//! copies enregistrées en tête de l'image lisent. Ce tampon est réécrit à chaque image :
//! le travail précédent doit être terminé.
use super::{device::Gpu, memory::Buffer};
use crate::platform::linux::ffi::vulkan::VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
use std::rc::Rc;

/// Au-delà, le tampon projeté est libéré après l'image qui l'a employé.
const KEEP_BYTES: u64 = 4 << 20;

pub(super) struct Staging {
    bytes: Vec<u8>,
    buffer: Option<Buffer>,
    gpu: Rc<Gpu>,
}
impl Staging {
    pub fn new(gpu: &Rc<Gpu>) -> Self {
        Self {
            bytes: Vec::new(),
            buffer: None,
            gpu: Rc::clone(gpu),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    /// Ajoute `rows` (lignes de `row` octets chacune, prises dans `pixels` au pas
    /// `stride`) ; renvoie leur début, aligné sur 4 octets.
    pub fn push(&mut self, pixels: &[u8], stride: usize, row: usize, rows: usize) -> usize {
        let offset = self.bytes.len().next_multiple_of(4);
        self.bytes.resize(offset, 0);
        self.bytes.reserve(row * rows);
        for line in pixels.chunks(stride).take(rows) {
            self.bytes.extend_from_slice(&line[..row]);
        }
        offset
    }
    /// Écrit les octets en attente dans le tampon projeté (agrandi si besoin) et le
    /// renvoie ; les débuts rendus par `push` y restent valides jusqu'à l'image suivante.
    pub fn send(&mut self) -> crate::Result<u64> {
        let size = self.bytes.len() as u64;
        if self.buffer.as_ref().is_none_or(|b| b.size < size) {
            self.buffer = None;
            self.buffer = Some(Buffer::new(
                &self.gpu,
                size.next_power_of_two(),
                VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
            )?);
        }
        let buffer = self.buffer.as_mut().expect("staging buffer");
        buffer.write_at(0, &self.bytes);
        self.bytes.clear();
        Ok(buffer.raw)
    }
    /// Oublie les octets en attente (enregistrement abandonné : les copies qui devaient
    /// les lire n'auront pas lieu).
    pub fn clear(&mut self) {
        self.bytes.clear();
    }
    /// Rend le tampon projeté et la mémoire des envois, s'il n'en reste aucun en attente.
    pub fn release(&mut self) {
        if self.bytes.is_empty() {
            self.buffer = None;
            self.bytes = Vec::new();
        }
    }
    /// Après une image terminée : un grand tampon (chargement d'images) est rendu.
    pub fn trim(&mut self) {
        if self.buffer.as_ref().is_some_and(|b| b.size > KEEP_BYTES) {
            self.buffer = None;
        }
        if self.bytes.is_empty() && self.bytes.capacity() > KEEP_BYTES as usize {
            self.bytes = Vec::new();
        }
    }
}
