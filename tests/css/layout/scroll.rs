//! Conteneurs défilants : hauteur mesurée sur le contenu, colonnes flex qui ne rétrécissent
//! pas leurs enfants, barres fines, gouttière stable, barres masquées, `::-webkit-scrollbar`.
use crate::support::*;
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::{
        prelude::*,
        scene::{DrawCommand, PointF},
    },
};
css_file!(mod css, "scroll.css");

#[derive(Default)]
struct Scrolling;

impl Application for Scrolling {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div>
                <div id="page" class="page">
                    <div id="stack" class="stack">
                        <div id="b1" class="block"></div>
                        <div id="carousel" class="carousel">
                            <div class="card"></div><div class="card"></div>
                            <div class="card"></div><div class="card"></div>
                        </div>
                        <div id="inner" class="inner">
                            <div class="block"></div><div class="block"></div>
                        </div>
                        <div id="section" class="section">
                            <p class="line">Un</p><p class="line">Deux</p><p class="line">Trois</p>
                        </div>
                        <div id="b2" class="block"></div>
                    </div>
                </div>
                <div id="boxed" class="boxed">
                    <div id="first" class="section"><p class="line">A</p><p class="line">B</p><p class="line">C</p></div>
                    <div id="second" class="section"><p class="line">D</p><p class="line">E</p><p class="line">F</p></div>
                </div>
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

/// `overflow-x: auto` mesure la hauteur de son contenu (+ la barre réservée), pas
/// toute la place offerte ; un `display: flex; flex-direction: column` dans un
/// conteneur défilant prend la hauteur de son contenu sans rétrécir ses enfants.
#[test]
fn scroll_containers_and_flex_columns_keep_their_content_height() {
    let app = HeadlessApp::new(Scrolling).unwrap();
    let h = |id: &str| height(app.bounds(id).unwrap());
    // 6 + 90 + 8 de padding, + 12 de barre horizontale réservée.
    near(h("carousel"), 116.);
    // Un défilement vertical sans hauteur définie prend celle de son contenu.
    near(h("inner"), 240.);
    near(h("section"), 3. * 20. + 2. * 4.);
    near(h("b1"), 120.);
    near(h("b2"), 120.);
    near(h("stack"), 120. + 116. + 240. + 68. + 120. + 4. * 10.);
    near(h("page"), 300.);
    // Le contenu défile : la page mesure toute la pile.
    let page = app.find("page").unwrap();
    let metrics = page.scroll_metrics().unwrap();
    near(metrics.extent, h("stack"));
}

/// `min-height: auto` des éléments d'une colonne flex : une colonne trop petite
/// ne rétrécit pas ses enfants sous la hauteur de leur contenu (ils débordent).
#[test]
fn flex_column_items_do_not_shrink_below_their_content() {
    let app = HeadlessApp::new(Scrolling).unwrap();
    let h = |id: &str| height(app.bounds(id).unwrap());
    near(h("boxed"), 100.);
    near(h("first"), 68.);
    near(h("second"), 68.);
}

struct Scrollers;

impl Application for Scrollers {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div style="display:flex; gap:10px">
                <div id="default" class="scroller"><div class="tall"></div></div>
                <div id="thin" class="scroller thin"><div class="tall"></div></div>
                <div id="thin-fits" class="scroller thin"><div class="short"></div></div>
                <div id="thin-stable" class="scroller thin stable"><div class="short"></div></div>
                <div id="none" class="scroller no-bar"><div class="tall"></div></div>
                <div id="smart" class="scroller thin smart"><div class="tall"></div></div>
                <div id="smart-fits" class="scroller thin smart"><div class="short"></div></div>
            </div>
        }
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

#[test]
fn scrollbar_width_color_and_gutter_shape_scroll_containers() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(Scrollers, options(1600., 300.))?;
    let metrics = |app: &HeadlessApp<Scrollers>, id: &str| {
        app.find(id)
            .unwrap()
            .scroll_metrics()
            .expect("scroll container")
    };
    let width = |r: karbon_ui::ui::scene::RectF| r.right - r.left;
    assert_eq!(width(metrics(&app, "default").viewport), 188.);
    let thin = metrics(&app, "thin");
    assert_eq!((width(thin.viewport), thin.thickness), (190., 10.));
    assert_eq!(width(thin.thumb), 4.);
    // Contenu qui tient : pas de gouttière, sauf avec scrollbar-gutter: stable.
    assert_eq!(width(metrics(&app, "thin-fits").viewport), 200.);
    assert_eq!(width(metrics(&app, "thin-stable").viewport), 190.);
    let none = metrics(&app, "none");
    assert_eq!((width(none.viewport), none.thickness), (200., 0.));
    // Pouce rouge arrondi (scrollbar-color), piste transparente non peinte.
    let thumbs = app
        .scene()
        .commands
        .iter()
        .filter(|c| {
            matches!(c, DrawCommand::Fill { color, rect, radius }
                if *color == Color::hex(0xff0000) && *rect == thin.thumb && *radius == 2.)
        })
        .count();
    assert_eq!(thumbs, 1);
    // Sans barre, le contenu défile encore.
    app.scroll("none", 0., 120.)?;
    assert!(metrics(&app, "none").offset > 0.);
    // ::-webkit-scrollbar { width: 8px } : gouttière de 8 px (smart-scroll de RustMusic),
    // toujours réservée seulement si le contenu déborde.
    let smart = metrics(&app, "smart");
    assert_eq!((width(smart.viewport), smart.thickness), (192., 8.));
    assert_eq!(width(metrics(&app, "smart-fits").viewport), 200.);
    Ok(())
}

