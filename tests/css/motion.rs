//! Animations et transitions : `@keyframes` à plusieurs segments (répétition, alternance,
//! arrêt), transitions de couleur et de largeur au survol ou quand la vue bascule une
//! classe, mouvements réduits.
use crate::support::*;
use karbon_ui::ui::{
    prelude::*,
    scene::{DrawCommand, PointF},
};
css_file!(mod css, "motion.css");

#[test]
fn compiled_keyframes_interpolate_each_segment_repeat_alternate_and_stop() {
    use karbon_ui::ui::runtime::UiRuntime;
    use std::time::{Duration, Instant};
    let mut ui=UiRuntime::new(View(||ui!{@styles(css);<div><div class="pulse" style="width:100px; height:50px; background:#ffffff"></div></div>}),&Services).unwrap();
    let now = Instant::now();
    prepare(&mut ui, now);
    let stats = ui.stats();
    let opacity = |ui: &UiRuntime| {
        ui.scene()
            .commands
            .iter()
            .find_map(|c| {
                if let DrawCommand::PushEffect { opacity, .. } = c {
                    Some(*opacity)
                } else {
                    None
                }
            })
            .unwrap_or(1.)
    };
    near(opacity(&ui), 0.);
    prepare(&mut ui, now + Duration::from_millis(225));
    near(opacity(&ui), 0.5);
    prepare(&mut ui, now + Duration::from_millis(350));
    near(opacity(&ui), 1.);
    prepare(&mut ui, now + Duration::from_millis(725));
    near(opacity(&ui), 0.75);
    prepare(&mut ui, now + Duration::from_millis(1850));
    near(opacity(&ui), 1.);
    prepare(&mut ui, now + Duration::from_millis(2200));
    near(opacity(&ui), 0.);
    assert_eq!(ui.stats().views, stats.views);
    assert_eq!(ui.stats().layouts, stats.layouts);
    assert!(ui.next_wake().is_none());
}

#[test]
fn background_color_transitions_interpolate_over_frames() {
    use karbon_ui::ui::runtime::UiEvent;
    use std::time::Duration;
    let mut ui = runtime(|| ui! {@styles(css);<div class="fade"></div>});
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let fill = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        ui.scene()
            .commands
            .iter()
            .find_map(|c| match c {
                DrawCommand::Fill { color, .. } => Some(*color),
                DrawCommand::Shape {
                    fill: Some(karbon_ui::ui::rendering::brush::Background::Solid(color)),
                    ..
                } => Some(*color),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(fill(&ui), Color::hex(0x000000));
    ui.event(UiEvent::PointerMove(PointF { x: 10., y: 10. }));
    prepare_headless(&mut ui, now + Duration::from_millis(10));
    assert!(ui.next_wake().is_some());
    prepare_headless(&mut ui, now + Duration::from_millis(60));
    let mid = fill(&ui);
    assert!((100..=155).contains(&mid.red), "{mid:?}");
    prepare_headless(&mut ui, now + Duration::from_millis(200));
    assert_eq!(fill(&ui), Color::hex(0xffffff));
}

#[test]
fn width_transitions_relayout_each_frame_on_hover() {
    use karbon_ui::ui::runtime::UiEvent;
    use std::time::Duration;
    let mut ui = runtime(|| ui! {@styles(css);<div><div class="grow"></div></div>});
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let width = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        let b = ui.root().unwrap().children[0].bounds();
        b.right - b.left
    };
    assert_eq!(width(&ui), 100.);
    ui.event(UiEvent::PointerMove(PointF { x: 10., y: 10. }));
    prepare_headless(&mut ui, now + Duration::from_millis(10));
    assert!(ui.next_wake().is_some());
    prepare_headless(&mut ui, now + Duration::from_millis(60));
    let mid = width(&ui);
    assert!((140. ..=160.).contains(&mid), "{mid}");
    prepare_headless(&mut ui, now + Duration::from_millis(200));
    assert_eq!(width(&ui), 200.);
    // Le pointeur part : retour animé vers la largeur de base.
    ui.event(UiEvent::PointerLeave);
    prepare_headless(&mut ui, now + Duration::from_millis(210));
    prepare_headless(&mut ui, now + Duration::from_millis(260));
    let back = width(&ui);
    assert!((140. ..=160.).contains(&back), "{back}");
    prepare_headless(&mut ui, now + Duration::from_millis(400));
    assert_eq!(width(&ui), 100.);
}

static NARROW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Une classe basculée par la vue (reconstruction) anime la largeur et ses voisins suivent.
#[test]
fn width_transitions_follow_a_toggled_class_and_reduced_motion_jumps() {
    use std::{sync::atomic::Ordering, time::Duration};
    fn view() -> Element {
        let narrow = NARROW.load(Ordering::Relaxed);
        ui! {@styles(css);
            <div><div class="rail">
                <div class="panel" class:narrow={narrow}></div>
                <div class="rest"></div>
            </div></div>
        }
    }
    NARROW.store(false, Ordering::Relaxed);
    let mut ui = runtime(view);
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let widths = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        let root = &ui.root().unwrap().children[0];
        (
            {
                let b = root.children[0].bounds();
                b.right - b.left
            },
            {
                let b = root.children[1].bounds();
                b.right - b.left
            },
        )
    };
    assert_eq!(widths(&ui), (288., 112.));
    NARROW.store(true, Ordering::Relaxed);
    ui.invalidate();
    prepare_headless(&mut ui, now + Duration::from_millis(10));
    assert!(ui.next_wake().is_some());
    prepare_headless(&mut ui, now + Duration::from_millis(110));
    let (panel, rest) = widths(&ui);
    assert!((170. ..=194.).contains(&panel), "{panel}");
    assert!((rest - (400. - panel)).abs() < 0.01, "{rest}");
    prepare_headless(&mut ui, now + Duration::from_millis(300));
    assert_eq!(widths(&ui), (76., 324.));
    assert!(ui.next_wake().is_none());
    // Mouvements réduits : la nouvelle largeur s'applique d'emblée.
    ui.set_reduced_motion(true);
    NARROW.store(false, Ordering::Relaxed);
    ui.invalidate();
    prepare_headless(&mut ui, now + Duration::from_millis(310));
    assert_eq!(widths(&ui), (288., 112.));
}

