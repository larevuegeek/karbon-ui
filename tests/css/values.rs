//! Valeurs : `@import`, variables de la feuille et de l'application, unités relatives et
//! expressions (`calc`, `clamp`, `min`), fonctions de couleur, `currentColor`, propriétés
//! personnalisées héritées, classes CSS et recettes Rust mêlées.
use crate::support::*;
use karbon_ui::ui::{
    element::{Layout, SizeF},
    layout,
    prelude::*,
    runtime::UiRuntime,
    scene::{DrawCommand, PointF, RectF},
    styling::{self, CssVariables, StyleEnvironment},
    template::TemplateContext,
};
use std::time::Instant;
css_file!(mod css, "values.css");
styles! {RUST {margin_left:17;font_size:22;font_weight:800;}}

#[test]
fn imports_variables_class_names_box_geometry_and_font_properties() {
    let mut root = ui! {@styles(css);
        <div style="padding:10px">
            <div id="panel" class="panel"><span id="title" class="title">Bonjour</span></div>
            <div id="next" style="height:10px"></div>
            <div id="grid" class="grid">
                <div style="height:10px"></div><div style="height:10px"></div><div style="height:10px"></div>
            </div>
        </div>
    };
    let bounds = RectF::new(0., 0., 600., 400.);
    layout::validate(&root).unwrap();
    layout::arrange(&mut root, bounds, bounds, &mut Measure).unwrap();
    let panel = root.find(ElementId::named("panel")).unwrap();
    near(panel.bounds().left, 21.);
    near(panel.bounds().top, 13.);
    near(layout::width(panel.bounds()), 200.);
    near(
        root.find(ElementId::named("next")).unwrap().bounds().top,
        54.,
    );
    let title = root.find(ElementId::named("title")).unwrap();
    near(title.bounds().left, 30.);
    near(title.bounds().top, 16.);
    let mut scene = karbon_ui::ui::scene::Scene::new(Color::hex(0));
    title.widget.as_ref().unwrap().paint(
        &karbon_ui::ui::element::PaintContext {
            rect: title.bounds(),
            intrinsic: SizeF::default(),
            theme: Theme::DARK,
            state: Default::default(),
        },
        &mut scene,
    );
    assert!(
        matches!(&scene.commands[0],DrawCommand::FontText{font,size,..} if font.family=="Consolas"&&font.weight==700&&*size==20.)
    );
    let grid = root.find(ElementId::named("grid")).unwrap();
    assert!(matches!(grid.layout, Layout::Grid { columns: 2 }));
    near(
        grid.children[1].bounds().left - grid.children[0].bounds().right,
        7.,
    );
    near(
        grid.children[2].bounds().top - grid.children[0].bounds().bottom,
        11.,
    );
}

#[test]
fn percentages_exclude_margins_and_rust_css_classes_compose_at_a_logical_scale() {
    let mut root = ui! {@styles(css);<div style:flex-direction={"row"}><div id="wide" class="wide"></div></div>};
    let bounds = RectF::new(0., 0., 600., 100.);
    layout::validate(&root).unwrap();
    layout::arrange(&mut root, bounds, bounds, &mut Measure).unwrap();
    let child = &root.children[0];
    near(child.bounds().left, 10.);
    near(layout::width(child.bounds()), 300.);
    let mut row = ui! {<div style:flex-direction={"row"} style="align-items:center">
        <div style:width={FILL} style="max-width:100px; height:20px" style:margin={10}></div>
        <div style:width={FILL} style="height:20px" style:margin={10}></div>
    </div>};
    let allocation = RectF::new(0., 0., 300., 100.);
    layout::validate(&row).unwrap();
    layout::arrange(&mut row, allocation, allocation, &mut Measure).unwrap();
    near(row.children[0].bounds().left, 10.);
    near(row.children[0].bounds().top, 40.);
    near(layout::width(row.children[0].bounds()), 100.);
    near(row.children[1].bounds().left, 130.);
    near(layout::width(row.children[1].bounds()), 160.);
    let cx = TemplateContext::default().with_scale(2.);
    let node = ui! {@context(cx);<span class={[css::TITLE,RUST]}>Mixte</span>};
    near(node.style.margin.left, 34.);
    let mut node = node;
    layout::arrange(&mut node, bounds, bounds, &mut Measure).unwrap();
    let mut scene = karbon_ui::ui::scene::Scene::new(Color::hex(0));
    node.widget.as_ref().unwrap().paint(
        &karbon_ui::ui::element::PaintContext {
            rect: bounds,
            intrinsic: SizeF::default(),
            theme: Theme::DARK,
            state: Default::default(),
        },
        &mut scene,
    );
    assert!(
        matches!(&scene.commands[0],DrawCommand::FontText{font,size,..} if font.weight==800&&font.family=="Consolas"&&*size==44.)
    );
}

