//! Sélecteurs et états : classes composées, négation, `:nth-child`, combinateurs,
//! `:hover` / `:active` / `:focus-visible` / `:focus-within`, `:placeholder-shown`,
//! `:indeterminate`, raccourcis conditionnels et pseudo-éléments `::before` / `::after`.
use crate::support::*;
use karbon_ui::ui::{
    layout,
    prelude::*,
    runtime::{UiEvent, UiRuntime},
    scene::{DrawCommand, PointF},
    styling::{self, StyleEnvironment},
};
use std::time::{Duration, Instant};
css_file!(mod css, "selectors.css");

/// Bouton dont la classe `.feedback` change d'opacité et de transformation au survol.
struct Feedback;
impl Application for Feedback {
    type Message = Msg;
    fn update(&mut self, _: Msg) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div style="padding:10px; gap:10px">
                <button id="feedback" class="feedback" style="width:200px; height:50px" onclick={Msg::Click(1)}>Tester</button>
            </div>
        }
    }
}

#[test]
fn css_states_replace_base_properties_and_active_keeps_unmodified_hover_properties() {
    let mut ui = UiRuntime::new(Feedback, &Services).unwrap();
    let start = Instant::now();
    prepare(&mut ui, start);
    near(opacity(&ui), 0.8);
    let views = ui.stats().views;
    let layouts = ui.stats().layouts;
    let point = PointF { x: 100., y: 35. };
    ui.event(UiEvent::PointerMove(point));
    prepare(&mut ui, start + Duration::from_millis(1));
    prepare(&mut ui, start + Duration::from_millis(120));
    near(opacity(&ui), 0.5);
    let matrix = ui
        .scene()
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::PushEffect { transform, .. } => Some(*transform),
            _ => None,
        })
        .unwrap();
    near(matrix.m11, 1.1);
    // The base translateX(4px) is replaced, not multiplied/added to the CSS hover transform.
    near(matrix.dx, -11.);
    ui.event(UiEvent::PointerDown(point));
    prepare(&mut ui, start + Duration::from_millis(121));
    prepare(&mut ui, start + Duration::from_millis(240));
    near(opacity(&ui), 0.3);
    let matrix = ui
        .scene()
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::PushEffect { transform, .. } => Some(*transform),
            _ => None,
        })
        .unwrap();
    near(matrix.m11, 1.1);
    assert_eq!(ui.stats().views, views);
    assert_eq!(ui.stats().layouts, layouts);
}

