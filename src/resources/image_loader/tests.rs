use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Decoder {
    calls: AtomicUsize,
    edge: u32,
}
impl ImageDecoder for Decoder {
    fn decode(&self, _: &[u8], _: ImageLoadOptions) -> crate::Result<Image> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Image::new(
            self.edge,
            self.edge,
            vec![0; (self.edge * self.edge * 4) as usize],
        )
    }
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "karbon-ui-image-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        if let (Ok(path), Ok(root)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
            && path.starts_with(root)
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("karbon-ui-image-test-"))
        {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
const PNG: &[u8] = include_bytes!("../../../tests/fixtures/images/cover.png");
#[test]
fn headers_read_png_and_jpeg_baseline_or_progressive_without_decoding_pixels() {
    let info = inspect_image(PNG).unwrap();
    assert_eq!((info.width, info.height), (256, 256));
    assert_eq!(info.format, ImageFormat::Png);
    for marker in [0xc0, 0xc2] {
        let jpeg = [
            0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, marker, 0, 11, 8, 0, 120, 0, 240, 1, 1, 0x11,
            0,
        ];
        let info = inspect_image(&jpeg).unwrap();
        assert_eq!((info.width, info.height), (240, 120));
        assert_eq!(info.format, ImageFormat::Jpeg);
        for length in 2..jpeg.len() {
            assert!(matches!(
                inspect_image(&jpeg[..length]),
                Err(Error::Image(ImageError::Malformed { .. }))
            ));
        }
        for length in 0..2 {
            assert!(inspect_image(&jpeg[..length]).is_err());
        }
    }
    assert!(matches!(
        inspect_image(&[0xff, 0xd8, 0xff, 0xe0, 0, 0]),
        Err(Error::Image(ImageError::Malformed { .. }))
    ));
    assert!(matches!(
        inspect_image(&[0xff, 0xd8, 0xff, 0xda]),
        Err(Error::Image(ImageError::Malformed { .. }))
    ));
    assert!(matches!(
        inspect_image(b"GIF89a"),
        Err(Error::Image(ImageError::UnsupportedFormat))
    ));
}
#[test]
fn file_loader_bounds_reading_and_checks_headers_before_calling_decoder() {
    let temp = Temp::new();
    let path = temp.0.join("cover.png");
    std::fs::write(&path, PNG).unwrap();
    let decoder = Decoder {
        calls: AtomicUsize::new(0),
        edge: 64,
    };
    let cancel = CancellationToken::new();
    let loaded = load_image_file(&decoder, &path, ImageLoadOptions::default(), &cancel).unwrap();
    assert_eq!(loaded.file_bytes, PNG.len());
    assert_eq!(loaded.source.width, 256);
    assert_eq!(loaded.image.width, 64);
    assert!(matches!(
        load_image_file(
            &decoder,
            &path,
            ImageLoadOptions {
                max_file_bytes: 1,
                ..Default::default()
            },
            &cancel
        ),
        Err(Error::Image(ImageError::TooLarge { .. }))
    ));
    assert!(matches!(
        load_image_file(
            &decoder,
            &path,
            ImageLoadOptions {
                max_source_pixels: 100,
                ..Default::default()
            },
            &cancel
        ),
        Err(Error::Image(ImageError::TooLarge { .. }))
    ));
    assert_eq!(decoder.calls.load(Ordering::Relaxed), 1);
    let mut oversized = PNG.to_vec();
    oversized[16..20].copy_from_slice(&100_000_u32.to_be_bytes());
    std::fs::write(&path, oversized).unwrap();
    assert!(matches!(
        load_image_file(&decoder, &path, Default::default(), &cancel),
        Err(Error::Image(ImageError::TooLarge { .. }))
    ));
    assert_eq!(decoder.calls.load(Ordering::Relaxed), 1);
    assert!(matches!(
        load_image_file(&decoder, &temp.0, Default::default(), &cancel),
        Err(Error::InvalidInput { .. }) | Err(Error::Io(_))
    ));
}
#[test]
fn cancellation_precedes_io_and_decoder_output_is_validated() {
    let decoder = Decoder {
        calls: AtomicUsize::new(0),
        edge: 256,
    };
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        load_image_file(
            &decoder,
            Path::new("absent-karbon-ui-image"),
            Default::default(),
            &cancelled
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        load_image_file(
            &decoder,
            Path::new("absent-karbon-ui-image"),
            Default::default(),
            &CancellationToken::new()
        ),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound
    ));
    let temp = Temp::new();
    let path = temp.0.join("cover.png");
    std::fs::write(&path, PNG).unwrap();
    assert!(matches!(
        load_image_file(
            &decoder,
            &path,
            Default::default(),
            &CancellationToken::new()
        ),
        Err(Error::Image(ImageError::TooLarge { .. }))
    ));
    assert!(matches!(
        ImageLoadOptions {
            thumbnail_edge: 0,
            ..Default::default()
        }
        .validate(),
        Err(Error::Image(ImageError::InvalidLimits))
    ));
    assert!(matches!(
        ImageLoadOptions::default().check_dimensions(0, 20),
        Err(Error::Image(ImageError::Malformed { .. }))
    ));
    assert!(matches!(
        ImageLoadOptions::default().check_dimensions(u32::MAX, u32::MAX),
        Err(Error::Image(ImageError::TooLarge { .. }))
    ));
}
