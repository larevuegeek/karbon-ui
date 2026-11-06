//! Syntaxe HTML des templates : texte libre, blocs, balises natives, attributs
//! communs, feuille par défaut et animations `data-*`.
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::prelude::*,
    ui::{
        drag_drop::DropEvent,
        element::{Layout, Role},
        runtime::Key,
        scene::DrawCommand,
    },
};

use std::{rc::Rc, time::Duration};

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Name(String),
    Saved,
    Notify(bool),
    Volume(f64),
    Theme(u64),
    Size(u8),
    Open(u64),
    Close,
}

css_file!(mod css, "html.css");

#[derive(Default)]
struct Page {
    name: String,
    notify: bool,
    volume: f64,
    theme: Option<u64>,
    size: u8,
    dialog: bool,
    tracks: Vec<(u64, &'static str)>,
    log: Vec<Msg>,
}

impl Application for Page {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui_file!("ui/page.ui", context = cx, styles = css)
    }
    fn update(&mut self, message: Msg) -> bool {
        match &message {
            Msg::Name(n) => self.name = n.clone(),
            Msg::Notify(v) => self.notify = *v,
            Msg::Volume(v) => self.volume = *v,
            Msg::Theme(t) => self.theme = Some(*t),
            Msg::Size(s) => self.size = *s,
            Msg::Close => self.dialog = false,
            _ => {}
        }
        self.log.push(message);
        true
    }
}

fn page() -> HeadlessApp<Page> {
    HeadlessApp::new(Page {
        tracks: vec![(1, "Intro"), (2, "Outro")],
        ..Page::default()
    })
    .unwrap()
}

#[test]
fn plain_text_entities_and_inline_runs_build_one_rich_paragraph() {
    let app = page();
    let intro = app.accessible("intro").unwrap();
    assert_eq!(intro.role, Role::Text);
    assert_eq!(intro.name, "Hello you & me — see the guide.");
    let link = app
        .accessibility()
        .into_iter()
        .find(|n| n.role == Role::Link)
        .unwrap();
    assert_eq!(link.name, "guide");
    assert!(app.scene().commands.iter().any(|c| matches!(
        c,
        DrawCommand::RichText { paragraph, .. } if &*paragraph.text == "Hello you & me — see the guide."
    )));
    let heading = app.accessible("title").unwrap();
    assert_eq!(heading.role, Role::Heading);
}

#[test]
fn links_and_controls_route_typed_messages() {
    let mut app = page();
    let link = app
        .accessibility()
        .into_iter()
        .find(|n| n.role == Role::Link)
        .unwrap();
    app.click(link.id).unwrap();
    assert_eq!(app.app().log, [Msg::Open(7)]);
    app.focus("name").unwrap();
    app.type_text("Ada").unwrap();
    assert_eq!(app.app().name, "Ada");
    app.click("notify").unwrap();
    assert!(app.app().notify);
    assert_eq!(
        app.accessible("notify").unwrap().name,
        "Notifications",
        "the text following a checkbox is its label"
    );
    assert_eq!(app.accessible("name").unwrap().name, "Your name");
    app.click("save").unwrap();
    assert!(app.app().log.contains(&Msg::Saved));
}

#[test]
fn blocks_lists_and_semantic_containers() {
    let mut app = page();
    let list = app.find("tracks").unwrap();
    assert_eq!(list.children.len(), 2);
    assert_eq!(app.accessible("tracks").unwrap().role, Role::List);
    assert!(app.contains_text("Intro"));
    assert!(app.contains_text("No dialog"));
    app.update_app(|p| p.dialog = true).unwrap();
    // Le dialogue déclaré dans la page est remonté au-dessus d'elle.
    let root = app.root().unwrap();
    assert!(matches!(root.layout, Layout::Layers));
    assert_eq!(app.accessible("settings").unwrap().role, Role::Dialog);
    app.update_app(|p| p.tracks.reverse()).unwrap();
}

#[test]
fn default_stylesheet_and_inline_styles_apply_before_overrides() {
    let app = page();
    let row = app.find("row").unwrap();
    assert!(matches!(row.layout, Layout::Row));
    let scroll = app.find("scroller").unwrap();
    assert!(matches!(scroll.layout, Layout::Scroll(_)));
    let p = app.bounds("intro").unwrap();
    let next = app.bounds("after-intro").unwrap();
    // Marge basse par défaut d'un <p> : 12 px.
    assert_eq!(next.top - p.bottom, 12.);
}

