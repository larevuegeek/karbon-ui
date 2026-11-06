//! Boîtes : `box-sizing`, marges automatiques et négatives, positionnement relatif,
//! absolu et fixe, ordre `z-index` partagé par la peinture et les clics.
use crate::support::*;
use karbon_ui::ui::{
    layout,
    prelude::*,
    scene::{DrawCommand, PointF},
};
css_file!(mod css, "boxes.css");

#[test]
fn content_box_adds_padding_border_and_auto_margins_center() {
    let mut root = ui! {@styles(css);<div style="align-items:start"><div class="content"></div><div class="centered"></div><div class="negative"></div></div>};
    arrange(&mut root, 300., 200.);
    near(layout::width(root.children[0].bounds()), 124.);
    near(layout::height(root.children[0].bounds()), 44.);
    near(layout::width(layout::content_rect(&root.children[0])), 100.);
    near(root.children[1].bounds().left, 100.);
    near(root.children[2].bounds().left, -10.);
    near(root.children[2].bounds().top, 69.);
}

#[test]
fn positioned_children_use_padding_box_percentages_and_do_not_consume_flow_space() {
    let mut root = ui! {@styles(css);<div style="padding:20px"><div class="position-host"><div class="absolute"></div><div style="width:100px; height:20px"></div><div class="relative"></div><div style="width:100px; height:20px"></div><div class="absolute-stretch"></div></div><div id="after" style="height:20px"></div></div>};
    arrange(&mut root, 600., 400.);
    let host = &root.children[0];
    near(host.children[0].bounds().left, 160.);
    near(host.children[0].bounds().top, 27.);
    near(layout::width(host.children[0].bounds()), 148.);
    near(host.children[1].bounds().top, 32.);
    near(host.children[2].bounds().left, 47.);
    near(host.children[2].bounds().top, 59.);
    near(host.children[3].bounds().top, 72.);
    near(host.children[4].bounds().left, 32.);
    near(layout::width(host.children[4].bounds()), 266.);
    near(host.children[4].bounds().top, 161.);
    near(root.children[1].bounds().top, 180.);
}

#[test]
fn z_order_and_visible_overflow_use_the_same_order_for_painting_and_clicks() {
    use karbon_ui::ui::runtime::{UiEvent, UiRuntime};
    let mut ui=UiRuntime::new(View(||ui!{@styles(css);<div style="padding:20px"><div class="position-host"><button id="upper" class="z-upper" onclick={Msg::Click(2)}>Haut</button><button class="z-lower" onclick={Msg::Click(1)}>Bas</button><button id="outside" class="outside" onclick={Msg::Click(3)}>Dehors</button></div></div>}),&Services).unwrap();
    let now = std::time::Instant::now();
    prepare(&mut ui, now);
    assert!(
        ui.event(UiEvent::PointerDown(PointF { x: 140., y: 130. }))
            .handled
    );
    assert_eq!(ui.focused(), Some(ElementId::named("upper")));
    ui.event(UiEvent::Cancel);
    assert!(
        ui.event(UiEvent::PointerDown(PointF { x: 345., y: 45. }))
            .handled
    );
    assert_eq!(ui.focused(), Some(ElementId::named("outside")));
    let mut clipped=UiRuntime::new(View(||ui!{@styles(css);<div style="padding:20px"><div class="position-host hidden"><button class="outside" onclick={Msg::Click(3)}>Dehors</button></div></div>}),&Services).unwrap();
    prepare(&mut clipped, now);
    assert!(
        !clipped
            .event(UiEvent::PointerDown(PointF { x: 345., y: 45. }))
            .handled
    );
    let commands = &ui.scene().commands;
    let red = commands
        .iter()
        .position(|c| matches!(c,DrawCommand::Fill{color,..}if *color==Color::hex(0xff0000)))
        .unwrap();
    let green = commands
        .iter()
        .position(|c| matches!(c,DrawCommand::Fill{color,..}if *color==Color::hex(0x00ff00)))
        .unwrap();
    assert!(red < green);
}

