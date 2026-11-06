//! Headless checks: the page builds and lays out at the reference size and at the
//! minimum window size, shows the key texts, and the few local states react.
use super::*;
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::scene::Image,
    window::WindowCommand,
};

fn app(width: f32, height: f32) -> HeadlessApp<MusicApp> {
    HeadlessApp::with_options(
        MusicApp::default(),
        HeadlessOptions {
            width,
            height,
            reduced_motion: true,
            ..Default::default()
        },
    )
    .expect("the page builds, validates and lays out")
}

#[test]
fn page_builds_at_reference_and_minimum_sizes() {
    for (width, height) in [(1440., 1040.), (1100., 760.)] {
        let app = app(width, height);
        let root = app.root().expect("a laid out tree");
        karbon_ui::ui::layout::validate(root).expect("valid tree");
        for text in [
            "Bonsoir, David",
            "Voici de quoi reprendre, et quelques albums à redécouvrir.",
            "Stairway To Heaven",
            "EN PAUSE",
        ] {
            assert!(app.contains_text(text), "{text} at {width}×{height}");
        }
        if width == 1440. {
            // The mixes are in the first screen at the reference size only.
            assert!(app.contains_text("The Beatles, Ed Sheeran, Britney Spears et d'autres"));
        }
        // Fixed regions: title bar on top, player at the bottom, page in between.
        let close = app.bounds("window.close").expect("window controls");
        assert!(close.right <= width + 0.5 && close.top >= 0.);
        let play = app.bounds("resume.play").expect("resume hero");
        assert!(play.top > 60. && play.bottom < height);
        let volume = app.bounds("player.volume").expect("player volume");
        assert!(volume.bottom <= height + 0.5 && volume.top > height - 150.);
    }
}

#[test]
fn local_states_follow_the_svelte_toggles() {
    let mut app = app(1440., 1040.);
    app.click("resume.play").unwrap();
    assert!(app.app().playing);
    assert!(app.contains_text("EN COURS DE LECTURE"));
    app.click("sidebar.footer.tab").unwrap();
    assert!(!app.app().footer_collapsed);
    app.send(Msg::Volume(20.)).unwrap();
    assert_eq!(app.app().volume, 20.);
    app.click("window.close").unwrap();
    assert!(matches!(
        app.take_window_commands().as_slice(),
        [WindowCommand::Action(
            _,
            karbon_ui::window::WindowAction::Close
        )]
    ));
}

/// Every menu of the captures builds, lays out and closes; their choices update the page.
#[test]
fn menus_open_close_and_update_the_page() {
    let mut app = app(1280., 900.);
    for (name, menu) in Menu::NAMES {
        app.send(Msg::Menu(menu)).unwrap();
        assert_eq!(app.app().menu, Some(menu), "{name}");
        app.send(Msg::CloseMenu).unwrap();
        assert_eq!(app.app().menu, None, "{name}");
    }
    // Source et bibliothèque : le bouton suit le choix.
    app.click("sidebar.source").unwrap();
    assert_eq!(app.app().menu, Some(Menu::Source));
    app.send(Msg::ChoisirSource(2)).unwrap();
    assert!(app.contains_text("Toutes les sources") && app.app().menu.is_none());
    app.click("sidebar.bibliotheque").unwrap();
    app.send(Msg::ChoisirBibliotheque(1)).unwrap();
    assert!(app.contains_text("451 titres · 4 biblio."));
    // Sections affichées : une tuile apparaît dans la navigation.
    assert!(!app.contains_text("Années"));
    app.send(Msg::BasculerOnglet(4)).unwrap();
    assert!(app.contains_text("Années"));
    // Minuteur : le bouton affiche le temps restant.
    app.send(Msg::Veille(Some(Veille::Minutes(30)))).unwrap();
    assert!(app.contains_text("30:00"));
    // Sortie audio : la pastille suit la sortie choisie.
    app.send(Msg::ChoisirSortie(3)).unwrap();
    assert!(app.contains_text("Realtek Digital Output"));
}

/// Search suggestions while typing: the field keeps the focus, Up/Down highlight.
#[test]
fn search_suggestions_follow_the_typed_query() {
    let mut app = app(1280., 900.);
    app.send(Msg::Search("q".into())).unwrap();
    assert_eq!(app.app().menu, None);
    app.send(Msg::Search("que".into())).unwrap();
    assert_eq!(app.app().menu, Some(Menu::Recherche));
    assert_eq!(app.app().resultats().len(), 12);
    assert!(app.contains_text("Que du love") && app.contains_text("Kings and Queens"));
    app.send(Msg::SearchKey(Key::Down)).unwrap();
    app.send(Msg::SearchKey(Key::Down)).unwrap();
    assert_eq!(app.app().suggestion, Some(1));
    app.send(Msg::Search("velazquez".into())).unwrap();
    assert_eq!(app.app().resultats().len(), 1, "accents ignored");
    app.send(Msg::SearchPick(0)).unwrap();
    assert_eq!(app.app().menu, None);
}

