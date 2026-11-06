//! Atlas de glyphes : chaque glyphe est rastérisé une fois par Cairo (`graphics/gpu.rs`,
//! options de police du dessin Cairo) à l'échelle d'affichage et à sa phase au quart de
//! pixel, puis rangé par étagères dans des pages de 1024 × 1024 : masques R8 teintés par
//! le shader, ou BGRA prémultiplié pour les glyphes en couleur (émojis).
//!
//! Les pixels des glyphes nouveaux attendent dans le tampon intermédiaire (`staging.rs`) ;
//! `record` les copie en tête de l'image (page effacée à sa création, puis une copie par
//! glyphe), avant les passages de rendu qui les lisent.
use super::{
    commands::transition,
    device::Gpu,
    memory::{Image, color_range},
    staging::Staging,
};
use crate::platform::linux::{
    ffi::{glib::Object, vulkan::*},
    graphics::gpu::rasterize_glyph,
};
use std::{collections::HashMap, ffi::c_void, rc::Rc};

const PAGE: usize = 1024;
/// Pages au-delà desquelles l'atlas est vidé au début de l'image suivante.
const MAX_PAGES: usize = 8;
/// Étapes et accès des copies et effacements.
pub(super) const TRANSFER: (u64, u64) = (
    VK_PIPELINE_STAGE_2_ALL_TRANSFER_BIT,
    VK_ACCESS_2_TRANSFER_WRITE_BIT,
);
/// Étapes et accès d'une page lue par le shader.
pub(super) const SAMPLED: (u64, u64) = (
    VK_PIPELINE_STAGE_2_FRAGMENT_SHADER_BIT,
    VK_ACCESS_2_SHADER_SAMPLED_READ_BIT,
);

