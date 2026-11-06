//! Décodage d'images par GDI+ : PNG embarqué, données invalides, miniature produite par
//! le décodeur de fond sur un autre thread.
use super::super::imaging::DrawingEngine;
use crate::{ImageError, resources::ImageLoadOptions};

#[test]
fn decodes_embedded_png_and_rejects_invalid_data() {
    let engine = DrawingEngine::initialize().unwrap();
    let image = engine
        .decode(include_bytes!(
            "../../../../tests/fixtures/images/cover.png"
        ))
        .unwrap();
    assert_eq!((image.width, image.height), (256, 256));
    assert_eq!(image.pixels.len(), 256 * 256 * 4);
    assert!(image.pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
    assert!(matches!(
        engine.decode(b"not an image"),
        Err(crate::Error::Image(ImageError::Decode))
    ));
}
#[test]
fn background_decoder_can_outlive_ui_resources_and_runs_on_its_worker_thread() {
    use crate::application::Resources;
    let engine = DrawingEngine::initialize().unwrap();
    let decoder = engine.image_decoder().unwrap();
    drop(engine);
    let image = std::thread::spawn(move || {
        decoder.decode(
            include_bytes!("../../../../tests/fixtures/images/cover.png"),
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
