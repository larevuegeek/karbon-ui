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
    app.click_at(PointF {
        x: 20.,
        y: top + 10.,
    })
    .unwrap();
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

#[derive(Default)]
struct Glow;

impl Application for Glow {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        ui! {@styles(css);
            <div id="glow.page" class="glow-page">
                <div id="glow" class="glow"></div>
                <div id="glow.content" class="glow-content"></div>
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        true
    }
}

/// Un élément `absolute` d'une page défilante suit le contenu, y compris quand la vue est
/// reconstruite pendant le défilement : il n'est pas reposé en haut de la zone visible.
#[test]
fn an_absolute_child_keeps_scrolling_with_the_content_across_rebuilds() {
    let mut app = HeadlessApp::new(Glow).unwrap();
    let top = app.bounds("glow.page").unwrap().top;
    let at = |app: &HeadlessApp<Glow>, id: &str| app.bounds(id).unwrap().top - top;
    let mask = |app: &HeadlessApp<Glow>| {
        app.scene()
            .commands
            .iter()
            .find_map(|c| match c {
                DrawCommand::PushMask { bounds, .. } => Some(bounds.top - top),
                _ => None,
            })
            .unwrap()
    };
    app.scroll("glow.page", 0., 100.).unwrap();
    near(at(&app, "glow"), -100.);
    app.send(()).unwrap();
    near(at(&app, "glow.content"), -100.);
    near(at(&app, "glow"), -100.);
    near(mask(&app), -100.);
    app.scroll("glow.page", 0., -50.).unwrap();
    near(at(&app, "glow"), -50.);
    near(mask(&app), -50.);
}

#[derive(Default)]
struct Asked {
    request: Option<karbon_ui::ui::scroll::ScrollRequest>,
}

impl Application for Asked {
    type Message = Option<karbon_ui::ui::scroll::ScrollRequest>;
    fn view(&self, _: &ViewContext) -> Element {
        let request = self.request;
        ui! {@styles(css);
            <div id="asked" class="asked" style:scroll_request={request}>
                <div class="asked-content"></div>
            </div>
        }
    }
    fn update(&mut self, request: Self::Message) -> bool {
        self.request = request;
        true
    }
}

fn asked_offset(app: &HeadlessApp<Asked>) -> f32 {
    app.find("asked").unwrap().scroll_metrics().unwrap().offset
}

/// `style:scroll_request` : un nouveau jeton fait glisser le conteneur jusqu'à sa cible ;
/// la même demande reprise par une reconstruction laisse le défilement de l'utilisateur.
#[test]
fn a_scroll_request_glides_once_per_token() {
    use karbon_ui::ui::scroll::ScrollRequest;
    let mut app = HeadlessApp::new(Asked::default()).unwrap();
    app.send(Some(ScrollRequest {
        token: 1,
        offset: 900.,
        smooth: true,
    }))
    .unwrap();
    // Glissement en cours : pas encore arrivé.
    assert!(asked_offset(&app) < 900.);
    // Un glissement demandé dure 450 ms (plus qu'un cran de molette).
    app.advance(std::time::Duration::from_millis(200)).unwrap();
    assert!(asked_offset(&app) < 900.);
    app.advance(std::time::Duration::from_millis(300)).unwrap();
    near(asked_offset(&app), 900.);
    app.scroll("asked", 0., -300.).unwrap();
    app.send(Some(ScrollRequest {
        token: 1,
        offset: 900.,
        smooth: true,
    }))
    .unwrap();
    near(asked_offset(&app), 600.);
    // Sans glissement : d'un coup ; au-delà de la plage : borné.
    app.send(Some(ScrollRequest {
        token: 2,
        offset: 5000.,
        smooth: false,
    }))
    .unwrap();
    let max = app
        .find("asked")
        .unwrap()
        .scroll_metrics()
        .unwrap()
        .max_offset();
    near(asked_offset(&app), max);
}

/// Barre fine (trait de 4 px dans une gouttière de 10) : le pouce se saisit sur toute la
/// largeur de la gouttière, et le tirer fait défiler.
#[test]
fn a_thin_thumb_is_grabbed_across_the_whole_gutter() {
    use karbon_ui::ui::runtime::UiEvent;
    let mut app = HeadlessApp::new(Asked::default()).unwrap();
    let s = app.find("asked").unwrap().scroll_metrics().unwrap();
    assert_eq!(s.thumb.right - s.thumb.left, 4.);
    // Dans la gouttière, à droite du trait, à mi-hauteur du pouce.
    let x = s.track.right - 1.;
    let y = (s.thumb.top + s.thumb.bottom) / 2.;
    app.event(UiEvent::PointerMove(PointF { x, y })).unwrap();
    app.event(UiEvent::PointerDown(PointF { x, y })).unwrap();
    app.event(UiEvent::PointerMove(PointF { x, y: y + 50. }))
        .unwrap();
    app.event(UiEvent::PointerUp(PointF { x, y: y + 50. }))
        .unwrap();
    // 50 px de pouce : la même part de la plage de défilement.
    let travel = (s.track.bottom - s.track.top) - (s.thumb.bottom - s.thumb.top);
    near(asked_offset(&app), 50. / travel * s.max_offset());
}

