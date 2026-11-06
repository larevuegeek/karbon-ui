use super::raster::{blur, generate_with};
use super::*;
const SHADOW: BoxShadow = BoxShadow {
    x: 0.,
    y: 6.,
    blur: 12.,
    spread: 0.,
    color: Color::rgba(30, 100, 200, 128),
    inset: false,
};
fn image(frame: &Frame) -> &Arc<Image> {
    match &frame.paint {
        Paint::Image(image) => image,
        _ => panic!("vector shadow"),
    }
}
fn pixel(frame: &Frame, x: f32, y: f32) -> [u8; 4] {
    let x = (x - frame.x) as usize;
    let y = (y - frame.y) as usize;
    let i = (y * image(frame).width as usize + x) * 4;
    image(frame).pixels[i..i + 4].try_into().unwrap()
}
#[test]
fn soft_shadow_is_premultiplied_hollow_and_offset_and_inset_are_distinct() {
    let frame = generate(80., 50., [10.; 4], SHADOW, 1.).unwrap();
    assert_eq!(pixel(&frame, 40., 25.), [0; 4]);
    assert!(pixel(&frame, 40., 54.)[3] > pixel(&frame, 40., -5.)[3]);
    assert!(pixel(&frame, 40., 54.)[3] > pixel(&frame, 40., 62.)[3]);
    assert!(
        image(&frame)
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3].iter().all(|n| *n <= p[3]))
    );
    let inset = generate(
        80.,
        50.,
        [10.; 4],
        BoxShadow {
            inset: true,
            ..SHADOW
        },
        1.,
    )
    .unwrap();
    assert_eq!(pixel(&inset, 40., 54.)[3], 0);
    assert!(pixel(&inset, 40., 2.)[3] > pixel(&inset, 40., 25.)[3]);
}
#[test]
fn cached_shadow_reuses_images_when_translated_and_releases_removed_shadows() {
    let shadows = Shadows::new(&[SHADOW]);
    let mut cache = Cache::default();
    cache
        .prepare(RectF::new(0., 0., 80., 50.), [10.; 4], shadows, false)
        .unwrap();
    let first = Arc::downgrade(image(&cache.frames[0]));
    cache
        .prepare(RectF::new(100., 200., 80., 50.), [10.; 4], shadows, false)
        .unwrap();
    assert!(Arc::ptr_eq(
        &first.upgrade().unwrap(),
        image(&cache.frames[0])
    ));
    cache
        .prepare(
            RectF::new(0., 0., 80., 50.),
            [10.; 4],
            Shadows::new(&[]),
            false,
        )
        .unwrap();
    assert!(first.upgrade().is_none());
    assert!(generate(10000., 10000., [10.; 4], SHADOW, 1.).is_err());
}
#[test]
fn separable_blur_preserves_mass_and_symmetry_away_from_edges() {
    let mut mask = vec![0.; 51 * 51];
    mask[25 * 51 + 25] = 1.;
    blur(&mut mask, 51, 51, 3.);
    assert!((mask.iter().sum::<f32>() - 1.).abs() < 0.001);
    assert!((mask[25 * 51 + 20] - mask[25 * 51 + 30]).abs() < 0.00001);
    assert!(mask[25 * 51 + 25] > mask[25 * 51 + 20]);
}
/// Les ombres sans flou deviennent des traits et des remplissages : pas d'image de
/// la taille de l'élément pour une ligne d'un pixel.
#[test]
fn unblurred_rings_and_square_bands_are_vector_shapes() {
    let ring = |inset| BoxShadow {
        x: 0.,
        y: 0.,
        blur: 0.,
        spread: 2.,
        color: Color::rgba(255, 255, 255, 40),
        inset,
    };
    let outer = vector(80., 50., [10.; 4], ring(false)).unwrap();
    assert!(matches!(
        &outer[..],
        [Frame { paint: Paint::Ring { radii, width, .. }, x, y, width: w, height: h, inset: false, .. }]
            if *radii == [12.; 4] && *width == 2. && (*x, *y, *w, *h) == (-2., -2., 84., 54.)
    ));
    let inset = vector(80., 50., [10.; 4], ring(true)).unwrap();
    assert!(matches!(
        &inset[..],
        [Frame { paint: Paint::Ring { radii, .. }, x, y, width: w, height: h, inset: true, .. }]
            if *radii == [10.; 4] && (*x, *y, *w, *h) == (0., 0., 80., 50.)
    ));
    // `inset 0 -1px 0` sur une boîte à angles droits : la bande du bas.
    let bottom = vector(
        80.,
        50.,
        [0.; 4],
        BoxShadow {
            y: -1.,
            spread: 0.,
            inset: true,
            ..ring(true)
        },
    )
    .unwrap();
    assert!(matches!(
        &bottom[..],
        [Frame { paint: Paint::Fill(_), x, y, width: w, height: h, .. }]
            if (*x, *y, *w, *h) == (0., 49., 80., 1.)
    ));
    // Décalage sur une boîte arrondie, ou flou : image.
    assert!(
        vector(
            80.,
            50.,
            [4.; 4],
            BoxShadow {
                y: 1.,
                ..ring(false)
            }
        )
        .is_none()
    );
    assert!(vector(80., 50., [0.; 4], SHADOW).is_none());
    // Le cache n'alloue aucune image pour ces ombres.
    let mut cache = Cache::default();
    cache
        .prepare(
            RectF::new(0., 0., 80., 50.),
            [10.; 4],
            Shadows::new(
                const {
                    &[BoxShadow {
                        x: 0.,
                        y: 0.,
                        blur: 0.,
                        spread: 1.,
                        color: Color::rgba(0, 0, 0, 90),
                        inset: false,
                    }]
                },
            ),
            false,
        )
        .unwrap();
    let mut scene = crate::ui::scene::Scene::new(Color::rgba(0, 0, 0, 0));
    cache.paint(RectF::new(10., 10., 80., 50.), false, &mut scene);
    assert!(matches!(
        &scene.commands[..],
        [crate::ui::scene::DrawCommand::Shape { stroke: Some((width, _)), fill: None, .. }] if *width == 1.
    ));
    assert!(scene.validate().is_ok());
}

