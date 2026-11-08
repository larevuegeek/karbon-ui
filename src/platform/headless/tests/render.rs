//! Rendu logiciel et protocole hors écran (aperçu, banc d'essai, PNG) du backend
//! headless.
use super::{Loader, Notes};
use crate::{
    diagnostics::BenchmarkOptions,
    platform::headless::{Headless, HeadlessApp, HeadlessOptions, LABEL},
    ui::{scene::PointF, theme::Theme},
};

#[test]
fn software_render_draws_the_scene_at_the_requested_scale() {
    let mut app = HeadlessApp::with_options(
        Notes::default(),
        HeadlessOptions {
            width: 320.,
            height: 240.,
            theme: Theme::LIGHT,
            ..Default::default()
        },
    )
    .unwrap();
    let image = app.render(2.).unwrap();
    assert_eq!((image.width, image.height), (640, 480));
    // Le fond est opaque et le bouton « Add » est peint d'une autre couleur que lui.
    let at = |p: PointF| {
        let offset = ((p.y * 2.) as usize * 640 + (p.x * 2.) as usize) * 4;
        image.pixels[offset..offset + 4].to_vec()
    };
    let background = at(PointF { x: 315., y: 235. });
    assert_eq!(background[3], 255);
    let add = app.bounds("add").unwrap();
    let inside = at(PointF {
        x: add.left + 3.,
        y: (add.top + add.bottom) / 2.,
    });
    assert_ne!(inside, background);
    assert!(app.render(0.).is_err());
}

#[test]
fn offscreen_protocol_runs_on_the_headless_backend() {
    let options = BenchmarkOptions {
        width: 320,
        height: 240,
        frames: 2,
        ..Default::default()
    };
    let image = crate::platform::offscreen::render_preview::<Headless>(
        options,
        Box::new(Loader::default()),
    )
    .unwrap();
    assert_eq!((image.width, image.height), (320, 240));
    let report = crate::platform::offscreen::render_benchmark::<Headless>(
        options,
        Box::new(Notes::default()),
    )
    .unwrap();
    assert_eq!(report.backend, LABEL);
    assert_eq!(report.phases.len(), 6);
    assert_eq!(report.phases[0].before.working_set_bytes, 0);
    let path = std::env::temp_dir().join(format!("karbon-headless-{}.png", std::process::id()));
    <Headless as crate::platform::Backend>::write_png(&image, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let info = crate::resources::inspect_image(&bytes).unwrap();
    assert_eq!((info.width, info.height), (320, 240));
}
