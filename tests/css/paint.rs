//! Peinture : fonds et dégradés superposés, coins arrondis, ombres mises en cache, images
//! `object-fit`, filtres, découpe arrondie et clics, bordures par côté, contours, calage
//! des bordures sur les pixels physiques, dégradés radiaux, `backdrop-filter`, masques
//! et texte rempli d'un dégradé.
use crate::support::*;
use karbon_ui::ui::{
    element::SizeF,
    layout,
    prelude::*,
    scene::{DrawCommand, PointF},
    styling::StyleEnvironment,
};
css_file!(mod css, "paint.css");

#[test]
fn object_fit_positions_a_shared_image_and_clips_cover_to_the_content_box() {
    use karbon_ui::ui::scene::Image;
    use std::sync::Arc;
    let image = Arc::new(Image::new(200, 100, vec![255; 200 * 100 * 4]).unwrap());
    let mut root = ui! {@styles(css);<div style:flex-direction={"row"}><img src={Arc::clone(&image)} class="cover" style="width:100px; height:100px"><img src={Arc::clone(&image)} class="contain" style="width:100px; height:100px"></div>};
    arrange(&mut root, 200., 100.);
    let a = paint(&root.children[0]);
    let b = paint(&root.children[1]);
    a.validate().unwrap();
    b.validate().unwrap();
    assert!(matches!(a.commands[0], DrawCommand::PushClip(_)));
    assert!(
        matches!(&a.commands[1],DrawCommand::Image{rect,image:source}if rect.left == -100.&&rect.top==0.&&layout::width(*rect)==200.&&Arc::ptr_eq(source,&image))
    );
    assert!(
        matches!(&b.commands[1],DrawCommand::Image{rect,..}if rect.left==100.&&rect.top==25.&&layout::height(*rect)==50.)
    );
}

#[test]
fn layered_gradients_rounded_shapes_and_cached_outer_inset_shadows_use_the_native_scene() {
    use karbon_ui::ui::rendering::brush::Background;
    use karbon_ui::ui::runtime::{UiEvent, UiRuntime};
    let mut ui = UiRuntime::new(
        View(|| ui! {@styles(css);<div style="padding:40px"><div class="visual"></div></div>}),
        &Services,
    )
    .unwrap();
    let now = std::time::Instant::now();
    prepare(&mut ui, now);
    ui.scene().validate().unwrap();
    let commands = &ui.scene().commands;
    let fills = commands
        .iter()
        .filter_map(|c| {
            if let DrawCommand::Shape {
                fill: Some(bg),
                radii,
                ..
            } = c
            {
                assert_eq!(*radii, [20., 0., 12., 4.]);
                Some(bg)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 3);
    assert!(matches!(fills[0], Background::Solid(_)));
    assert!(matches!(fills[1], Background::Linear(_)));
    assert!(matches!(fills[2], Background::Radial(_)));
    let images = commands
        .iter()
        .filter_map(|c| {
            if let DrawCommand::Image { image, rect } = c {
                Some((std::sync::Arc::clone(image), *rect))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(images.len(), 2);
    assert!(images[0].1.left < 40.);
    let shadow_index = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Image { .. }))
        .unwrap();
    let shape_index = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Shape { .. }))
        .unwrap();
    assert!(shadow_index < shape_index);
    let layouts = ui.stats().layouts;
    ui.event(UiEvent::PointerMove(PointF { x: 80., y: 60. }));
    prepare(&mut ui, now + std::time::Duration::from_millis(20));
    let next = ui
        .scene()
        .commands
        .iter()
        .find_map(|c| {
            if let DrawCommand::Image { image, .. } = c {
                Some(image)
            } else {
                None
            }
        })
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(next, &images[0].0));
    assert_eq!(ui.stats().layouts, layouts);
}

