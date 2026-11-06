//! Filtres natifs : chaîne d'effets, alpha, flou, ombre, groupes imbriqués, DPI et cache.
use super::*;
fn filtered(id: u64, rect: RectF, filters: Filters, color: Color) -> Scene {
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::PushFilter {
        id: ElementId(id),
        bounds: rect,
        filters,
    });
    scene.fill(rect, 0., color);
    scene.commands.push(DrawCommand::PopFilter);
    scene
}
pub(crate) fn verify_filters(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    for scale in [1., 1.5, 2.] {
        let width = (120. * scale) as u32;
        let scene = filtered(
            1,
            RectF::new(30., 30., 30., 30.),
            Filters::new(&[Filter::Blur(4.), Filter::Opacity(0.5)]),
            Color::rgb(255, 0, 0),
        );
        let pixels = renderer.render(&scene, width, width, scale).unwrap();
        let pixel = |x: f32, y: f32| {
            let i = ((y * scale) as usize * width as usize + (x * scale) as usize) * 4;
            &pixels[i..i + 4]
        };
        assert!(
            (pixel(45., 45.)[3] as i32 - 128).abs() < 3,
            "center alpha {:?}",
            pixel(45., 45.)
        );
        assert!(pixel(27., 45.)[3] > 5);
        assert!(pixel(27., 45.)[3] < pixel(33., 45.)[3]);
        assert_eq!(pixel(5., 5.)[3], 0);
        let records = renderer.filters.records;
        renderer.render(&scene, width, width, scale).unwrap();
        assert_eq!(renderer.filters.records, records);
        let scene = filtered(
            1,
            RectF::new(30., 30., 30., 30.),
            Filters::new(&[Filter::Invert(1.)]),
            Color::rgba(255, 0, 0, 128),
        );
        let pixels = renderer.render(&scene, width, width, scale).unwrap();
        let i = ((45. * scale) as usize * width as usize + (45. * scale) as usize) * 4;
        assert!(
            (pixels[i] as i32 - 128).abs() < 3 && pixels[i + 2] < 3 && pixels[i + 3] == 128,
            "straight alpha inversion {:?}",
            &pixels[i..i + 4]
        );
        let scene = filtered(
            2,
            RectF::new(20., 20., 20., 20.),
            Filters::new(
                const {
                    &[Filter::DropShadow {
                        x: 20.,
                        y: 0.,
                        blur: 0.,
                        color: Color::rgb(0, 0, 255),
                    }]
                },
            ),
            Color::rgb(255, 0, 0),
        );
        let pixels = renderer.render(&scene, width, width, scale).unwrap();
        let i = ((30. * scale) as usize * width as usize + (50. * scale) as usize) * 4;
        assert!(
            pixels[i] > 240 && pixels[i + 2] < 5,
            "drop shadow position {:?}",
            &pixels[i..i + 4]
        );
    }
    let mut nested = Scene::new(Color::rgba(0, 0, 0, 0));
    nested.commands.push(DrawCommand::PushFilter {
        id: ElementId(10),
        bounds: RectF::new(20., 20., 50., 50.),
        filters: Filters::new(&[Filter::Opacity(0.5)]),
    });
    nested.commands.extend(
        filtered(
            11,
            RectF::new(30., 30., 20., 20.),
            Filters::new(&[Filter::Grayscale(1.)]),
            Color::rgb(255, 0, 0),
        )
        .commands,
    );
    nested.commands.push(DrawCommand::PopFilter);
    let pixels = renderer.render(&nested, 120, 120, 1.).unwrap();
    let i = (40 * 120 + 40) * 4;
    assert!(
        (pixels[i] as i32 - pixels[i + 2] as i32).abs() < 2
            && (pixels[i + 3] as i32 - 128).abs() <= 1,
        "nested pixel {:?}",
        &pixels[i..i + 4]
    );
    assert_eq!(renderer.filters.len(), 2);
    renderer
        .render(&Scene::new(Color::rgba(0, 0, 0, 0)), 120, 120, 1.)
        .unwrap();
    assert_eq!(renderer.filters.len(), 0);
}
#[test]
fn software_native_filter_chain_alpha_blur_shadow_nested_groups_dpi_and_cache() {
    verify_filters(RenderMode::Software);
}
