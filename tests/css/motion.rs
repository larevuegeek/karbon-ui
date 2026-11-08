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

/// `@keyframes` de `height` (égaliseur) et de `width` en pourcentage (progression) :
/// un layout par image, la barre reste collée au bas de sa rangée.
#[test]
fn size_keyframes_animate_heights_and_percentage_widths() {
    use std::time::Duration;
    let mut ui = runtime(|| {
        ui! {@styles(css);
            <div>
                <div class="eq-row"><div class="eq"></div></div>
                <div class="track"><div class="progress"></div></div>
            </div>
        }
    });
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let bounds = |ui: &karbon_ui::ui::runtime::UiRuntime, i: usize| {
        ui.root().unwrap().children[i].children[0].bounds()
    };
    let bar = bounds(&ui, 0);
    assert_eq!((bar.right - bar.left, bar.bottom - bar.top), (3., 4.));
    assert_eq!(bar.bottom, bounds(&ui, 0).bottom);
    let layouts = ui.stats().layouts;
    prepare_headless(&mut ui, now + Duration::from_millis(200));
    let bar = bounds(&ui, 0);
    assert!((bar.bottom - bar.top - 11.).abs() < 0.01, "{bar:?}");
    let row = ui.root().unwrap().children[0].bounds();
    assert_eq!(bar.bottom, row.bottom);
    assert_eq!(ui.stats().layouts, layouts + 1);
    let progress = bounds(&ui, 1);
    assert!(
        (progress.right - progress.left - 160.).abs() < 0.01,
        "{progress:?}"
    );
    prepare_headless(&mut ui, now + Duration::from_millis(1200));
    let progress = bounds(&ui, 1);
    assert_eq!(progress.right - progress.left, 0.);
}

static PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `.pause .slide { animation-play-state: paused }` : la classe d'un ancêtre basculée par
/// la vue fige l'animation, puis la reprend là où elle s'était arrêtée.
#[test]
fn an_ancestor_state_rule_pauses_and_resumes_the_animation() {
    use std::{sync::atomic::Ordering, time::Duration};
    fn view() -> Element {
        let paused = PAUSED.load(Ordering::Relaxed);
        ui! {@styles(css); <div><div class:pause={paused}><div class="slide"></div></div></div>}
    }
    PAUSED.store(false, Ordering::Relaxed);
    let mut ui = runtime(view);
    let now = std::time::Instant::now();
    let x = |ui: &mut karbon_ui::ui::runtime::UiRuntime, ms: u64| {
        prepare_headless(ui, now + Duration::from_millis(ms));
        painted(ui).2
    };
    assert!(x(&mut ui, 0).abs() < 0.01);
    assert!((x(&mut ui, 250) - 25.).abs() < 0.01);
    PAUSED.store(true, Ordering::Relaxed);
    ui.invalidate();
    assert!((x(&mut ui, 250) - 25.).abs() < 0.01);
    assert!((x(&mut ui, 700) - 25.).abs() < 0.01);
    assert!(ui.next_wake().is_none());
    PAUSED.store(false, Ordering::Relaxed);
    ui.invalidate();
    assert!((x(&mut ui, 700) - 25.).abs() < 0.01);
    assert!((x(&mut ui, 800) - 35.).abs() < 0.01);
}

/// `style="animation-delay: …"` règle le minutage de l'animation posée par la classe
/// (satellites décalés de -1,4 s de RustMusic) ; `animation-duration` aussi.
#[test]
fn inline_animation_longhands_adjust_the_class_animation() {
    use std::time::Duration;
    let mut ui = runtime(|| {
        ui! {@styles(css); <div><div class="slide" style="animation-delay: -250ms; animation-duration: 2s"></div></div>}
    });
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    // 250 ms d'avance sur 2 s : 12,5 % du trajet de 100 px.
    assert!((painted(&ui).2 - 12.5).abs() < 0.01, "{}", painted(&ui).2);
    prepare_headless(&mut ui, now + Duration::from_millis(250));
    assert!((painted(&ui).2 - 25.).abs() < 0.01, "{}", painted(&ui).2);
}

/// Règle composée `.slide.back { animation-direction: reverse; animation-delay: -100ms }` :
/// les réglages remplacent ceux de l'animation de la classe.
#[test]
fn a_compound_rule_overrides_direction_and_delay() {
    use std::time::Duration;
    let mut ui = runtime(|| ui! {@styles(css); <div><div class="slide back"></div></div>});
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    // À rebours, déjà avancée de 100 ms : 90 px.
    assert!((painted(&ui).2 - 90.).abs() < 0.01, "{}", painted(&ui).2);
    prepare_headless(&mut ui, now + Duration::from_millis(150));
    assert!((painted(&ui).2 - 75.).abs() < 0.01, "{}", painted(&ui).2);
}

/// `.swap:hover { transition: none }` : la règle d'état remplace la transition de
/// l'élément (`motion-reduce:transition-none` de Tailwind), la largeur change d'emblée.
#[test]
fn a_state_rule_replaces_the_transition() {
    use karbon_ui::ui::runtime::UiEvent;
    use std::time::Duration;
    let mut ui = runtime(|| ui! {@styles(css);<div><div class="swap"></div></div>});
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let width = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        let b = ui.root().unwrap().children[0].bounds();
        b.right - b.left
    };
    assert_eq!(width(&ui), 100.);
    ui.event(UiEvent::PointerMove(PointF { x: 10., y: 10. }));
    prepare_headless(&mut ui, now + Duration::from_millis(10));
    prepare_headless(&mut ui, now + Duration::from_millis(20));
    assert_eq!(width(&ui), 200.);
}