/// Glyphe placé dans l'atlas.
#[derive(Clone, Copy)]
pub(super) struct Glyph {
    pub page: usize,
    /// Coin de l'image du glyphe depuis le pixel de son origine.
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub uv: [f32; 4],
    /// Glyphe en couleur (page BGRA, non teintée).
    pub colored: bool,
}
struct Page {
    image: Image,
    colored: bool,
    /// Étagère courante : ordonnée, hauteur et abscisse libre.
    shelf: (usize, usize, usize),
}
/// Copie en attente : page, rectangle et début des pixels dans le tampon intermédiaire.
struct Upload {
    page: usize,
    rect: [usize; 4],
    offset: usize,
}
#[derive(Hash, PartialEq, Eq)]
struct Key {
    font: u32,
    glyph: u32,
    scale: u32,
    x: u8,
    y: u8,
    colored: bool,
}
pub(super) struct Atlas {
    pages: Vec<Page>,
    glyphs: HashMap<Key, Option<Glyph>>,
    /// Polices gardées vivantes : leur adresse reste leur identité.
    fonts: Vec<Object>,
    uploads: Vec<Upload>,
    gpu: Rc<Gpu>,
}
impl Atlas {
    pub fn new(gpu: &Rc<Gpu>) -> Self {
        Self {
            pages: Vec::new(),
            glyphs: HashMap::new(),
            fonts: Vec::new(),
            uploads: Vec::new(),
            gpu: Rc::clone(gpu),
        }
    }
    /// `true` si l'atlas déborde et doit être vidé.
    pub fn full(&self) -> bool {
        self.pages.len() > MAX_PAGES
    }
    /// Vide l'atlas ; le GPU ne doit plus lire ses pages.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.glyphs.clear();
        self.fonts.clear();
        self.uploads.clear();
    }
    /// Vue de la page `page`.
    pub fn view(&self, page: usize) -> u64 {
        self.pages[page].image.view
    }
    /// Numéro stable de `font` dans l'atlas.
    pub fn font(&mut self, font: &Object) -> u32 {
        if let Some(i) = self.fonts.iter().position(|f| f.as_ptr() == font.as_ptr()) {
            return i as u32;
        }
        self.fonts.push(font.clone());
        self.fonts.len() as u32 - 1
    }
    /// Glyphe `glyph` de la police numéro `id`, à l'échelle `scale` et décalé de
    /// (`x`, `y`) quarts de pixel ; `None` pour un glyphe sans encre. Les pixels d'un
    /// glyphe nouveau vont dans `staging`.
    pub fn glyph(
        &mut self,
        staging: &mut Staging,
        (id, font): (u32, &Object),
        glyph: u32,
        scale: f32,
        (x, y): (u8, u8),
        colored: bool,
    ) -> crate::Result<Option<Glyph>> {
        let key = Key {
            font: id,
            glyph,
            scale: scale.to_bits(),
            x,
            y,
            colored,
        };
        if let Some(found) = self.glyphs.get(&key) {
            return Ok(*found);
        }
        let phase = (f32::from(x) / 4., f32::from(y) / 4.);
        let entry = match rasterize_glyph(font, glyph, scale, phase, colored)? {
            Some(image) => {
                let (width, height) = (image.width, image.height);
                let (page, px, py) = self.place(width, height, colored)?;
                let bytes = if colored { 4 } else { 1 };
                let offset = staging.push(&image.pixels, image.stride, width * bytes, height);
                self.uploads.push(Upload {
                    page,
                    rect: [px, py, width, height],
                    offset,
                });
                let n = PAGE as f32;
                Some(Glyph {
                    page,
                    left: image.left,
                    top: image.top,
                    width,
                    height,
                    uv: [
                        px as f32 / n,
                        py as f32 / n,
                        (px + width) as f32 / n,
                        (py + height) as f32 / n,
                    ],
                    colored,
                })
            }
            None => None,
        };
        self.glyphs.insert(key, entry);
        Ok(entry)
    }
    /// Place un rectangle de `width` × `height` (1 pixel de marge) dans une page.
    fn place(
        &mut self,
        width: usize,
        height: usize,
        colored: bool,
    ) -> crate::Result<(usize, usize, usize)> {
        let (w, h) = (width + 1, height + 1);
        if let Some((index, page)) = self
            .pages
            .iter_mut()
            .enumerate()
            .rev()
            .find(|(_, p)| p.colored == colored)
        {
            let (y, shelf, x) = page.shelf;
            if x + w <= PAGE && h <= shelf {
                page.shelf.2 += w;
                return Ok((index, x, y));
            }
            // Nouvelle étagère sous la courante.
            if y + shelf + h <= PAGE {
                page.shelf = (y + shelf, h, w);
                return Ok((index, 0, y + shelf));
            }
        }
        let format = if colored {
            VK_FORMAT_B8G8R8A8_UNORM
        } else {
            VK_FORMAT_R8_UNORM
        };
        let usage = VK_IMAGE_USAGE_SAMPLED_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
        self.pages.push(Page {
            image: Image::new(&self.gpu, PAGE, PAGE, format, usage)?,
            colored,
            shelf: (0, h, w),
        });
        Ok((self.pages.len() - 1, 0, 0))
    }
    /// Enregistre dans `command` les copies en attente depuis le tampon intermédiaire
    /// `staging` : chaque page touchée passe en destination de copie (effacée si elle est
    /// neuve), reçoit ses glyphes, puis revient en lecture par le shader.
    pub fn record(&mut self, command: *mut c_void, staging: u64) {
        if self.uploads.is_empty() {
            return;
        }
        let gpu = &self.gpu;
        let mut regions = Vec::new();
        for (index, page) in self.pages.iter_mut().enumerate() {
            regions.clear();
            regions.extend(
                self.uploads
                    .iter()
                    .filter(|u| u.page == index)
                    .map(|u| region(u.rect, u.offset)),
            );
            if regions.is_empty() {
                continue;
            }
            let image = &mut page.image;
            let fresh = image.layout == VK_IMAGE_LAYOUT_UNDEFINED;
            let after = if fresh {
                (VK_PIPELINE_STAGE_2_NONE, VK_ACCESS_2_NONE)
            } else {
                (SAMPLED.0, VK_ACCESS_2_NONE)
            };
            transition(
                gpu,
                command,
                image,
                VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                after,
                TRANSFER,
            );
            if fresh {
                let zero = VkClearColorValue { float32: [0.; 4] };
                let range = color_range();
                // SAFETY : tampon en cours d'enregistrement ; image en destination de copie.
                unsafe {
                    (gpu.fns.vkCmdClearColorImage)(
                        command,
                        image.raw,
                        VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        &zero,
                        1,
                        &range,
                    )
                };
                // L'effacement précède les copies (deux écritures de la même image).
                transition(
                    gpu,
                    command,
                    image,
                    VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    TRANSFER,
                    TRANSFER,
                );
            }
            // SAFETY : régions dans la page et dans le tampon intermédiaire (écrit par
            // `Staging::send`, mémoire cohérente : visible par la soumission).
            unsafe {
                (gpu.fns.vkCmdCopyBufferToImage)(
                    command,
                    staging,
                    image.raw,
                    VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    regions.len() as u32,
                    regions.as_ptr(),
                )
            };
            transition(
                gpu,
                command,
                image,
                VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                TRANSFER,
                SAMPLED,
            );
        }
        self.uploads.clear();
    }
}
/// Copie de `width` × `height` pixels jointifs depuis `offset` vers (`x`, `y`).
pub(super) fn region([x, y, width, height]: [usize; 4], offset: usize) -> VkBufferImageCopy {
    VkBufferImageCopy {
        bufferOffset: offset as u64,
        bufferRowLength: 0,
        bufferImageHeight: 0,
        imageSubresource: VkImageSubresourceLayers {
            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
            mipLevel: 0,
            baseArrayLayer: 0,
            layerCount: 1,
        },
        imageOffset: VkOffset3D {
            x: x as i32,
            y: y as i32,
            z: 0,
        },
        imageExtent: VkExtent3D {
            width: width as u32,
            height: height as u32,
            depth: 1,
        },
    }
}
