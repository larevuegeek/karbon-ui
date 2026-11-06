use crate::{Error, ImageError, task::CancellationToken, ui::scene::Image};
use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

/// Encoded image format recognised by [`inspect_image`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// PNG image.
    Png,
    /// JPEG image.
    Jpeg,
}
/// Format and dimensions read from an encoded image header, without decoding it.
#[derive(Debug, Clone, Copy)]
pub struct EncodedImageInfo {
    /// Encoded format.
    pub format: ImageFormat,
    /// Source width in pixels.
    pub width: u32,
    /// Source height in pixels.
    pub height: u32,
}
/// Limits applied by [`load_image_file`] before and after decoding.
///
/// The default allows 8 MiB files, 16 megapixel sources at most 8192 pixels per edge,
/// and decodes to thumbnails of at most 192 pixels per edge.
#[derive(Debug, Clone, Copy)]
pub struct ImageLoadOptions {
    /// Maximum file size in bytes (1 to 64 MiB).
    pub max_file_bytes: usize,
    /// Maximum source pixel count, `width * height` (1 to 64,000,000).
    pub max_source_pixels: u64,
    /// Maximum source width or height in pixels (1 to 16384).
    pub max_source_edge: u32,
    /// Maximum width or height of the decoded thumbnail in pixels (1 to 2048).
    pub thumbnail_edge: u32,
}
impl Default for ImageLoadOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 8 * 1024 * 1024,
            max_source_pixels: 16_000_000,
            max_source_edge: 8192,
            thumbnail_edge: 192,
        }
    }
}
impl ImageLoadOptions {
    /// Check that every limit is within its allowed range.
    ///
    /// # Errors
    ///
    /// [`ImageError::InvalidLimits`] when a limit is zero or above its maximum.
    pub fn validate(self) -> crate::Result<()> {
        if self.max_file_bytes == 0
            || self.max_file_bytes > 64 * 1024 * 1024
            || self.max_source_pixels == 0
            || self.max_source_pixels > 64_000_000
            || self.max_source_edge == 0
            || self.max_source_edge > 16384
            || self.thumbnail_edge == 0
            || self.thumbnail_edge > 2048
        {
            return Err(ImageError::InvalidLimits.into());
        }
        Ok(())
    }
    /// Check the options, then that a `width` x `height` source fits the edge and pixel
    /// limits.
    ///
    /// # Errors
    ///
    /// [`ImageError::InvalidLimits`] when the options are invalid,
    /// [`ImageError::Malformed`] when a dimension is zero and [`ImageError::TooLarge`]
    /// when the dimensions exceed the budget.
    pub fn check_dimensions(self, width: u32, height: u32) -> crate::Result<()> {
        self.validate()?;
        if width == 0 || height == 0 {
            return Err(malformed("zero image dimension"));
        }
        if width > self.max_source_edge
            || height > self.max_source_edge
            || u64::from(width) * u64::from(height) > self.max_source_pixels
        {
            return Err(too_large("source dimensions exceed the configured limits"));
        }
        Ok(())
    }
}
/// Image decoder that can be shared with background tasks.
///
/// Obtained from [`Resources::image_decoder`](crate::application::Resources::image_decoder);
/// it carries no native handle across threads.
pub trait ImageDecoder: Send + Sync {
    /// Produce a premultiplied BGRA thumbnail, never larger than the source.
    /// The implementation keeps its native resources confined to the calling thread.
    ///
    /// # Errors
    ///
    /// Any [`Error`]; the built-in decoder returns [`Error::Image`] for invalid or
    /// out-of-budget data.
    fn decode(&self, bytes: &[u8], options: ImageLoadOptions) -> crate::Result<Image>;
}
/// Image produced by [`load_image_file`], with source information and timings.
pub struct LoadedImage {
    /// Decoded thumbnail, ready to draw.
    pub image: Arc<Image>,
    /// Format and dimensions of the source file.
    pub source: EncodedImageInfo,
    /// Size of the source file in bytes.
    pub file_bytes: usize,
    /// Time spent reading and inspecting the file.
    pub read_time: Duration,
    /// Time spent decoding the thumbnail.
    pub decode_time: Duration,
}
fn malformed(reason: &'static str) -> Error {
    ImageError::Malformed { reason }.into()
}
fn too_large(reason: &'static str) -> Error {
    ImageError::TooLarge { reason }.into()
}
/// Read the format and dimensions of a PNG or JPEG image from its bytes, without
/// decoding pixels.
///
/// # Errors
///
/// [`ImageError::UnsupportedFormat`] for other formats and [`ImageError::Malformed`] for
/// a malformed header.
pub fn inspect_image(bytes: &[u8]) -> crate::Result<EncodedImageInfo> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if bytes.len() < 33 || bytes[8..12] != 13_u32.to_be_bytes() || &bytes[12..16] != b"IHDR" {
            return Err(malformed("truncated PNG header"));
        }
        return Ok(EncodedImageInfo {
            format: ImageFormat::Png,
            width: u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            height: u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        });
    }
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err(ImageError::UnsupportedFormat.into());
    }
    let mut offset = 2;
    while offset < bytes.len() {
        if bytes[offset] != 0xff {
            return Err(malformed("invalid JPEG marker"));
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        let marker = *bytes
            .get(offset)
            .ok_or_else(|| malformed("truncated JPEG"))?;
        offset += 1;
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if marker == 0xda || marker == 0xd9 || marker == 0x00 {
            break;
        }
        let length = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| malformed("truncated JPEG"))?;
        let length = u16::from_be_bytes(length.try_into().unwrap()) as usize;
        if length < 2
            || offset
                .checked_add(length)
                .is_none_or(|end| end > bytes.len())
        {
            return Err(malformed("truncated JPEG segment"));
        }
        if (0xc0..=0xcf).contains(&marker) && ![0xc4, 0xc8, 0xcc].contains(&marker) {
            if length < 8 || bytes[offset + 7] == 0 || length < 8 + 3 * bytes[offset + 7] as usize {
                return Err(malformed("incomplete JPEG dimensions"));
            }
            return Ok(EncodedImageInfo {
                format: ImageFormat::Jpeg,
                width: u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]) as u32,
                height: u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as u32,
            });
        }
        offset += length;
    }
    Err(malformed("missing JPEG dimensions"))
}
fn cancelled(cancel: &CancellationToken) -> crate::Result<()> {
    if cancel.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
/// Read, check and decode an image file into a thumbnail, within `options`' limits.
///
/// Intended for background tasks: `cancel` is checked between reads and around
/// decoding, and a cancelled load fails with [`Error::Cancelled`].
///
/// # Errors
///
/// [`Error::Io`] when opening or reading the file fails, [`Error::Cancelled`] when
/// `cancel` is triggered, [`Error::InvalidInput`] when `path` is not a regular file,
/// [`Error::Image`] when the options are invalid, the file is not PNG/JPEG, exceeds a limit, or the decoder returns an out-of-budget
/// thumbnail, and any error from `decoder`.
pub fn load_image_file(
    decoder: &dyn ImageDecoder,
    path: &Path,
    options: ImageLoadOptions,
    cancel: &CancellationToken,
) -> crate::Result<LoadedImage> {
    options.validate()?;
    cancelled(cancel)?;
    let start = Instant::now();
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Error::invalid_input("image path is not a regular file"));
    }
    if metadata.len() > options.max_file_bytes as u64 {
        return Err(too_large("file larger than the file size limit"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        cancelled(cancel)?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if count > options.max_file_bytes - bytes.len() {
            return Err(too_large("file larger than the file size limit"));
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    let source = inspect_image(&bytes)?;
    options.check_dimensions(source.width, source.height)?;
    let read_time = start.elapsed();
    cancelled(cancel)?;
    let decode_start = Instant::now();
    let image = decoder.decode(&bytes, options)?;
    cancelled(cancel)?;
    if image.width == 0
        || image.height == 0
        || image.width > options.thumbnail_edge
        || image.height > options.thumbnail_edge
        || image.width > source.width
        || image.height > source.height
    {
        return Err(too_large("decoded thumbnail exceeds the thumbnail limits"));
    }
    let image = Image::new(image.width, image.height, image.pixels)?;
    Ok(LoadedImage {
        image: Arc::new(image),
        source,
        file_bytes: bytes.len(),
        read_time,
        decode_time: decode_start.elapsed(),
    })
}

#[cfg(test)]
mod tests;