/// Couleur du premier fond peint et effet (opacité, translation horizontale) du premier
/// groupe d'effet.
fn painted(ui: &karbon_ui::ui::runtime::UiRuntime) -> (Option<Color>, f32, f32) {
    let commands = &ui.scene().commands;
    let fill = commands.iter().find_map(|c| match c {
        DrawCommand::Fill { color, .. } => Some(*color),
        DrawCommand::Shape {
            fill: Some(karbon_ui::ui::rendering::brush::Background::Solid(color)),
            ..
        } => Some(*color),
        _ => None,
    });
    let (opacity, x) = commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::PushEffect {
                opacity, transform, ..
            } => Some((*opacity, transform.dx)),
            _ => None,
        })
        .unwrap_or((1., 0.));
    (fill, opacity, x)
}

#[test]
fn partial_keyframes_animate_each_property_on_its_own_track() {
    use std::time::Duration;
    let mut ui = runtime(|| ui! {@styles(css);<div><div class="rise"></div></div>});
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let stats = ui.stats();
    let (_, opacity, x) = painted(&ui);
    near(opacity, 0.);
    near(x, 0.);
    prepare_headless(&mut ui, now + Duration::from_millis(250));
    let (_, opacity, x) = painted(&ui);
    near(opacity, 0.25);
    near(x, 20.);
    prepare_headless(&mut ui, now + Duration::from_millis(750));
    let (_, opacity, x) = painted(&ui);
    near(opacity, 0.75);
    near(x, 20.);
    // Opacité et transformation seules : ni reconstruction ni layout par image.
    assert_eq!(ui.stats().views, stats.views);
    assert_eq!(ui.stats().layouts, stats.layouts);
}

#[test]
fn keyframed_colours_and_animation_lists_follow_their_own_timing() {
    use std::time::Duration;
    let mut ui = runtime(|| ui! {@styles(css);<div><div class="glow"></div></div>});
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let channel = |ui: &karbon_ui::ui::runtime::UiRuntime| painted(ui).0.unwrap().red;
    // `glow` (délai -250 ms) : à 25 % du cycle, mi-chemin du noir de l'élément (étape 0 %
    // absente) vers le blanc de 50 %.
    let red = channel(&ui);
    assert!((120..=136).contains(&red), "{red}");
    // `rise` à rebours sur 2 s : commence à l'étape 100 % (opaque, sans translation).
    let (_, opacity, x) = painted(&ui);
    near(opacity, 1.);
    near(x, 0.);
    assert!(ui.next_wake().is_some());
    prepare_headless(&mut ui, now + Duration::from_millis(250));
    assert_eq!(channel(&ui), 255);
    prepare_headless(&mut ui, now + Duration::from_millis(500));
    let red = channel(&ui);
    assert!((120..=136).contains(&red), "{red}");
    let (_, opacity, x) = painted(&ui);
    near(opacity, 0.75);
    near(x, 20.);
    prepare_headless(&mut ui, now + Duration::from_millis(1000));
    let (_, opacity, x) = painted(&ui);
    near(opacity, 0.5);
    near(x, 40.);
    // Infinie : le fond continue après la fin de `rise` (sans remplissage).
    prepare_headless(&mut ui, now + Duration::from_millis(2250));
    assert_eq!(channel(&ui), 255);
    let (_, opacity, _) = painted(&ui);
    near(opacity, 1.);
    assert!(ui.next_wake().is_some());
    // Mouvements réduits : état final sans remplissage, la couleur de l'élément revient.
    ui.set_reduced_motion(true);
    prepare_headless(&mut ui, now + Duration::from_millis(2300));
    assert_eq!(painted(&ui).0, Some(Color::hex(0x000000)));
    assert!(ui.next_wake().is_none());
}
