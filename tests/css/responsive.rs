//! Requêtes média (imbriquées, de plage, d'orientation) et requêtes de conteneur : la
//! mise en page suit la fenêtre ou le conteneur, dans les deux sens.
use crate::support::*;
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::{
        element::SizeF,
        layout,
        prelude::*,
        scene::{DrawCommand, PointF, RectF},
        styling::{self, StyleEnvironment},
    },
};
css_file!(mod css, "responsive.css");

#[test]
fn nested_media_queries_reflow_and_restore_typography_borders_and_axis() {
    let mut root = ui! {@styles(css);<div style="padding:10px; align-items:start"><div class="responsive"><p style="margin:0">A</p><p style="margin:0">B</p></div></div>};
    let r = RectF::new(0., 0., 600., 400.);
    let env = |w, h| StyleEnvironment {
        viewport: SizeF {
            width: w,
            height: h,
        },
        pointer: Some(PointF { x: 20., y: 20. }),
        ..Default::default()
    };
    styling::resolve(&mut root, env(600., 400.)).unwrap();
    layout::arrange(&mut root, r, r, &mut Measure).unwrap();
    let change = styling::resolve(&mut root, env(600., 400.)).unwrap();
    assert!(change.paint);
    assert!(matches!(
        root.children[0].layout,
        karbon_ui::ui::element::Layout::Row
    ));
    near(layout::width(root.children[0].bounds()), 270.);
    assert!(paint(&root.children[0].children[0]).commands.iter().any(|c|matches!(c,DrawCommand::FontText{size,color,..}if *size==24.&&*color==Color::hex(0xff00ff))));
    let change = styling::resolve(&mut root, env(400., 800.)).unwrap();
    assert!(change.layout);
    layout::arrange(&mut root, r, r, &mut Measure).unwrap();
    assert!(matches!(
        root.children[0].layout,
        karbon_ui::ui::element::Layout::Column
    ));
    near(layout::width(root.children[0].bounds()), 100.);
    near(layout::width(layout::content_rect(&root.children[0])), 94.);
    assert!(
        paint(&root.children[0].children[0])
            .commands
            .iter()
            .any(|c| matches!(c,DrawCommand::FontText{size,..}if *size==16.))
    );
    styling::resolve(&mut root, env(800., 400.)).unwrap();
    layout::arrange(&mut root, r, r, &mut Measure).unwrap();
    near(layout::width(layout::content_rect(&root.children[0])), 254.);
}

fn responsive() -> Element {
    ui! {@styles(css);<div>
        <div class="resp"><div class="cell"></div><div class="cell first"></div></div>
        <div class="wide-only"></div>
        <div class="narrow-only"><button onclick={Msg::Click(0)}>Hidden</button></div>
        <div class="dir"><div class="dir-item"></div><div class="dir-item first"></div></div>
        <div class="rows"><div></div><div></div><div></div></div>
    </div>}
}

#[test]
fn media_queries_switch_display_grid_tracks_order_and_direction() {
    let mut root = responsive();
    arrange_headless(&mut root, 600., 600.);
    let c = &root.children;
    let grid = &c[0].children;
    assert_eq!(layout::width(grid[0].bounds()), 100.);
    assert_eq!(grid[1].bounds().left, 110.);
    assert_eq!(layout::height(c[1].bounds()), 20.);
    // Masqué : hors du flux, boîte et clip vides.
    assert_eq!(layout::height(c[2].bounds()), 0.);
    assert_eq!(layout::width(c[2].children[0].bounds()), 0.);
    assert_eq!(c[3].children[1].bounds().left, 50.);
    // Pistes de lignes : 30 px, reste (200 − 30 − 20), 20 px.
    let rows = &c[4].children;
    assert_eq!(layout::height(rows[0].bounds()), 30.);
    assert_eq!(layout::height(rows[1].bounds()), 150.);
    assert_eq!(rows[2].bounds().top - c[4].bounds().top, 180.);

    arrange_headless(&mut root, 300., 600.);
    let c = &root.children;
    let grid = &c[0].children;
    // Une colonne, et `.first` (order: -1) passe avant.
    assert_eq!(layout::width(grid[0].bounds()), 300.);
    assert!(grid[1].bounds().top < grid[0].bounds().top);
    assert_eq!(layout::height(c[1].bounds()), 0.);
    assert_eq!(layout::height(c[2].bounds()), 20.);
    // Colonne avec gap 4 : `.first` en tête.
    assert_eq!(c[3].children[1].bounds().top, c[3].bounds().top);
    assert_eq!(c[3].children[0].bounds().top, c[3].bounds().top + 14.);

    // Retour à la largeur initiale : état de base restauré.
    arrange_headless(&mut root, 600., 600.);
    assert_eq!(layout::width(root.children[0].children[0].bounds()), 100.);
    assert_eq!(layout::height(root.children[2].bounds()), 0.);
}

#[test]
fn hidden_subtrees_leave_focus_and_accessibility() {
    let mut ui = runtime(responsive);
    prepare_headless(&mut ui, std::time::Instant::now());
    assert!(!ui.accessibility().iter().any(|a| a.name == "Hidden"));
    assert!(!ui.scene().commands.iter().any(|c| matches!(c,
        DrawCommand::FormattedText { text, .. } | DrawCommand::FontText { text, .. } if &**text == "Hidden")));
}

struct Containers;