fn width(r: karbon_ui::ui::scene::RectF) -> f32 {
    r.right - r.left
}

/// The title bar button collapses the sidebar to its 76 px icon column (`w-19`) and
/// expands it back to 288 px (`w-72`); the main area takes the freed width.
#[test]
fn title_bar_button_collapses_and_expands_the_sidebar() {
    for (w, h) in [(1280., 900.), (960., 700.)] {
        let mut app = app(w, h);
        let expanded = app.bounds("sidebar").unwrap();
        let main = app.bounds("app.main").unwrap();
        assert_eq!(width(expanded), 288.);
        app.click("titlebar.sidebar").unwrap();
        assert!(app.app().sidebar_collapsed);
        let collapsed = app.bounds("sidebar").unwrap();
        assert_eq!(width(collapsed), 76., "{w}×{h}");
        let wide = app.bounds("app.main").unwrap();
        assert!((width(wide) - width(main) - 212.).abs() < 0.5, "{w}×{h}");
        assert_eq!(wide.right, main.right);
        // Tout reste dans la colonne : pas de débordement horizontal.
        for id in [
            "sidebar.source",
            "sidebar.bibliotheque",
            "sidebar.footer.profil",
        ] {
            let r = app.bounds(id).unwrap_or_else(|| panic!("{id}"));
            assert!(
                r.left >= collapsed.left && r.right <= collapsed.right,
                "{id} {r:?}"
            );
            // Centrés : même marge des deux côtés (bordure droite de 1 px comprise). À
            // 700 px de haut la colonne déborde et défile : la gouttière de 8 px de la
            // barre (smart-scroll) décale le centre, comme dans le Svelte.
            let (gauche, droite) = (r.left - collapsed.left, collapsed.right - 1. - r.right);
            let gouttiere = if h < 900. { 8. } else { 0. };
            assert!(
                (gauche - (droite - gouttiere)).abs() <= 0.5,
                "{id} {gauche} {droite}"
            );
        }
        assert_eq!(width(app.bounds("sidebar.source").unwrap()), 52.);
        assert_eq!(width(app.bounds("sidebar.footer.rescan").unwrap()), 48.);
        app.click("titlebar.sidebar").unwrap();
        assert!(!app.app().sidebar_collapsed);
        assert_eq!(width(app.bounds("sidebar").unwrap()), 288.);
        assert_eq!(app.bounds("app.main").unwrap(), main);
    }
}

/// Collapsed variants: icons only (titles as tooltips), no section titles, no
/// Fichier / Dossier, the footer column instead of the card.
#[test]
fn collapsed_sidebar_keeps_icons_and_hides_texts() {
    let mut app = app(1280., 900.);
    let expanded = [
        "Cet ordinateur",
        "Fichier",
        "Dossier",
        "Accueil",
        "Titres likés",
        "Podcasts",
        data::SCAN,
    ];
    for text in expanded {
        assert!(app.contains_text(text), "{text}");
    }
    assert!(app.find("sidebar.footer.tab").is_some());
    app.send(Msg::ToggleSidebar).unwrap();
    for text in expanded {
        assert!(!app.contains_text(text), "{text} still shown");
    }
    assert!(app.find("sidebar.footer.tab").is_none());
    assert!(app.find("sidebar.ouvrir").is_none());
    assert!(app.find("sidebar.playlists.options").is_none());
    // Noms accessibles des boutons réduits à leur icône (leur `title` en infobulle).
    let names = app
        .accessibility()
        .into_iter()
        .map(|e| e.name)
        .collect::<Vec<_>>();
    for title in [
        "Déplier la barre",
        "Changer de source",
        "Accueil",
        "Morceaux",
        "Changer de bibliothèque",
        "Titres likés",
        "Podcasts",
        "Radios",
        "Dossiers synchronisés",
        "David — Profil actif · changer",
    ] {
        assert!(names.iter().any(|t| t == title), "{title}: {names:?}");
    }
    assert!(!names.iter().any(|t| t == "Replier la barre"));
    // L'infobulle d'une tuile réduite à son icône paraît après le délai de survol.
    let source = app.bounds("sidebar.source").unwrap();
    app.hover("sidebar.source").unwrap();
    // Le délai part de l'horloge réelle du runtime : avancer l'horloge virtuelle jusque-là.
    let delai = app.runtime().next_wake().expect("tooltip delay");
    app.advance(delai.saturating_duration_since(app.now()) + std::time::Duration::from_millis(1))
        .unwrap();
    let tooltip = app
        .runtime()
        .tooltip()
        .expect("tooltip of the source button");
    assert_eq!(&*tooltip.text, "Changer de source");
    // pt-3 : la colonne commence 12 px sous le haut de la barre.
    let sidebar = app.bounds("sidebar").unwrap();
    assert_eq!(source.top - sidebar.top, 12.);
}

