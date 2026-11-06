//! Décodage d'images par ImageIO : PNG embarqué, données invalides, miniature du
//! décodeur de fond sur un autre thread, aller-retour PNG.
use super::super::imaging::{Imaging, decode, write_png};
use crate::{ImageError, application::Resources, resources::ImageLoadOptions};

const COVER: &[u8] = include_bytes!("../../../../tests/fixtures/images/cover.png");

#[test]
fn decodes_embedded_png_and_rejects_invalid_data() {
    let image = decode(COVER).unwrap();
    assert_eq!((image.width, image.height), (256, 256));
    assert_eq!(image.pixels.len(), 256 * 256 * 4);
    assert!(image.pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
    assert!(matches!(
        decode(b"not an image"),
        Err(crate::Error::Image(ImageError::Decode))
    ));
}
#[test]
fn background_decoder_runs_on_its_worker_thread_and_keeps_the_aspect_ratio() {
    let decoder = Imaging.image_decoder().unwrap();
    let image = std::thread::spawn(move || {
        decoder.decode(
            COVER,
            ImageLoadOptions {
                thumbnail_edge: 96,
                ..Default::default()
            },
        )
    })
    .join()
    .unwrap()
    .unwrap();
    assert_eq!((image.width, image.height), (96, 96));
    assert_eq!(image.pixels.len(), 96 * 96 * 4);
}
#[test]
fn png_export_round_trips_premultiplied_pixels() {
    let mut pixels = Vec::new();
    for y in 0..4u8 {
        for x in 0..4u8 {
            // Opaque à gauche, semi-transparent à droite.
            let alpha = if x < 2 { 255 } else { 128 };
            let blue = (u16::from(y) * 60).min(u16::from(alpha)) as u8;
            pixels.extend_from_slice(&[blue, 0, alpha / 2, alpha]);
        }
    }
    let image = crate::ui::scene::Image::new(4, 4, pixels.clone()).unwrap();
    let path = std::env::temp_dir().join(format!("karbon-png-{}.png", std::process::id()));
    write_png(&image, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(&bytes[1..4], b"PNG");
    let back = decode(&bytes).unwrap();
    assert_eq!((back.width, back.height), (4, 4));
    for (a, b) in back.pixels.iter().zip(&pixels) {
        assert!(a.abs_diff(*b) <= 2, "{a} != {b}");
    }
}