#[test]
fn fixed_control_escapes_parent_clip_and_stays_at_the_viewport_when_scrolling() {
    use karbon_ui::ui::runtime::{UiEvent, UiRuntime};
    let mut ui=UiRuntime::new(View(||ui!{@styles(css);<div><div style="overflow:auto; height:120px"><div style="height:800px"><div style="height:400px"></div><div style="height:50px"><button id="fixed" class="fixed" onclick={Msg::Click(1)}>Fixe</button></div></div></div></div>}),&Services).unwrap();
    let now = std::time::Instant::now();
    prepare(&mut ui, now);
    let before = ui
        .root()
        .unwrap()
        .find(ElementId::named("fixed"))
        .unwrap()
        .bounds();
    near(before.left, 490.);
    near(before.top, 350.);
    assert!(
        ui.event(UiEvent::PointerDown(PointF { x: 520., y: 370. }))
            .handled
    );
    assert_eq!(ui.focused(), Some(ElementId::named("fixed")));
    ui.event(UiEvent::Cancel);
    ui.event(UiEvent::Scroll {
        position: PointF { x: 30., y: 40. },
        delta_x: 0.,
        delta_y: 200.,
    });
    prepare(&mut ui, now);
    let after = ui
        .root()
        .unwrap()
        .find(ElementId::named("fixed"))
        .unwrap()
        .bounds();
    assert_eq!(before, after);
    assert!(
        ui.event(UiEvent::PointerDown(PointF { x: 520., y: 370. }))
            .handled
    );
}

#[test]
fn spacing_follows_element_font_root_font_and_expressions() {
    let mut root = ui! {@styles(css);<div class="spacing-root">
        <div class="em-pad"></div>
        <div class="calc-gap"><div style="width:10px; height:10px"></div><div style="width:10px; height:10px"></div></div>
        <div class="mixed-margin"></div>
        <div class="negative-em"></div>
    </div>};
    arrange(&mut root, 300., 300.);
    // `em` : police de l'élément (10 px) ; `rem` : racine (20 px).
    let em = &root.children[0];
    near(em.bounds().left, 20.);
    near(em.bounds().top, 5.);
    near(layout::width(em.bounds()), 80.);
    near(layout::height(em.bounds()), 30.);
    near(layout::content_rect(em).left, 40.);
    near(layout::content_rect(em).top, 15.);
    // calc()/max() : gap de 22 px, marge de 10 px, padding gauche de 10 px.
    let row = &root.children[1];
    near(row.bounds().top, em.bounds().bottom + 10.);
    near(row.children[0].bounds().left, 10.);
    near(
        row.children[1].bounds().left - row.children[0].bounds().right,
        22.,
    );
    // Côté en expression et côtés `auto` dans un même raccourci.
    let mixed = &root.children[2];
    near(mixed.bounds().left, 100.);
    near(mixed.bounds().top, row.bounds().bottom + 20.);
    // Marge négative en em et padding lu par var().
    let negative = &root.children[3];
    near(negative.bounds().left, -10.);
    near(
        layout::content_rect(negative).top - negative.bounds().top,
        15.,
    );
}

#[test]
fn spacing_in_viewport_units_and_conditional_rules_follows_the_viewport() {
    use karbon_ui::ui::{
        element::SizeF,
        scene::RectF,
        styling::{self, StyleEnvironment},
    };
    let mut root = ui! {@styles(css);<div style="font-size:20px; align-items:start">
        <div class="viewport-pad"></div><div class="media-pad"></div><div class="media-px"></div>
    </div>};
    let mut run = |width: f32, height: f32| {
        let env = StyleEnvironment {
            viewport: SizeF { width, height },
            ..Default::default()
        };
        styling::resolve(&mut root, env).unwrap();
        layout::validate(&root).unwrap();
        let r = RectF::new(0., 0., 600., 600.);
        layout::arrange(&mut root, r, r, &mut Measure).unwrap();
        let pad = |i: usize| {
            let node = &root.children[i];
            let content = layout::content_rect(node);
            [
                content.top - node.bounds().top,
                node.bounds().right - content.right,
                node.bounds().bottom - content.bottom,
                content.left - node.bounds().left,
            ]
        };
        [pad(0), pad(1), pad(2)]
    };
    let check = |a: [f32; 4], b: [f32; 4]| a.into_iter().zip(b).for_each(|(a, b)| near(a, b));
    // Grand viewport : 2vw = 20 px, 1vh = 6 px ; padding 1em = 10 px ; 4 px.
    let [viewport, media, px] = run(1000., 600.);
    check(viewport, [20., 0., 0., 6.]);
    check(media, [10.; 4]);
    check(px, [4.; 4]);
    // Petit viewport : expressions réévaluées ; règle conditionnelle en px sur une base en
    // em, et en rem (racine de 20 px) sur une base en px.
    let [viewport, media, px] = run(400., 300.);
    check(viewport, [8., 0., 0., 3.]);
    check(media, [2., 10., 10., 10.]);
    check(px, [4., 4., 4., 20.]);
}