/// Source and library menus still open while collapsed, to the right of their button
/// (`left: r.right + 8; top: r.top`), 300 and 272 px wide; choices still apply.
#[test]
fn sidebar_menus_open_beside_the_collapsed_column() {
    let mut app = app(1280., 900.);
    app.send(Msg::ToggleSidebar).unwrap();
    app.click("sidebar.source").unwrap();
    assert_eq!(app.app().menu, Some(Menu::Source));
    let button = app.bounds("sidebar.source").unwrap();
    let menu = app
        .bounds("sidebar.source.menu")
        .expect("source menu beside the button");
    assert!(
        (menu.left - (button.right + 8.)).abs() < 0.5,
        "{menu:?} {button:?}"
    );
    assert!((menu.top - button.top).abs() < 0.5, "{menu:?} {button:?}");
    assert_eq!(width(menu), 300.);
    assert!(app.contains_text("Toutes les sources"));
    app.send(Msg::ChoisirSource(2)).unwrap();
    assert_eq!((app.app().source, app.app().menu), (2, None));

    app.click("sidebar.bibliotheque").unwrap();
    assert_eq!(app.app().menu, Some(Menu::Bibliotheque));
    let button = app.bounds("sidebar.bibliotheque").unwrap();
    let menu = app
        .bounds("sidebar.bibliotheque.menu")
        .expect("library menu");
    assert!(
        (menu.left - (button.right + 8.)).abs() < 0.5,
        "{menu:?} {button:?}"
    );
    assert!((menu.top - button.top).abs() < 0.5, "{menu:?} {button:?}");
    assert_eq!(width(menu), 272.);
    app.send(Msg::ChoisirBibliotheque(1)).unwrap();
    assert_eq!(app.app().menu, None);

    // Le « … » de Fichier / Dossier n'existe plus : son menu se ferme avec le repli.
    app.send(Msg::ToggleSidebar).unwrap();
    app.send(Msg::Menu(Menu::Ouvrir)).unwrap();
    app.send(Msg::ToggleSidebar).unwrap();
    assert_eq!(app.app().menu, None);
}

/// Without reduced motion the width follows `transition: width 200ms`, frame by frame,
/// and the main area follows it.
#[test]
fn sidebar_width_is_animated_over_200_ms() {
    let mut app = HeadlessApp::with_options(
        MusicApp::default(),
        HeadlessOptions {
            width: 1280.,
            height: 900.,
            reduced_motion: false,
            ..Default::default()
        },
    )
    .unwrap();
    app.send(Msg::ToggleSidebar).unwrap();
    app.advance(std::time::Duration::from_millis(100)).unwrap();
    let mid = width(app.bounds("sidebar").unwrap());
    assert!(mid > 76. && mid < 288., "{mid}");
    let main = width(app.bounds("app.main").unwrap());
    assert!((main + mid - 1280.).abs() < 0.5, "{main} {mid}");
    app.advance(std::time::Duration::from_millis(150)).unwrap();
    assert_eq!(width(app.bounds("sidebar").unwrap()), 76.);
}

struct Gray;
impl karbon_ui::application::Resources for Gray {
    fn decode_image(&self, _: &[u8]) -> karbon_ui::Result<Arc<Image>> {
        Ok(Arc::new(Image::new(1, 1, vec![128; 4])?))
    }
}

