//! Encodeur PNG minimal, sans dépendance : RGBA 8 bits, blocs deflate non compressés.
use crate::{ImageError, ui::scene::Image};

fn crc32(chunks: &[&[u8]]) -> u32 {
    let mut crc = !0_u32;
    for chunk in chunks {
        for &byte in *chunk {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for chunk in data.chunks(5552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend((data.len() as u32).to_be_bytes());
    output.extend(kind);
    output.extend(data);
    output.extend(crc32(&[kind, data]).to_be_bytes());
}

/// Encoder une image BGRA prémultipliée en PNG RGBA (alpha non prémultiplié).
pub(super) fn encode(image: &Image) -> crate::Result<Vec<u8>> {
    if image.width == 0
        || image.height == 0
        || image.width > 16384
        || image.height > 16384
        || image.pixels.len() != image.width as usize * image.height as usize * 4
    {
        return Err(ImageError::InvalidPixels.into());
    }
    let row = image.width as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * image.height as usize);
    for line in image.pixels.chunks_exact(row) {
        // Filtre 0 (aucun) pour chaque ligne.
        raw.push(0);
        for pixel in line.as_chunks::<4>().0 {
            let alpha = u32::from(pixel[3]);
            // Alpha nul : pixel entièrement transparent, couleur indifférente (0).
            let straight = |c: u8| {
                (u32::from(c) * 255 + alpha / 2)
                    .checked_div(alpha)
                    .map_or(0, |value| value.min(255) as u8)
            };
            raw.extend([
                straight(pixel[2]),
                straight(pixel[1]),
                straight(pixel[0]),
                pixel[3],
            ]);
        }
    }
    let mut zlib = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    if blocks.peek().is_none() {
        zlib.extend([1, 0, 0, 0xff, 0xff]);
    }
    while let Some(block) = blocks.next() {
        let length = block.len() as u16;
        zlib.push(u8::from(blocks.peek().is_none()));
        zlib.extend(length.to_le_bytes());
        zlib.extend((!length).to_le_bytes());
        zlib.extend(block);
    }
    zlib.extend(adler32(&raw).to_be_bytes());
    let mut header = Vec::with_capacity(13);
    header.extend(image.width.to_be_bytes());
    header.extend(image.height.to_be_bytes());
    // 8 bits, RGBA, compression deflate, filtrage adaptatif, non entrelacé.
    header.extend([8, 6, 0, 0, 0]);
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut output, b"IHDR", &header);
    chunk(&mut output, b"IDAT", &zlib);
    chunk(&mut output, b"IEND", &[]);
    Ok(output)
}

pub(super) fn write(image: &Image, path: &std::path::Path) -> crate::Result<()> {
    let bytes = encode(image)?;
    if path.to_string_lossy().contains('\0') {
        return Err(crate::Error::invalid_input(
            "PNG path contains a NUL character",
        ));
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoded_png_has_valid_header_checksums_and_straight_alpha() {
        assert_eq!(crc32(&[b"IEND"]), 0xae42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
        // Pixel BGRA prémultiplié : rouge à 50 % d'opacité.
        let image = Image::new(2, 1, vec![0, 0, 128, 128, 0, 0, 0, 0]).unwrap();
        let png = encode(&image).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR"));
        assert_eq!(crate::resources::inspect_image(&png).unwrap().width, 2);
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]));
        // Données brutes stockées : filtre, puis RGBA non prémultiplié.
        let raw = [0, 255, 0, 0, 128, 0, 0, 0, 0];
        assert!(png.windows(raw.len()).any(|w| w == raw));
        assert!(
            encode(&Image {
                width: 3,
                height: 1,
                pixels: vec![0; 4],
            })
            .is_err()
        );
    }
}
