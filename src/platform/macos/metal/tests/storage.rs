//! Stockage `Managed` (GPU dédié, Mac Intel) et bornes des caches GPU.
//!
//! Sur un Mac Apple Silicon, le stockage `Managed` est accepté et se comporte comme
//! `Shared` : ces tests exercent les appels du chemin d'un GPU dédié (`didModifyRange`,
//! textures gérées, relecture synchronisée par blit) et vérifient qu'il rend au pixel
//! près comme le chemin par défaut. Les copies réelles entre mémoire du CPU et du GPU ne
//! se vérifient que sur une machine à GPU dédié.
use super::{difference, metal};
use crate::platform::macos::graphics::Renderer as CgRenderer;
use crate::platform::macos::metal::{
    Renderer, Trim,
    device::Storage,
    resources::{IDLE_FRAMES, POOL_FRAMES},
};
use crate::ui::{
    element::ElementId,
    rendering::{
        filter::{Filter, Filters},
        transform::Affine,
    },
    scene::{DrawCommand, Image, PointF, RectF, Scene},
    theme::Color,
};
use std::sync::Arc;

fn image() -> Arc<Image> {
    let mut pixels = Vec::new();
    for y in 0..48u32 {
        for x in 0..48u32 {
            pixels.extend([(x * 5) as u8, (y * 5) as u8, 90, 255]);
        }
    }
    Arc::new(Image::new(48, 48, pixels).unwrap())
}
/// Scène qui emploie chaque ressource : instances, arrêts, découpes, image (mipmaps),
/// rastérisation, atlas, couche d'opacité, groupe filtré et `backdrop-filter`.
fn scene(image: &Arc<Image>) -> Scene {
    static BLUR: [Filter; 1] = [Filter::Blur(3.)];
    let mut scene = Scene::new(Color::rgb(240, 240, 240));
    let c = &mut scene.commands;
    c.push(DrawCommand::Fill {
        rect: RectF::new(4., 4., 80., 40.),
        radius: 6.,
        color: Color::rgb(30, 90, 200),
    });
    c.push(DrawCommand::Image {
        rect: RectF::new(90., 4., 24., 24.),
        image: Arc::clone(image),
    });
    c.push(DrawCommand::Path {
        points: vec![
            PointF { x: 120., y: 40. },
            PointF { x: 140., y: 6. },
            PointF { x: 160., y: 40. },
        ],
        closed: true,
        color: Color::rgb(200, 40, 0),
    });
    c.push(DrawCommand::Text {
        rect: RectF::new(6., 50., 150., 20.),
        text: "Lecture en cours".into(),
        size: 14.,
        bold: false,
        color: Color::rgb(20, 20, 20),
        wrap: false,
    });
    let bounds = RectF::new(100., 50., 60., 30.);
    c.push(DrawCommand::PushEffect {
        transform: Affine::IDENTITY,
        opacity: 0.6,
        bounds,
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(100., 50., 40., 30.),
        radius: 0.,
        color: Color::rgb(0, 150, 0),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(120., 60., 40., 20.),
        radius: 4.,
        color: Color::rgb(150, 0, 150),
    });
    c.push(DrawCommand::PopEffect);
    c.push(DrawCommand::PushFilter {
        id: ElementId(7),
        bounds: RectF::new(10., 80., 50., 30.),
        filters: Filters::new(&BLUR),
    });
    c.push(DrawCommand::Fill {
        rect: RectF::new(15., 85., 40., 20.),
        radius: 0.,
        color: Color::rgb(220, 0, 60),
    });
    c.push(DrawCommand::PopFilter);
    c.push(DrawCommand::Backdrop {
        rect: RectF::new(70., 20., 60., 70.),
        radii: [8.; 4],
        filters: Filters::new(&BLUR),
    });
    scene
}
#[test]
fn managed_storage_renders_like_the_default_storage() {
    let Some(mut shared) = metal() else {
        return;
    };
    let mut managed = Renderer::with_storage(Some(Storage::MANAGED)).expect("renderer Managed");
    assert_eq!(managed.gpu.storage, Storage::MANAGED);
    let texture = managed
        .gpu
        .upload_texture(objc2_metal::MTLPixelFormat::BGRA8Unorm, 4, 4, false)
        .unwrap();
    assert_eq!(
        objc2_metal::MTLResource::storageMode(&*texture),
        objc2_metal::MTLStorageMode::Managed
    );
    let mut cg = CgRenderer::new().unwrap();
    let image = image();
    let scene = scene(&image);
    for scale in [1f32, 2.] {
        let (w, h) = ((170. * scale) as u32, (115. * scale) as u32);
        // Deux images de suite : la seconde réécrit les tampons et reprend les textures.
        for _ in 0..2 {
            let a = shared.render(&mut cg, &scene, w, h, scale).unwrap();
            let b = managed.render(&mut cg, &scene, w, h, scale).unwrap();
            assert!(a == b, "Managed et Shared diffèrent à {scale}x");
            let reference = cg.render(&scene, w, h, scale).unwrap();
            let d = difference(&b, &reference);
            assert!(
                d.mean <= 1.0,
                "Managed contre Core Graphics à {scale}x : {d:?}"
            );
        }
    }
}
#[test]
fn unused_images_and_rasterizations_are_released_after_idle_frames() {
    let Some(mut metal) = metal() else {
        return;
    };
    let mut cg = CgRenderer::new().unwrap();
    let image = image();
    let full = scene(&image);
    let empty = Scene::new(Color::rgb(255, 255, 255));
    metal.render(&mut cg, &full, 170, 115, 1.).unwrap();
    let [images, rasters, _] = metal.resources.counts();
    assert_eq!((images, rasters), (1, 1));
    // Encore gardées pendant `IDLE_FRAMES - 1` images sans elles, libérées ensuite.
    for _ in 1..IDLE_FRAMES {
        metal.render(&mut cg, &empty, 16, 16, 1.).unwrap();
    }
    assert_eq!(metal.resources.counts()[..2], [1, 1]);
    metal.render(&mut cg, &empty, 16, 16, 1.).unwrap();
    assert_eq!(metal.resources.counts()[..2], [0, 0]);
    // Les cibles en réserve inutilisées depuis `POOL_FRAMES` images aussi.
    metal.render(&mut cg, &full, 170, 115, 1.).unwrap();
    metal.render(&mut cg, &empty, 16, 16, 1.).unwrap();
    assert!(metal.resources.counts()[2] > 0);
    for _ in 0..POOL_FRAMES {
        metal.render(&mut cg, &empty, 16, 16, 1.).unwrap();
    }
    assert_eq!(metal.resources.counts()[2], 0);
}
#[test]
fn trimming_releases_idle_resources_and_everything_when_hidden() {
    let Some(mut metal) = metal() else {
        return;
    };
    let mut cg = CgRenderer::new().unwrap();
    let image = image();
    let other = Arc::new(Image::new(2, 2, vec![255; 16]).unwrap());
    let full = scene(&image);
    let mut alone = Scene::new(Color::rgb(255, 255, 255));
    alone.commands.push(DrawCommand::Image {
        rect: RectF::new(0., 0., 8., 8.),
        image: Arc::clone(&other),
    });
    let reference = metal.render(&mut cg, &full, 170, 115, 2.).unwrap();
    metal.render(&mut cg, &alone, 16, 16, 1.).unwrap();
    assert_eq!(
        metal.resources.counts(),
        [2, 1, metal.resources.counts()[2]]
    );
    // Repos : seule l'image de la dernière image reste, réserve et tampons vidés.
    metal.trim(Trim::Idle);
    assert_eq!(metal.resources.counts(), [1, 0, 0]);
    assert_eq!(metal.ring.bytes(), 0);
    // Masquée : tout est libéré ; l'image suivante recrée tout, à l'identique.
    metal.trim(Trim::Hidden);
    assert_eq!(metal.resources.counts(), [0, 0, 0]);
    assert_eq!(metal.resources.usage(), [0; 5]);
    let again = metal.render(&mut cg, &full, 170, 115, 2.).unwrap();
    assert!(again == reference);
}
/// Image traduite sans être rendue (pas de drawable, M3 de la revue) : l'image chargée
/// pendant la traduction attend ses mipmaps de l'encodage ; gardée, elle servirait ensuite
/// sans eux. Elle est oubliée et la traduction suivante redemande ses mipmaps.
#[test]
fn an_image_translated_but_not_rendered_asks_again_for_its_mipmaps() {
    let Some(mut metal) = metal() else {
        return;
    };
    let mut cg = CgRenderer::new().unwrap();
    let image = image();
    let full = scene(&image);
    let built = metal.build(&mut cg, &full, (170, 115), 1.).unwrap();
    assert_eq!(
        built.mipmaps.len(),
        1,
        "image nouvelle : mipmaps à calculer"
    );
    metal.resources.discard_unrendered(&built.mipmaps);
    metal.resources.end();
    assert_eq!(
        metal.resources.counts()[0],
        0,
        "texture sans mipmaps oubliée"
    );
    let again = metal.build(&mut cg, &full, (170, 115), 1.).unwrap();
    assert_eq!(again.mipmaps.len(), 1, "mipmaps redemandés");
    metal.resources.end();
}