/// Every Iconify name written in the templates has its generated SVG (tools/icons.py),
/// and the sidebar tiles their `.sb-icone` variant.
#[test]
fn templates_only_use_generated_icons() {
    let assets = Assets::load(&Gray).unwrap();
    let cx = TemplateContext::new(&assets, "icons");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/rustmusic_html");
    let mut files = vec![dir.join("mod.rs")];
    let mut stack = vec![dir.join("ui"), dir.join("data")];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    let mut checked = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for quoted in text.split('"').skip(1).step_by(2) {
            if [
                "material-symbols:",
                "material-symbols-light:",
                "mynaui:",
                "ph:",
                "lucide:",
                "tabler:",
                "heroicons:",
            ]
            .iter()
            .any(|p| quoted.starts_with(p))
            {
                assert!(
                    icons::svg(&cx, quoted, false).is_some(),
                    "{quoted} in {file:?}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 50, "{checked}");
    for tuile in std::iter::once(&data::ACCUEIL).chain(&data::ONGLETS) {
        assert!(icons::svg(&cx, tuile.2, true).is_some(), "{}", tuile.2);
    }
}

#[test]
fn colour_helpers_match_the_css_formulas() {
    // hue.ts on the lower-cased genre names of the two daily mixes.
    assert_eq!(hue("pop"), 305.);
    assert_eq!(hue("rock"), 341.);
    // oklch(0.8 0.12 10): the player accent of a pink cover (--lh 10, --ls 1).
    let acc = oklch(0.8, 0.12, 10., 1.);
    assert_eq!(acc, Color::rgba(0xff, 0x9c, 0xac, 255));
    assert_eq!(hex("#22c55e"), Color::hex(0x22c55e));
}

/// Engine caches (measures of a layout pass, text measures kept between frames): the
/// same interactions give the same laid out tree, texts, accessibility tree and pixels
/// with and without them.
#[test]
fn engine_caches_match_a_full_computation() {
    use karbon_ui::ui::{element::Element, runtime::UiEvent, scene::PointF};
    use std::time::Duration;
    fn snapshot(app: &mut HeadlessApp<MusicApp>, pixels: bool) -> Vec<String> {
        fn walk(node: &Element, out: &mut Vec<String>) {
            let b = node.bounds();
            let bits = [b.left, b.top, b.right, b.bottom].map(f32::to_bits);
            out.push(format!("{:?} {bits:?}", node.id));
            node.children.iter().for_each(|c| walk(c, out));
        }
        let mut out = Vec::new();
        walk(app.root().unwrap(), &mut out);
        out.extend(app.texts().iter().map(|t| t.to_string()));
        out.extend(app.accessibility().iter().map(|e| {
            let r = (e.bounds, e.clipped_bounds);
            format!("{:?} {} {r:?} {:?}", e.id, e.name, e.value)
        }));
        if pixels {
            out.push(format!("{:?}", app.render(1.).unwrap().pixels));
        }
        out
    }
    fn play(incremental: bool) -> Vec<Vec<String>> {
        let options = HeadlessOptions {
            width: 1440.,
            height: 1040.,
            ..Default::default()
        };
        let mut app = HeadlessApp::with_options(MusicApp::default(), options).unwrap();
        app.runtime_mut().set_incremental(incremental);
        app.run_tasks().unwrap();
        let frame = Duration::from_millis(16);
        let mut steps = vec![snapshot(&mut app, true)];
        for (_, menu) in Menu::NAMES.into_iter().take(5) {
            app.send(Msg::Menu(menu)).unwrap();
            app.advance(frame).unwrap();
            steps.push(snapshot(&mut app, false));
            app.send(Msg::CloseMenu).unwrap();
        }
        for _ in 0..2 {
            app.send(Msg::ToggleSidebar).unwrap();
            for i in 0..16 {
                app.advance(frame).unwrap();
                steps.push(snapshot(&mut app, i == 7));
            }
        }
        for i in 0..16 {
            let position = PointF { x: 700., y: 600. };
            let delta_y = if i < 10 { 45. } else { -70. };
            let scroll = UiEvent::Scroll {
                position,
                delta_x: 0.,
                delta_y,
            };
            app.event(scroll).unwrap();
            steps.push(snapshot(&mut app, false));
        }
        for i in 0..30 {
            let (w, h) = (1440. - 16.3 * i as f32, 1040. - (i % 4) as f32 * 7.);
            app.resize(w, h).unwrap();
            steps.push(snapshot(&mut app, i == 29));
        }
        app.send(Msg::Search("que".into())).unwrap();
        steps.push(snapshot(&mut app, true));
        steps
    }
    let (cached, full) = (play(true), play(false));
    assert_eq!(cached.len(), full.len());
    for (step, (a, b)) in cached.iter().zip(&full).enumerate() {
        let line = a.iter().zip(b).position(|(x, y)| x != y);
        assert!(
            line.is_none() && a.len() == b.len(),
            "step {step}: {line:?}"
        );
    }
}