/// Liste de 1000 lignes de 58 px dont 100 sont chargées ; des blocs vides tiennent la place
/// des autres. Quand la vue sort de la fenêtre, la fenêtre saute sous la vue.
struct Windowed {
    start: i64,
}
const WINDOW_ROW: f32 = 58.;
const WINDOW_TOTAL: i64 = 1000;

impl Application for Windowed {
    type Message = (f32, f32);
    fn view(&self, _: &ViewContext) -> Element {
        let start = self.start;
        let before = start as f32 * WINDOW_ROW;
        let after = (WINDOW_TOTAL - start - 100) as f32 * WINDOW_ROW;
        let rows: Vec<u64> = (start as u64..start as u64 + 100).collect();
        ui! {@styles(css);
            <div id="window" class="window-zone" onscroll={|s: karbon_ui::ui::scroll::ScrollMetrics| (s.offset, s.viewport_size())}>
                <div id={ElementId::named("window.before").item(start as u64)} class="window-spacer" style:height={before}></div>
                <div class="window-rows">
                    {#each rows as r (*r)}
                        <div id={ElementId::named("window.row").item(*r)} class="window-row"></div>
                    {/each}
                </div>
                <div id={ElementId::named("window.after").item(start as u64)} class="window-spacer" style:height={after}></div>
            </div>
        }
    }
    fn update(&mut self, (offset, view): (f32, f32)) -> bool {
        let first = (((offset - 6.) / WINDOW_ROW).floor() as i64).clamp(0, WINDOW_TOTAL);
        let last = (((offset + view - 6.) / WINDOW_ROW).ceil() as i64).clamp(0, WINDOW_TOTAL);
        if last <= self.start || first >= self.start + 100 {
            self.start = (first - 20).clamp(0, WINDOW_TOTAL - 100);
            return true;
        }
        false
    }
}

/// Les blocs vides changent d'identifiant avec la fenêtre : l'ancrage du défilement ne
/// s'y accroche pas (il compenserait leur nouvelle hauteur et la vue retomberait dans le
/// vide). Après chaque saut, la ligne attendue est à sa place, dans les deux sens.
#[test]
fn a_windowed_list_with_spacers_keeps_the_rows_under_the_view() {
    use karbon_ui::ui::element::ElementId;
    let mut app = HeadlessApp::new(Windowed { start: 0 }).unwrap();
    let top = app.bounds("window").unwrap().top;
    for step in [3000., 3000., 5000., -2000., 20000., -25000.] {
        app.scroll("window", 0., step).unwrap();
        let offset = app.find("window").unwrap().scroll_metrics().unwrap().offset;
        let first = ((offset - 6.) / WINDOW_ROW).floor().max(0.) as u64;
        let row = app
            .bounds(ElementId::named("window.row").item(first))
            .expect("ligne chargée sous la vue");
        near(row.top - top, 6. + first as f32 * WINDOW_ROW - offset);
    }
}

#[derive(Default)]
struct Revealed {
    request: Option<karbon_ui::ui::scroll::ScrollRequest>,
}

impl Application for Revealed {
    type Message = Option<karbon_ui::ui::scroll::ScrollRequest>;
    fn view(&self, _: &ViewContext) -> Element {
        let request = self.request;
        ui! {@styles(css);
            <div id="cv.zone" class="cv-zone" style:scroll_request={request}>
                {#each (0..200u64) as i (i)}
                    <div id={karbon_ui::ui::element::ElementId::named("cv.row").item(i)} class="cv-row"><div class="cv-mark"></div></div>
                {/each}
            </div>
        }
    }
    fn update(&mut self, request: Self::Message) -> bool {
        self.request = request;
        true
    }
}

/// Un défilement demandé loin dans une liste de lignes `content-visibility: auto` : les
/// lignes arrivées sous la vue sont rendues dans la même image, sans attendre la suivante.
#[test]
fn a_scroll_request_reveals_the_skipped_rows_it_brings_into_view() {
    use karbon_ui::ui::{element::ElementId, scroll::ScrollRequest};
    let mut app = HeadlessApp::new(Revealed::default()).unwrap();
    let far = ElementId::named("cv.row").item(150);
    assert_eq!(
        app.find(far).unwrap().children[0].bounds().right
            - app.find(far).unwrap().children[0].bounds().left,
        0.
    );
    app.send(Some(ScrollRequest {
        token: 1,
        offset: 150. * 40.,
        smooth: false,
    }))
    .unwrap();
    let mark = app.find(far).unwrap().children[0].bounds();
    near(mark.right - mark.left, 30.);
}

#[derive(Default)]
struct Targeted {
    shown: bool,
}

impl Application for Targeted {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let shown = self.shown;
        ui! {@styles(css);
            <div>
                {#if shown}
                    <div id="cv.targeted" class="cv-zone">
                        {#each (0..200u64) as i (i)}
                            <div id={karbon_ui::ui::element::ElementId::named("cv.target").item(i)} class="cv-row" style:scroll_initial_target={i == 150}><div class="cv-mark"></div></div>
                        {/each}
                    </div>
                {/if}
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        self.shown = true;
        true
    }
}

/// Une liste qui apparaît sur sa cible initiale, loin dans des lignes
/// `content-visibility: auto` : la cible est rendue dès la première image.
#[test]
fn an_initial_scroll_target_reveals_the_skipped_rows_around_it() {
    use karbon_ui::ui::element::ElementId;
    let mut app = HeadlessApp::new(Targeted::default()).unwrap();
    app.send(()).unwrap();
    let offset = app
        .find("cv.targeted")
        .unwrap()
        .scroll_metrics()
        .unwrap()
        .offset;
    near(offset, 150. * 40.);
    let mark = app
        .find(ElementId::named("cv.target").item(150))
        .unwrap()
        .children[0]
        .bounds();
    near(mark.right - mark.left, 30.);
}

#[derive(Default)]
struct TargetedCards {
    shown: bool,
}

impl Application for TargetedCards {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        let shown = self.shown;
        ui! {@styles(css);
            <div>
                {#if shown}
                    <div id="cv.cards" class="cv-zone">
                        {#each (0..200u64) as i (i)}
                            <div id={karbon_ui::ui::element::ElementId::named("cv.card").item(i)} class="cv-card" style:scroll_initial_target={i == 150}><div class="cv-card-body"></div></div>
                        {/each}
                    </div>
                {/if}
            </div>
        }
    }
    fn update(&mut self, _: ()) -> bool {
        self.shown = true;
        true
    }
}

/// Une cible initiale parmi des cartes plus hautes que leur taille provisoire : une fois
/// rendues, elles poussent la suite ; la cible reste en haut et tout ce qui est sous la
/// vue est rendu.
#[test]
fn an_initial_target_among_taller_cards_stays_on_top_with_the_view_rendered() {
    use karbon_ui::ui::element::ElementId;
    let mut app = HeadlessApp::new(TargetedCards::default()).unwrap();
    app.send(()).unwrap();
    let view = app
        .find("cv.cards")
        .unwrap()
        .scroll_metrics()
        .unwrap()
        .viewport;
    let target = app.bounds(ElementId::named("cv.card").item(150)).unwrap();
    near(target.top, view.top);
    for i in 0..200u64 {
        let card = app.find(ElementId::named("cv.card").item(i)).unwrap();
        let b = card.bounds();
        if b.bottom > view.top && b.top < view.bottom {
            let body = card.children[0].bounds();
            assert!(
                (body.bottom - body.top - 70.).abs() < 0.5,
                "carte {i} sous la vue non rendue : {body:?}"
            );
        }
    }
}

struct Lyrics {
    active: u64,
}

impl Application for Lyrics {
    type Message = u64;
    fn view(&self, _: &ViewContext) -> Element {
        let active = self.active;
        ui! {@styles(css);
            <div>
                <div id="iv.zone" class="iv-zone">
                    {#each (0..100u64) as i (i)}
                        <div class="iv-row" style:scroll_into_view={i == active}></div>
                    {/each}
                </div>
            </div>
        }
    }
    fn update(&mut self, active: u64) -> bool {
        self.active = active;
        true
    }
}

/// La ligne active : sa zone part dessus, puis glisse vers chaque nouvelle ligne active,
/// y compris une ligne déjà visitée ; `scroll-margin-top` la garde sous le haut de la vue.
#[test]
fn the_active_line_is_brought_into_view_and_glides_to_the_next_one() {
    use std::time::Duration;
    let offset = |app: &HeadlessApp<Lyrics>| {
        app.find("iv.zone")
            .unwrap()
            .scroll_metrics()
            .unwrap()
            .offset
    };
    let mut app = HeadlessApp::new(Lyrics { active: 10 }).unwrap();
    near(offset(&app), 10. * 40. - 80.);

    app.send(20).unwrap();
    // Le glissement part de l'ancienne position…
    assert!(offset(&app) < 20. * 40. - 80.);
    app.advance(Duration::from_millis(200)).unwrap();
    let midway = offset(&app);
    assert!(
        midway > 10. * 40. - 80. && midway < 20. * 40. - 80.,
        "{midway}"
    );
    // … et s'arrête sur la nouvelle ligne.
    app.advance(Duration::from_millis(400)).unwrap();
    near(offset(&app), 20. * 40. - 80.);

    // Retour à une ligne déjà active plus tôt : elle est amenée de nouveau.
    app.send(10).unwrap();
    app.advance(Duration::from_millis(600)).unwrap();
    near(offset(&app), 10. * 40. - 80.);

    // Reconstruire sans changer de ligne laisse le défilement de l'utilisateur.
    app.update_app(|_| {}).unwrap();
    near(offset(&app), 10. * 40. - 80.);
}