#[derive(Default)]
struct Login {
    secret: String,
}

impl Application for Login {
    type Message = String;
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            <form>
                <input id="secret" type="password" value={self.secret.as_str()}
                       oninput={|v| v} aria-label="Mot de passe">
            </form>
        }
    }
    fn update(&mut self, value: String) -> bool {
        self.secret = value;
        true
    }
}

#[test]
fn password_fields_mask_display_accessibility_and_clipboard() {
    let mut app = HeadlessApp::new(Login::default()).unwrap();
    app.focus("secret").unwrap();
    app.type_text("abé").unwrap();
    assert_eq!(app.app().secret, "abé");
    assert!(app.contains_text("•••") && !app.contains_text("abé"));
    let node = app.accessible("secret").unwrap();
    assert_eq!(node.value.as_deref(), Some("•••"));
    assert!(node.text.is_none());
    app.edit(karbon_ui::ui::text_input::EditCommand::SelectAll)
        .unwrap();
    app.edit(karbon_ui::ui::text_input::EditCommand::Copy)
        .unwrap();
    assert_eq!(app.clipboard(), None);
    // Les positions restent celles du texte réel : effacer retire un caractère entier.
    app.press(Key::End).unwrap();
    app.press(Key::Backspace).unwrap();
    assert_eq!(app.app().secret, "ab");
}

/// Animations `data-*` : sortie (fantôme non interactif), décalage et rejeu.
#[derive(Default)]
struct Animated {
    items: Vec<u64>,
    replay: u32,
}

impl Application for Animated {
    type Message = u64;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! {@context(cx);@styles(css);
            <ul id="list" data-stagger="100ms" style="padding:0">
                {#each self.items as item (*item)}
                    <li id={ElementId::named("item").item(*item)} class="track"
                        data-exit="slide-out-left 200ms linear" data-replay={self.replay}
                        onclick={*item}>Item {item}</li>
                {/each}
            </ul>
        }
    }
    fn update(&mut self, item: u64) -> bool {
        self.items.retain(|i| *i != item);
        true
    }
}

fn opacity_of(app: &HeadlessApp<Animated>, text: &str) -> Option<f32> {
    let mut stack = Vec::new();
    for command in &app.scene().commands {
        match command {
            DrawCommand::PushEffect { opacity, .. } => stack.push(*opacity),
            DrawCommand::PopEffect => {
                stack.pop();
            }
            DrawCommand::RichText { paragraph, .. } if &*paragraph.text == text => {
                return Some(stack.iter().product());
            }
            _ => {}
        }
    }
    None
}

#[test]
fn exit_animations_keep_a_fading_ghost_then_drop_it() {
    let mut app = HeadlessApp::new(Animated {
        items: vec![1, 2, 3],
        replay: 0,
    })
    .unwrap();
    app.advance(Duration::from_millis(1000)).unwrap();
    assert!(app.contains_text("Item 2"));
    let item = ElementId::named("item").item(2);
    app.click(item).unwrap();
    // L'élément a quitté l'arbre : plus de cible, mais un fantôme qui s'efface.
    assert!(app.find(item).is_none());
    assert!(app.contains_text("Item 2"));
    app.advance(Duration::from_millis(100)).unwrap();
    let faded = opacity_of(&app, "Item 2").unwrap();
    assert!(faded > 0.3 && faded < 0.7, "{faded}");
    assert!(app.runtime().next_wake().is_some());
    app.advance(Duration::from_millis(150)).unwrap();
    assert!(!app.contains_text("Item 2"));
    assert!(app.contains_text("Item 3"));
}

#[test]
fn stagger_delays_each_child_and_replay_restarts_the_entry() {
    let mut app = HeadlessApp::new(Animated {
        items: vec![1, 2, 3],
        replay: 0,
    })
    .unwrap();
    // .track : apparition de 200 ms, décalée de 100 ms par enfant.
    app.advance(Duration::from_millis(150)).unwrap();
    let first = opacity_of(&app, "Item 1").unwrap();
    let third = opacity_of(&app, "Item 3").unwrap();
    assert!(first > 0.5, "{first}");
    assert_eq!(third, 0.);
    app.advance(Duration::from_millis(600)).unwrap();
    assert_eq!(opacity_of(&app, "Item 3").unwrap(), 1.);
    app.update_app(|a| a.replay = 1).unwrap();
    app.advance(Duration::from_millis(10)).unwrap();
    assert!(opacity_of(&app, "Item 1").unwrap() < 0.5);
}