#[test]
fn layered_background_painting_keeps_solid_color_below_a_transparent_gradient() {
    struct Background;
    impl Application for Background {
        type Message = ();
        fn update(&mut self, _: ()) -> bool {
            false
        }
        fn view(&self, _: &ViewContext) -> Element {
            ui! {@styles(css);<div><div id="panel" class="panel"><span>Fond</span></div></div>}
        }
    }
    let mut ui = UiRuntime::new(Background, &Services).unwrap();
    prepare(&mut ui, Instant::now());
    let commands = &ui.scene().commands;
    let solid = commands
        .iter()
        .position(|c| matches!(c,DrawCommand::Fill{color,..}if *color==Color::hex(0x123456)))
        .unwrap();
    let gradient = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Gradient { .. }))
        .unwrap();
    assert!(solid < gradient);
    ui.scene().validate().unwrap();
}

#[test]
fn compiled_dimension_expressions_follow_parent_font_root_font_and_viewport() {
    let mut root = ui! {@styles(css);<div style="font-size:18px"><div class="units"><span class="relative-text">Unités</span></div></div>};
    styling::resolve(
        &mut root,
        StyleEnvironment {
            viewport: SizeF {
                width: 1000.,
                height: 600.,
            },
            ..Default::default()
        },
    )
    .unwrap();
    layout::validate(&root).unwrap();
    let r = RectF::new(0., 0., 600., 400.);
    layout::arrange(&mut root, r, r, &mut Measure).unwrap();
    let child = &root.children[0];
    near(layout::width(child.bounds()), 260.);
    near(layout::height(child.bounds()), 60.);
    let text = &child.children[0];
    near(layout::width(text.bounds()), 180.);
    assert!(
        paint(text)
            .commands
            .iter()
            .any(|c| matches!(c,DrawCommand::FontText{size,..}if *size==30.))
    );
    styling::resolve(
        &mut root,
        StyleEnvironment {
            viewport: SizeF {
                width: 400.,
                height: 1000.,
            },
            ..Default::default()
        },
    )
    .unwrap();
    layout::arrange(&mut root, r, r, &mut Measure).unwrap();
    near(layout::height(root.children[0].bounds()), 80.);
    near(layout::width(root.children[0].children[0].bounds()), 120.);
}

#[test]
fn typed_dynamic_variables_override_aliases_calc_fonts_padding_and_preserve_old_view_snapshots() {
    use karbon_ui::ui::styling::CssVariables;
    let mut vars = CssVariables::default();
    let cx = TemplateContext::default().with_variables(&vars);
    let mut old = ui! {@context(cx);@styles(css);<div style="padding:10px"><div class="dynamic"><span>Dynamique</span></div></div>};
    arrange(&mut old, 600., 400.);
    near(layout::width(old.children[0].bounds()), 270.);
    near(layout::height(old.children[0].bounds()), 40.);
    vars.set_length("--gutter", 50.);
    vars.set_length("--height", 80.);
    vars.set_length("--type-size", 24.);
    vars.set_family("--family", "Segoe UI");
    vars.set_color("--accent", Color::hex(0xff0080));
    vars.set_number("--alpha", 0.75);
    vars.set_insets("--pad", [2., 6.]);
    let cx = TemplateContext::default().with_variables(&vars);
    let mut new = ui! {@context(cx);@styles(css);<div style="padding:10px"><div class="dynamic"><span>Dynamique</span></div></div>};
    arrange(&mut new, 600., 400.);
    near(layout::width(new.children[0].bounds()), 240.);
    near(layout::height(new.children[0].bounds()), 80.);
    near(layout::width(layout::content_rect(&new.children[0])), 228.);
    assert!(paint(&new.children[0].children[0]).commands.iter().any(|c|matches!(c,DrawCommand::FontText{font,size,color,..}if font.family=="Segoe UI"&&*size==24.&&*color==Color::hex(0xff0080))));
    arrange(&mut old, 600., 400.);
    near(layout::width(old.children[0].bounds()), 270.);
}

#[test]
fn cyclic_runtime_dimension_variables_return_an_error_without_recursing_forever() {
    use karbon_ui::ui::{
        element::Length,
        layout::{ComputedLength, Expression},
        styling::CssVariables,
    };
    static CYCLE: Expression = Expression::Variable {
        name: "--gutter",
        fallback: &Expression::Px(20.),
        length: true,
    };
    let mut vars = CssVariables::default();
    vars.set_length("--gutter", Length::Computed(ComputedLength::new(&CYCLE)));
    let cx = TemplateContext::default().with_variables(&vars);
    let mut root = ui! {@context(cx);@styles(css);<div><div class="dynamic"></div></div>};
    styling::resolve(&mut root, StyleEnvironment::default()).unwrap();
    assert!(
        layout::measure(
            &mut root,
            SizeF {
                width: 600.,
                height: 400.
            },
            &mut Measure
        )
        .is_err()
    );
}