impl Application for Containers {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div>
                <div id="panel" class="cq">
                    <div class="cq-row">
                        <div id="a" class="cq-cell"></div>
                        <div id="b" class="cq-cell"></div>
                    </div>
                    <p id="label" class="cq-label">Libellé</p>
                    <div id="optional" class="cq-hidden" style="height:12px"></div>
                </div>
                <div id="outside" class="cq-row"><div id="c" class="cq-cell"></div><div id="d" class="cq-cell"></div></div>
                <div id="wide" class="cq-wide"></div>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

fn label_color(app: &HeadlessApp<Containers>) -> Option<Color> {
    app.scene().commands.iter().find_map(|c| match c {
        DrawCommand::RichText {
            paragraph, color, ..
        } if &*paragraph.text == "Libellé" => Some(*color),
        DrawCommand::FormattedText { text, color, .. }
        | DrawCommand::FontText { text, color, .. }
            if &**text == "Libellé" =>
        {
            Some(*color)
        }
        _ => None,
    })
}

#[test]
fn container_queries_follow_the_container_width() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(Containers, options(800., 400.))?;
    // Conteneur de 400 px : deux colonnes séparées de 10 px.
    let (a, b) = (app.bounds("a").unwrap(), app.bounds("b").unwrap());
    assert_eq!(a.top, b.top);
    assert_eq!(b.left - a.right, 10.);
    assert_eq!(label_color(&app), Some(Color::hex(0x000000)));
    let optional = app.bounds("optional").unwrap();
    assert_eq!(optional.bottom - optional.top, 12.);
    // Hors conteneur : la requête ne s'applique pas (une seule colonne).
    let (c, d) = (app.bounds("c").unwrap(), app.bounds("d").unwrap());
    assert!(d.top >= c.bottom);
    // @media en syntaxe de plage.
    let wide = app.bounds("wide").unwrap();
    assert_eq!(wide.bottom - wide.top, 30.);
    // Fenêtre de 500 px : conteneur de 250 px, une colonne, libellé rouge, bloc masqué.
    app.resize(500., 400.)?;
    let (a, b) = (app.bounds("a").unwrap(), app.bounds("b").unwrap());
    assert!(b.top >= a.bottom, "{a:?} {b:?}");
    assert_eq!(label_color(&app), Some(Color::hex(0xff0000)));
    let optional = app.bounds("optional").unwrap();
    assert_eq!(optional.bottom - optional.top, 0.);
    let wide = app.bounds("wide").unwrap();
    assert_eq!(wide.bottom - wide.top, 10.);
    // Retour à 800 px : la requête se réévalue dans l'autre sens.
    app.resize(800., 400.)?;
    let (a, b) = (app.bounds("a").unwrap(), app.bounds("b").unwrap());
    assert_eq!(a.top, b.top);
    Ok(())
}

struct Themed(std::rc::Rc<std::cell::Cell<ColorPreferences>>);

impl Application for Themed {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div memo={0u8}><p class="themed">Thème</p></div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
    fn color_preferences(&self) -> ColorPreferences {
        self.0.get()
    }
}

fn themed_color(app: &HeadlessApp<Themed>) -> Option<Color> {
    app.scene().commands.iter().find_map(|c| match c {
        DrawCommand::RichText {
            paragraph, color, ..
        } if &*paragraph.text == "Thème" => Some(*color),
        DrawCommand::FormattedText { text, color, .. }
        | DrawCommand::FontText { text, color, .. }
            if &**text == "Thème" =>
        {
            Some(*color)
        }
        _ => None,
    })
}

#[test]
fn colour_preferences_of_the_application_drive_the_media_features() -> karbon_ui::Result<()> {
    let preferences = std::rc::Rc::new(std::cell::Cell::new(ColorPreferences {
        scheme: ColorScheme::Light,
        high_contrast: false,
    }));
    let mut app = HeadlessApp::with_options(Themed(preferences.clone()), options(800., 400.))?;
    assert_eq!(themed_color(&app), Some(Color::hex(0x111111)));
    // Sous-arbre mémorisé : il suit quand même le thème.
    preferences.set(ColorPreferences {
        scheme: ColorScheme::Dark,
        high_contrast: false,
    });
    app.resize(801., 400.)?;
    assert_eq!(themed_color(&app), Some(Color::hex(0x0000ff)));
    preferences.set(ColorPreferences {
        scheme: ColorScheme::Dark,
        high_contrast: true,
    });
    app.resize(800., 400.)?;
    assert_eq!(themed_color(&app), Some(Color::hex(0x00ff00)));
    Ok(())
}

struct ViewportUnits;

impl Application for ViewportUnits {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div>
                <div style="width:200px; height:100px">
                    <div id="memo" memo={0u8}><div id="vh" style="height:50vh"></div></div>
                </div>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

#[test]
fn a_memoized_subtree_in_viewport_units_follows_the_window() -> karbon_ui::Result<()> {
    // Sous-arbre mémorisé dans une case fixe : la place offerte ne change pas avec la
    // fenêtre, seule l'unité `vh` le fait ; ses mesures retenues ne servent plus.
    let mut app = HeadlessApp::with_options(ViewportUnits, options(800., 400.))?;
    let height = |app: &HeadlessApp<ViewportUnits>, id| {
        let r = app.bounds(id).unwrap();
        r.bottom - r.top
    };
    assert_eq!(height(&app, "vh"), 200.);
    assert_eq!(height(&app, "memo"), 200.);
    app.resize(800., 600.)?;
    assert_eq!(height(&app, "vh"), 300.);
    assert_eq!(height(&app, "memo"), 300.);
    app.resize(800., 400.)?;
    assert_eq!(height(&app, "memo"), 200.);
    Ok(())
}