#[test]
fn css_filters_wrap_the_subtree_and_hover_can_remove_them_without_relayout() {
    use karbon_ui::ui::runtime::{UiEvent, UiRuntime};
    let mut ui=UiRuntime::new(View(||ui!{@styles(css);<div style="padding:30px"><div class="filtered" style="width:200px; height:80px; background:#204060"><span>Filtré</span></div></div>}),&Services).unwrap();
    let now = std::time::Instant::now();
    prepare(&mut ui, now);
    ui.scene().validate().unwrap();
    let scope = ui
        .scene()
        .commands
        .iter()
        .position(|c| matches!(c, DrawCommand::PushFilter { .. }))
        .unwrap();
    let text = ui
        .scene()
        .commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Text { .. } | DrawCommand::FontText { .. }))
        .unwrap();
    let end = ui
        .scene()
        .commands
        .iter()
        .position(|c| matches!(c, DrawCommand::PopFilter))
        .unwrap();
    assert!(scope < text && text < end);
    let layouts = ui.stats().layouts;
    ui.event(UiEvent::PointerMove(PointF { x: 60., y: 60. }));
    prepare(&mut ui, now);
    assert!(
        !ui.scene()
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::PushFilter { .. }))
    );
    assert_eq!(ui.stats().layouts, layouts);
}

#[test]
fn rounded_overflow_clips_children_and_excludes_corner_clicks() {
    use karbon_ui::ui::runtime::{UiEvent, UiRuntime};
    let mut ui = UiRuntime::new(
        View(|| {
            ui! {
                <div style="width:100px; height:100px; border-radius:40px; overflow:hidden">
                    <button id="corner-button" style:width={FILL} style="height:100px" onclick={Msg::Click(1)}>Action</button>
                </div>
            }
        }),
        &Services,
    )
    .unwrap();
    let now = std::time::Instant::now();
    prepare(&mut ui, now);
    ui.scene().validate().unwrap();
    assert!(
        ui.scene()
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::PushShapeClip { .. }))
    );
    assert!(
        !ui.event(UiEvent::PointerDown(PointF { x: 1., y: 1. }))
            .handled
    );
    assert!(
        ui.event(UiEvent::PointerDown(PointF { x: 50., y: 50. }))
            .handled
    );
    assert_eq!(ui.focused(), Some(ElementId::named("corner-button")));
}

#[test]
fn single_side_borders_and_outlines_paint_without_layout_changes() {
    let scene = scene(|| {
        ui! {@styles(css);<div style="padding: 20px">
            <div class="bottom-line"></div><div class="sides"></div><div class="outlined"></div>
        </div>}
    });
    let fills = scene
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Fill { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect::<Vec<_>>();
    // Une seule bande rouge de 2 px en bas de la première boîte (y = 20 + 20 − 2).
    let red = fills
        .iter()
        .filter(|(_, c)| *c == Color::hex(0xff0000))
        .collect::<Vec<_>>();
    assert_eq!(red.len(), 1);
    assert_eq!((red[0].0.top, layout::height(red[0].0)), (38., 2.));
    assert!(
        fills
            .iter()
            .any(|(r, c)| *c == Color::hex(0x0000ff) && layout::width(*r) == 4.)
    );
    assert_eq!(
        fills
            .iter()
            .filter(|(_, c)| *c == Color::hex(0x00ff00))
            .count(),
        3
    );
    // Outline : trait de 2 px décalé de 3 px (centre à 4 px du bord).
    let ring = scene
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::Shape {
                rect,
                stroke: Some((2., color)),
                ..
            } if *color == Color::hex(0xffff00) => Some(*rect),
            _ => None,
        })
        .expect("outline");
    assert_eq!(layout::width(ring), 28.);
}

