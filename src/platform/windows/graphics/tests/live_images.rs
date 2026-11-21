//! Images vivantes (`LiveImage`) : l'image suivante d'une source, de même taille, est
//! copiée dans le bitmap de la précédente, et s'affiche au pixel près comme une image
//! neuve ; une autre taille, ou une image ordinaire, prend un bitmap neuf comme avant.
use super::super::*;
use crate::ui::scene::LiveImage;

/// Image opaque de 64 × 48 pixels, différente selon `seed`.
fn frame(seed: u8) -> Arc<Image> {
    let pixels = (0..64 * 48 * 4)
        .map(|i| (i as u8).wrapping_mul(seed).wrapping_add(seed))
        .collect();
    Arc::new(Image::opaque_bgra(64, 48, pixels).unwrap())
}

fn scene(image: &Arc<Image>) -> Scene {
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::Image {
        rect: RectF::new(0., 0., 64., 48.),
        image: Arc::clone(image),
    });
    scene
}

/// Le bitmap gardé pour la seule image de la scène : sa clé, et le bitmap lui-même (une
/// référence de plus, qui l'empêche d'être libéré et son adresse d'être reprise).
fn kept(renderer: &Renderer) -> (usize, ID2D1Bitmap) {
    let canvas = &renderer.target.as_ref().unwrap().canvas;
    assert_eq!(canvas.bitmaps.len(), 1);
    let (key, (_, bitmap)) = canvas.bitmaps.iter().next().unwrap();
    (*key, bitmap.clone())
}

#[test]
fn the_next_image_of_a_live_source_reuses_the_bitmap_and_shows_the_same_pixels() {
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    let view = LiveImage::new();
    let first = frame(3);
    view.set(Arc::clone(&first));
    renderer.render(&scene(&first), 64, 48, 1.).unwrap();
    let (first_key, first_bitmap) = kept(&renderer);
    let second = frame(7);
    view.set(Arc::clone(&second));
    let updated = renderer.render(&scene(&second), 64, 48, 1.).unwrap();
    let (second_key, second_bitmap) = kept(&renderer);
    // Nouvelle clé (nouvelle image), même bitmap natif.
    assert_ne!(first_key, second_key);
    assert_eq!(first_bitmap, second_bitmap);
    // Les mêmes pixels qu'une image neuve dessinée par un autre renderer.
    let mut fresh = Renderer::new(RenderMode::Software).unwrap();
    let plain = Arc::new(Image::new(64, 48, second.pixels.clone()).unwrap());
    assert_eq!(updated, fresh.render(&scene(&plain), 64, 48, 1.).unwrap());
    // Une autre taille : un bitmap neuf.
    let bigger = Arc::new(Image::opaque_bgra(65, 48, vec![1; 65 * 48 * 4]).unwrap());
    view.set(Arc::clone(&bigger));
    renderer.render(&scene(&bigger), 64, 48, 1.).unwrap();
    assert_ne!(kept(&renderer).1, second_bitmap);
    // Une image ordinaire n'emprunte rien.
    let other = frame(9);
    renderer.render(&scene(&other), 64, 48, 1.).unwrap();
    assert_eq!(kept(&renderer).0, Arc::as_ptr(&other) as usize);
}
