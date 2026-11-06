/// Premultiplied-alpha BGRA pixels, shared by the CPU and GPU paths.
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major BGRA bytes, `width * height * 4` long.
    pub pixels: Vec<u8>,
}
impl Image {
    /// Validate and wrap a pixel buffer.
    ///
    /// # Errors
    ///
    /// [`ImageError::InvalidPixels`](crate::ImageError::InvalidPixels) if a dimension is
    /// zero, the buffer length is not `width * height * 4`, or a colour channel exceeds its
    /// alpha (not premultiplied).
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> crate::Result<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4));
        if width == 0
            || height == 0
            || expected != Some(pixels.len())
            || pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[..3].iter().any(|&c| c > p[3]))
        {
            return Err(crate::ImageError::InvalidPixels.into());
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}