#[test]
fn rounded_border_without_one_side_keeps_its_corners() {
    let scene =
        scene(|| ui! {@styles(css);<div style="padding: 20px"><div class="tab"></div></div>});
    // Contour arrondi uniforme sur la boîte prolongée sous le côté absent, rogné à la boîte.
    let at = scene
        .commands
        .iter()
        .position(|c| {
            matches!(c, DrawCommand::Shape { radii, stroke: Some((w, color)), .. }
                if *radii == [8., 8., 0., 0.] && *w == 1. && *color == Color::hex(0x00ffff))
        })
        .expect("rounded stroke");
    let DrawCommand::Shape { rect, .. } = &scene.commands[at] else {
        unreachable!()
    };
    assert_eq!((rect.top, rect.bottom), (20., 37.));
    assert!(matches!(&scene.commands[at - 1], DrawCommand::PushClip(clip) if clip.bottom == 36.));
    assert!(matches!(&scene.commands[at + 1], DrawCommand::PopClip));
}

#[test]
fn border_widths_snap_to_physical_pixels() {
    let height = |pixel_scale| {
        let mut root = ui! {@styles(css);<div><div class="snapped"><div style="height: 10px"></div></div></div>};
        arrange_headless_in(
            &mut root,
            100.,
            100.,
            StyleEnvironment {
                viewport: SizeF {
                    width: 100.,
                    height: 100.,
                },
                pixel_scale,
                ..Default::default()
            },
        );
        layout::height(root.children[0].bounds())
    };
    assert_eq!(height(1.), 12.);
    // À 150 %, 1 px CSS devient un pixel physique : 2/3 de pixel logique, comme Chromium.
    assert!((height(1.5) - (10. + 4. / 3.)).abs() < 1e-3);
    assert_eq!(height(2.), 12.);
}

#[test]
fn radial_gradients_accept_explicit_radii() {
    use karbon_ui::ui::rendering::brush::{Background, RadialExtent};
    let scene =
        scene(|| ui! {@styles(css);<div><div class="spot"></div><div class="oval"></div></div>});
    let radials = scene
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Shape {
                fill: Some(Background::Radial(g)),
                ..
            } => Some(g.clone()),
            DrawCommand::RadialGradient { gradient, .. } => Some(gradient.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(radials.len(), 2);
    assert!(matches!(radials[0].extent, RadialExtent::Absolute));
    assert_eq!(radials[0].radii(100., 100.), [40., 40.]);
    assert_eq!(radials[1].radii(100., 100.), [80., 170.]);
}

#[test]
fn backdrop_mask_and_text_clip_emit_native_scopes() {
    use karbon_ui::ui::rendering::brush::Background;
    let scene = scene(|| {
        ui! {@styles(css);<div>
            <div class="glass"></div><div class="faded"><p>Liste</p></div>
            <div class="gradient-text"><p>Dégradé</p></div>
        </div>}
    });
    scene.validate().unwrap();
    let c = &scene.commands;
    assert!(
        c.iter()
            .any(|c| matches!(c, DrawCommand::Backdrop { radii, filters, .. }
        if *radii == [12.; 4] && filters.values.len() == 1))
    );
    let mask = c
        .iter()
        .position(|c| matches!(c, DrawCommand::PushMask { .. }))
        .unwrap();
    let red = c
        .iter()
        .position(
            |c| matches!(c, DrawCommand::Fill { color, .. } if *color == Color::hex(0xff0000)),
        )
        .unwrap();
    let pop = c
        .iter()
        .position(|c| matches!(c, DrawCommand::PopMask))
        .unwrap();
    assert!(mask < red && red < pop);
    // Le dégradé ne se peint pas en fond : il remplit le texte.
    let fill = c
        .iter()
        .position(|c| {
            matches!(
                c,
                DrawCommand::PushTextFill {
                    fill: Background::Linear(_),
                    ..
                }
            )
        })
        .unwrap();
    let text = c
        .iter()
        .position(|c| matches!(c, DrawCommand::FontText { text, .. } | DrawCommand::FormattedText { text, .. } if text.contains("Dégradé")))
        .unwrap();
    assert!(fill < text);
    assert!(!c.iter().any(|c| matches!(
        c,
        DrawCommand::Shape {
            fill: Some(Background::Linear(_)),
            ..
        } | DrawCommand::Gradient { .. }
    )));
}