#[derive(Default)]
struct Sticky {
    /// Derniers clics : `true` pour un en-tête, `false` pour une ligne.
    clicks: Vec<bool>,
}

impl Application for Sticky {
    type Message = bool;
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div>
            <div id="sticky.page" class="sticky-page">
                <div id="sticky.a" class="sticky-section">
                    <div id="sticky.head.a" class="sticky-head" onclick={true}></div>
                    {#each 0..6 as row (row)}<div class="sticky-row" onclick={false}></div>{/each}
                </div>
                <div id="sticky.b" class="sticky-section">
                    <div id="sticky.head.b" class="sticky-head"></div>
                    {#each 0..6 as row (row)}<div class="sticky-row"></div>{/each}
                </div>
            </div>
            </div>
        }
    }
    fn update(&mut self, head: bool) -> bool {
        self.clicks.push(head);
        false
    }
}

/// Un en-tête collant reste en haut de la vue tant que sa section y est, puis remonte avec
/// elle ; le suivant prend sa place. Sections de 30 + 6 × 50 = 330.
#[test]
fn a_sticky_header_stays_at_the_top_of_its_scrolled_section() {
    let mut app = HeadlessApp::new(Sticky::default()).unwrap();
    let top = app.bounds("sticky.page").unwrap().top;
    let at = |app: &HeadlessApp<Sticky>, id: &str| app.bounds(id).unwrap().top - top;
    near(at(&app, "sticky.head.a"), 0.);
    near(at(&app, "sticky.head.b"), 330.);
    app.scroll("sticky.page", 0., 100.).unwrap();
    near(at(&app, "sticky.a"), -100.);
    near(at(&app, "sticky.head.a"), 0.);
    // Collé au-dessus des lignes qui passent dessous : c'est lui qui reçoit le clic.
    app.click_at(PointF { x: 20., y: top + 10. }).unwrap();
    assert_eq!(app.app().clicks, [true]);
    // Bas de la section A à 10 de la vue : son en-tête remonte avec elle.
    app.scroll("sticky.page", 0., 220.).unwrap();
    near(at(&app, "sticky.head.a"), 10. - 30.);
    near(at(&app, "sticky.head.b"), 10.);
    app.scroll("sticky.page", 0., 40.).unwrap();
    near(at(&app, "sticky.head.b"), 0.);
    // Retour en haut : chacun à sa place dans le flux.
    app.scroll("sticky.page", 0., -360.).unwrap();
    near(at(&app, "sticky.head.a"), 0.);
    near(at(&app, "sticky.head.b"), 330.);
}