#[test]
fn color_functions_are_evaluated_at_compile_time() {
    let mut root = ui! {@styles(css);<div>
        <div class="red-oklch"></div><div class="green-hsl"></div><div class="legacy-hsl"></div>
        <div class="mix-srgb"></div><div class="mix-oklch"></div>
    </div>};
    arrange_headless(&mut root, 100., 100.);
    let bg = |i: usize| root.children[i].style.background;
    assert_eq!(bg(0), Some(Color::hex(0xff0000)));
    assert_eq!(bg(1), Some(Color::rgba(0, 128, 0, 128)));
    assert_eq!(bg(2), Some(Color::hex(0xff0000)));
    assert_eq!(bg(3), Some(Color::hex(0x404040)));
    // Milieu de blanc et noir en OKLCH : L = 0,5, gris neutre.
    let mid = bg(4).unwrap();
    assert!(mid.red == mid.green && mid.green == mid.blue && (95..=101).contains(&mid.red));
}

#[test]
fn current_color_follows_the_computed_text_color() {
    let scene =
        scene(|| ui! {@styles(css);<div class="current" style="width: 50px; height: 20px"></div>});
    assert!(scene.commands.iter().any(|c| matches!(c,
        DrawCommand::Shape { stroke: Some((_, color)), .. } | DrawCommand::Stroke { color, .. }
            if *color == Color::hex(0x336699))));
    let mut root = ui! {@styles(css);<div class="current"></div>};
    arrange_headless(&mut root, 100., 100.);
    assert_eq!(
        root.style.background,
        Some(Color::rgba(0x33, 0x66, 0x99, 128))
    );
}

#[test]
fn colors_with_variables_resolve_at_runtime_from_app_variables_and_states() {
    let mut vars = CssVariables::default();
    let build = |vars: &CssVariables| {
        let cx = TemplateContext::default().with_variables(vars);
        ui! {@styles(css);@context(cx);<div class="hue" style="width: 50px; height: 50px"></div>}
    };
    let mut root = build(&vars);
    arrange_headless(&mut root, 100., 100.);
    let base = root.style.background.unwrap();
    // --hue: 250 (racine) : bleu.
    assert!(base.blue > base.red, "{base:?}");
    vars.set_number("--hue", 29.23);
    vars.set_color("--brand", Color::hex(0x123456));
    let mut root = build(&vars);
    arrange_headless(&mut root, 100., 100.);
    assert_eq!(root.style.background, Some(Color::hex(0xff0000)));
    let hovered = StyleEnvironment {
        pointer: Some(PointF { x: 5., y: 5. }),
        ..Default::default()
    };
    arrange_headless_in(&mut root, 100., 100., hovered);
    assert_ne!(root.style.background, Some(Color::hex(0xff0000)));
}

#[test]
fn custom_properties_inherit_from_classes_states_and_template_values() {
    let hue = 29.23f32;
    let mut root = ui! {@styles(css);<div class="palette">
        <div class="swatch"></div>
        <div style:--h={hue} style="--w: 5px"><div class="swatch"></div></div>
    </div>};
    arrange_headless(&mut root, 200., 100.);
    let first = &root.children[0];
    let green = first.style.background.unwrap();
    assert!(
        green.green > green.red && green.green > green.blue,
        "{green:?}"
    );
    // --w: 30px hérité de .palette dans calc().
    assert_eq!(layout::width(first.bounds()), 60.);
    let nested = &root.children[1].children[0];
    assert_eq!(nested.style.background, Some(Color::hex(0xff0000)));
    assert_eq!(layout::width(nested.bounds()), 10.);
    // Survol de .palette : la règle d'état redéfinit --h pour tout le sous-arbre.
    arrange_headless_in(
        &mut root,
        200.,
        100.,
        StyleEnvironment {
            pointer: Some(PointF { x: 150., y: 90. }),
            ..Default::default()
        },
    );
    assert_eq!(
        root.children[0].style.background,
        Some(Color::hex(0xff0000))
    );
}

#[test]
fn element_custom_properties_take_precedence_over_application_variables() {
    let mut vars = CssVariables::default();
    vars.set_number("--h", 140.);
    vars.set_length("--w", karbon_ui::ui::element::Length::Px(15.));
    let cx = TemplateContext::default().with_variables(&vars);
    let mut root = ui! {@styles(css);@context(cx);<div>
        <div class="swatch"></div>
        <div class="palette"><div class="swatch"></div></div>
        <div style:--h={29.23f32}><div class="swatch"></div></div>
    </div>};
    arrange_headless(&mut root, 200., 100.);
    // Variable de l'application, sans déclaration d'élément.
    assert_eq!(layout::width(root.children[0].bounds()), 30.);
    let green = root.children[0].style.background.unwrap();
    assert!(green.green > green.red);
    // La classe .palette (--w: 30px) l'emporte sur l'application.
    assert_eq!(layout::width(root.children[1].children[0].bounds()), 60.);
    assert_eq!(
        root.children[2].children[0].style.background,
        Some(Color::hex(0xff0000))
    );
}