#[test]
fn reduced_motion_drops_exit_ghosts_immediately() {
    let mut app = HeadlessApp::with_options(
        Animated {
            items: vec![1],
            replay: 0,
        },
        HeadlessOptions {
            reduced_motion: true,
            ..Default::default()
        },
    )
    .unwrap();
    app.click(ElementId::named("item").item(1)).unwrap();
    assert!(!app.contains_text("Item 1"));
}

#[derive(Debug, Clone, PartialEq)]
enum Ev {
    Open,
    Close,
    Pick(usize),
    Submit,
    Field(String),
    Menu,
    Dropped(u64, u64),
}

#[derive(Default)]
struct Widgets {
    open: bool,
    rows: Rc<Vec<String>>,
    log: Vec<Ev>,
}

impl Application for Widgets {
    type Message = Ev;
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            <div style="padding:8px; gap:6px">
                <k-split id="split" axis="horizontal" fraction="0.25" min-first="40" style="height:60px">
                    <nav id="side"><span>Menu</span></nav>
                    <section id="main" aria-label="Contenu">
                        <button id="anchor" title="Ouvre le menu" onclick={Ev::Open}>Ouvrir</button>
                    </section>
                </k-split>
                <form id="form" onsubmit={Ev::Submit}>
                    <input id="field" type="text" value="" oninput={Ev::Field}>
                    <button id="submit" type="submit">Envoyer</button>
                </form>
                <ol id="numbers">
                    {#each ["a", "b"] as (index, letter) (index)}
                        <li onclick={Ev::Pick(index)}>{index}: {letter}</li>
                    {:else}
                        <li>Vide</li>
                    {/each}
                </ol>
                <div id="drop" draggable={(1, 7)} ondrop={|e: DropEvent| Ev::Dropped(e.source, e.target)}
                     oncontextmenu={|_| Ev::Menu} tabindex="-1">Glisser</div>
                <k-virtual-list id="rows" items={Rc::clone(&self.rows)} item="row" row-height="20" style="height:60px">
                    <p style="margin:0">{row}</p>
                </k-virtual-list>
                <hr id="rule">
                {#if self.open}
                    <k-popover anchor="anchor" aria-label="Menu" onclose={Ev::Close}>
                        <button id="close" onclick={Ev::Close}>Fermer</button>
                    </k-popover>
                {/if}
            </div>
        }
    }
    fn update(&mut self, message: Ev) -> bool {
        match message {
            Ev::Open => self.open = true,
            Ev::Close => self.open = false,
            _ => {}
        }
        self.log.push(message);
        true
    }
}

#[test]
fn engine_elements_forms_lists_and_common_attributes() {
    let mut app = HeadlessApp::new(Widgets {
        rows: Rc::new((0..100).map(|i| format!("Ligne {i}")).collect()),
        ..Default::default()
    })
    .unwrap();
    assert!(matches!(
        app.find("split").unwrap().layout,
        Layout::Split(_)
    ));
    assert_eq!(app.accessible("side").unwrap().role, Role::Navigation);
    let main = app.accessible("main").unwrap();
    assert_eq!((main.role, main.name.as_str()), (Role::Group, "Contenu"));
    assert_eq!(app.accessible("numbers").unwrap().role, Role::List);
    assert!(app.contains_text("0: a") && app.contains_text("1: b"));
    assert!(!app.contains_text("Vide"));
    assert_eq!(app.accessible("rule").unwrap().role, Role::Separator);
    // Liste virtualisée : seules les lignes visibles sont construites.
    assert!(app.contains_text("Ligne 0") && !app.contains_text("Ligne 50"));
    // Formulaire : Entrée dans le champ et bouton submit envoient onsubmit.
    app.focus("field").unwrap();
    app.type_text("x").unwrap();
    app.press(Key::Enter).unwrap();
    app.click("submit").unwrap();
    assert_eq!(
        app.app().log,
        [Ev::Field("x".into()), Ev::Submit, Ev::Submit]
    );
    // tabindex="-1" : hors de la navigation clavier.
    assert!(!app.accessible("drop").is_some_and(|n| n.focusable));
    // Popover ancré, remonté au-dessus de la page.
    app.click("anchor").unwrap();
    assert!(matches!(app.root().unwrap().layout, Layout::Layers));
    assert_eq!(app.focused(), Some(ElementId::named("close")));
    app.click("close").unwrap();
    assert!(!matches!(app.root().unwrap().layout, Layout::Layers));
}