#[test]
fn wide_blurs_on_a_reduced_grid_match_the_full_resolution_shadow() {
    for (w, h, blur, y) in [
        (1260., 124., 50., 20.),
        (210., 210., 32., 12.),
        (80., 40., 8., 2.),
    ] {
        let shadow = BoxShadow {
            x: 0.,
            y,
            blur,
            spread: 0.,
            color: Color::rgba(0, 0, 0, 200),
            inset: false,
        };
        let pixels = |down| match generate_with(w, h, [16.; 4], shadow, 1., down)
            .unwrap()
            .paint
        {
            Paint::Image(image) => image.pixels.clone(),
            _ => unreachable!(),
        };
        let (exact, fast) = (pixels(1.), pixels(8.));
        assert_eq!(exact.len(), fast.len());
        let worst = exact
            .iter()
            .zip(&fast)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(worst <= 6, "{w}x{h} blur {blur}: écart {worst}/255");
    }
}

#[test]
fn live_resize_stretches_blurred_layers_and_redraws_vector_ones() {
    const LAYERS: &[BoxShadow] = &[
        BoxShadow {
            x: 0.,
            y: 8.,
            blur: 24.,
            spread: 0.,
            color: Color::rgba(0, 0, 0, 120),
            inset: false,
        },
        BoxShadow {
            x: 0.,
            y: 0.,
            blur: 0.,
            spread: 1.,
            color: Color::rgba(255, 255, 255, 30),
            inset: false,
        },
    ];
    let shadows = Shadows::new(LAYERS);
    let mut cache = Cache::default();
    cache
        .prepare(RectF::new(0., 0., 200., 100.), [12.; 4], shadows, false)
        .unwrap();
    let image = |c: &Cache| {
        c.frames.iter().find_map(|f| match &f.paint {
            Paint::Image(i) => Some(Arc::clone(i)),
            _ => None,
        })
    };
    let before = image(&cache).unwrap();
    // Pendant le redimensionnement : même image, étirée ; anneau refait à la taille.
    cache
        .prepare(RectF::new(0., 0., 260., 130.), [12.; 4], shadows, true)
        .unwrap();
    assert!(Arc::ptr_eq(&before, &image(&cache).unwrap()));
    let mut scene = crate::ui::scene::Scene::new(Color::rgba(0, 0, 0, 0));
    cache.paint(RectF::new(0., 0., 260., 130.), false, &mut scene);
    let (mut stretched, mut ring) = (None, None);
    for command in &scene.commands {
        match command {
            crate::ui::scene::DrawCommand::Image { rect, .. } => stretched = Some(*rect),
            crate::ui::scene::DrawCommand::Shape { rect, .. } => ring = Some(*rect),
            _ => {}
        }
    }
    let (stretched, ring) = (stretched.unwrap(), ring.unwrap());
    assert_eq!(layout::width(stretched), before.width as f32 + 60.);
    assert_eq!((layout::width(ring), layout::height(ring)), (262., 132.));
    // Fin du redimensionnement : image exacte recalculée.
    cache
        .prepare(RectF::new(0., 0., 260., 130.), [12.; 4], shadows, false)
        .unwrap();
    assert!(!Arc::ptr_eq(&before, &image(&cache).unwrap()));
}