#[test]
fn state_colors_restore_without_changing_geometry() {
    let mut root = ui! {@styles(css);<button class="control" style="width:200px; height:50px" onclick={Msg::Click(1)}>Action</button>};
    arrange(&mut root, 200., 50.);
    let change = styling::resolve(
        &mut root,
        StyleEnvironment {
            pointer: Some(PointF { x: 20., y: 20. }),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(change.paint);
    assert!(!change.layout);
    assert_eq!(root.style.background, Some(Color::hex(0x304050)));
    let scene = paint(&root);
    assert!(
        scene
            .commands
            .iter()
            .any(|c| matches!(c,DrawCommand::FontText{color,..}if *color==Color::hex(0x80ff80)))
    );
    let id = root.id;
    styling::resolve(
        &mut root,
        StyleEnvironment {
            pressed: Some(id),
            focused: Some(id),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(root.style.background, Some(Color::hex(0x405060)));
    styling::resolve(&mut root, StyleEnvironment::default()).unwrap();
    assert_eq!(root.style.background, Some(Color::hex(0x102030)));
    near(layout::width(layout::content_rect(&root)), 182.);
}

#[test]
fn compound_selectors_negation_nth_child_and_keyboard_focus_match_the_live_tree() {
    let mut root = ui! {@styles(css);<div><button class="compound primary" style="width:150px; height:50px" onclick={Msg::Click(1)}>Un</button><button class="compound" style="width:150px; height:50px" onclick={Msg::Click(2)}>Deux</button></div>};
    arrange(&mut root, 300., 150.);
    let color = |node: &Element| {
        paint(node)
            .commands
            .iter()
            .find_map(|c| match c {
                DrawCommand::FontText { color, .. } => Some(*color),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(color(&root.children[0]), Color::hex(0x204060));
    assert_eq!(color(&root.children[1]), Color::hex(0x101010));
    near(layout::width(layout::content_rect(&root.children[0])), 146.);
    near(layout::width(layout::content_rect(&root.children[1])), 150.);
    let id = root.children[0].id;
    styling::resolve(
        &mut root,
        StyleEnvironment {
            pointer: Some(PointF { x: 30., y: 20. }),
            focused: Some(id),
            focus_visible: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(color(&root.children[0]), Color::hex(0x20ff40));
    assert_eq!(
        root.children[0].style.background,
        Some(Color::hex(0xff00ff))
    );
    styling::resolve(
        &mut root,
        StyleEnvironment {
            focused: Some(id),
            focus_visible: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(root.children[0].style.background, None);
    assert_eq!(color(&root.children[0]), Color::hex(0x204060));
}

#[test]
fn placeholder_state_tracks_editor_text_even_when_application_does_not_rebuild() {
    use karbon_ui::ui::runtime::{Key, UiEvent, UiRuntime};
    let mut ui=UiRuntime::new(View(||ui!{@styles(css);<div><input type="text" class="placeholder" style="width:200px; height:50px" value="" placeholder="Saisir" oninput={Msg::Text}></div>}),&Services).unwrap();
    let now = std::time::Instant::now();
    prepare(&mut ui, now);
    let views = ui.stats().views;
    assert_eq!(
        ui.root().unwrap().children[0].style.background,
        Some(Color::hex(0xffff00))
    );
    ui.event(UiEvent::KeyDown {
        key: Key::Tab,
        shift: false,
        repeat: false,
    });
    prepare(&mut ui, now);
    ui.event(UiEvent::Text("Texte".into()));
    prepare(&mut ui, now);
    assert_eq!(ui.root().unwrap().children[0].style.background, None);
    assert_eq!(ui.stats().views, views);
}

#[test]
fn conditional_shorthands_reset_earlier_edges_and_gap_axes() {
    let mut root =
        ui! {@styles(css);<div class="state-box" style="width:200px; height:100px"></div>};
    arrange(&mut root, 200., 100.);
    styling::resolve(
        &mut root,
        StyleEnvironment {
            pointer: Some(PointF { x: 50., y: 50. }),
            ..Default::default()
        },
    )
    .unwrap();
    near(root.style.padding.left, 8.);
    near(root.style.padding.right, 12.);
    near(root.style.vertical_gap(), 3.);
    near(root.style.horizontal_gap(), 3.);
    near(root.style.margin.left, 2.);
    near(root.style.margin.right, 5.);
    styling::resolve(&mut root, StyleEnvironment::default()).unwrap();
    near(root.style.padding.left, 4.);
    near(root.style.vertical_gap(), 10.);
    near(root.style.margin.right, 0.);
}

#[test]
fn mixed_checkbox_matches_indeterminate_instead_of_checked() {
    let mut root = ui! {@styles(css);<label><input type="checkbox" class="tristate" indeterminate onchange={Msg::Checked}> Mixte</label>};
    arrange(&mut root, 200., 80.);
    let scene = paint(&root);
    assert!(
        scene
            .commands
            .iter()
            .any(|c| matches!(c,DrawCommand::FontText{color,..}if *color==Color::hex(0x00ff00)))
    );
}

#[test]
fn descendant_child_and_sibling_combinators_match_the_live_tree() {
    let mut root = ui! {@styles(css);<div>
        <div class="card"><div><div class="title">Deep</div><div class="direct">No</div></div><div class="direct">Yes</div></div>
        <div class="title">Outside</div>
        <div class="title strong">Plain</div>
        <div><div class="item"></div><div class="item"></div><div class="marker"></div><div></div><div class="after"></div></div>
        <div class="list"><div class="row"></div><div class="row"></div><div class="row"></div></div>
        <div class="card"><div class="title strong">Strong</div></div>
    </div>};
    arrange_headless(&mut root, 300., 400.);
    let c = &root.children;
    assert_eq!(
        text_color(&c[0].children[0].children[0]),
        Color::hex(0xff0000)
    );
    // `.card > .direct` : seul l'enfant direct.
    assert_ne!(
        text_color(&c[0].children[0].children[1]),
        Color::hex(0x00ff00)
    );
    assert_eq!(text_color(&c[0].children[1]), Color::hex(0x00ff00));
    assert_ne!(text_color(&c[1]), Color::hex(0xff0000));
    assert_ne!(text_color(&c[2]), Color::hex(0x00ffff));
    let siblings = &c[3].children;
    assert_eq!(siblings[0].style.background, None);
    assert_eq!(siblings[1].style.background, Some(Color::hex(0x0000ff)));
    assert_eq!(siblings[4].style.background, Some(Color::hex(0xff00ff)));
    let rows = &c[4].children;
    assert_eq!(rows[0].style.background, Some(Color::hex(0x111111)));
    assert_eq!(rows[1].style.background, Some(Color::hex(0x222222)));
    assert_eq!(rows[2].style.background, None);
    // Spécificité : `.card .title.strong` après `.card .title`.
    assert_eq!(text_color(&c[5].children[0]), Color::hex(0x00ffff));
}

#[test]
fn group_hover_and_focus_within_restyle_descendants() {
    use karbon_ui::ui::runtime::{Key, UiEvent};
    let mut ui = runtime(|| {
        ui! {@styles(css);<div class="group"><div class="reveal"></div>
        <div class="ring"><input type="text" value="x" oninput={Msg::Text}></div></div>}
    });
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let opacity = |ui: &karbon_ui::ui::runtime::UiRuntime| {
        ui.scene().commands.iter().find_map(|c| match c {
            DrawCommand::PushEffect {
                opacity, transform, ..
            } => Some((*opacity, transform.m11)),
            _ => None,
        })
    };
    let hidden = opacity(&ui);
    assert!(hidden.is_none_or(|(o, _)| o == 0.), "{hidden:?}");
    ui.event(UiEvent::PointerMove(PointF { x: 50., y: 5. }));
    prepare_headless(&mut ui, now + std::time::Duration::from_millis(16));
    let (shown, scale) = opacity(&ui).expect("effect");
    assert_eq!((shown, scale), (1., 1.5));
    // Focus dans le champ : bordure de l'ancêtre .ring.
    let before = ui.scene().commands.iter().any(|c| {
        matches!(c,
        DrawCommand::Shape { stroke: Some((_, color)), .. } | DrawCommand::Stroke { color, .. }
            if *color == Color::hex(0xffff00))
    });
    assert!(!before);
    ui.event(UiEvent::KeyDown {
        key: Key::Tab,
        shift: false,
        repeat: false,
    });
    prepare_headless(&mut ui, now + std::time::Duration::from_millis(32));
    assert!(ui.scene().commands.iter().any(|c| matches!(c,
        DrawCommand::Shape { stroke: Some((_, color)), .. } | DrawCommand::Stroke { color, .. }
            if *color == Color::hex(0xffff00))));
}

#[test]
fn before_and_after_generate_styled_boxes_that_follow_host_states() {
    let mut root = ui! {@styles(css);<div>
        <div class="badge"></div>
        <div class="label"><p>Texte</p></div>
    </div>};
    arrange_headless(&mut root, 300., 200.);
    let badge = &root.children[0];
    assert_eq!(badge.children.len(), 1);
    let dot = &badge.children[0];
    assert_eq!(dot.style.background, Some(Color::hex(0xff0000)));
    assert_eq!(dot.bounds().left - badge.bounds().left, 28.);
    assert_eq!(dot.bounds().top - badge.bounds().top, 22.);
    let label = &root.children[1];
    // ::before en tête, `content: none` n'engendre rien.
    assert_eq!(label.children.len(), 2);
    assert_eq!(text_color(&label.children[0]), Color::hex(0xffcc00));
    assert_eq!(label.children[1].bounds().left - label.bounds().left, 12.);
    arrange_headless_in(
        &mut root,
        300.,
        200.,
        StyleEnvironment {
            pointer: Some(PointF { x: 10., y: 10. }),
            ..Default::default()
        },
    );
    assert_eq!(
        root.children[0].children[0].style.background,
        Some(Color::hex(0x00ff00))
    );
}
