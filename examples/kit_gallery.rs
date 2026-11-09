//! Galerie des composants du kit : `cargo run --release --example kit_gallery`.
//! Aperçu sans fenêtre : `-- --export galerie.png [--size 1280x900] [--section 0] [--dark] [--dialog]`.
use karbon_ui::{
    Error, diagnostics,
    diagnostics::BenchmarkOptions,
    platform,
    ui::kit::{ICON_NAMES, prelude::*},
    window::{RenderMode, WindowOptions},
};

#[derive(Clone)]
enum Msg {
    Save,
    Name(String),
    Bio(String),
    Genre(u64),
    Terms(bool),
    Notify(bool),
    Quality(u64),
    Volume(f64),
    Tab(usize),
    Page(usize),
    Dialog(bool),
    Menu,
    Section(usize),
    Fold(usize),
    Slide(usize),
    Replay,
    Follow,
    Log(&'static str, bool),
    Clear,
    Dark(bool),
    Accent(u64),
    Toast,
}

const ACCENTS: [u32; 4] = [0x2563eb, 0x7c3aed, 0xe11d48, 0x059669];
/// Rubriques de la galerie, par type de composant : nom, icône, description.
const SECTIONS: [(&str, &str, &str); 12] = [
    ("Aperçu", "home", ""),
    (
        "Typographie",
        "edit",
        "Titres, paragraphes, code et icônes.",
    ),
    ("Boutons", "plus", "Couleurs, apparences, tailles et états."),
    (
        "Formulaires",
        "file",
        "Champs, listes, cases, interrupteurs et curseurs.",
    ),
    (
        "Données",
        "folder",
        "Chiffres clés, tableaux, listes et accordéons.",
    ),
    (
        "Retours d'état",
        "bell",
        "Alertes, progression et chargement.",
    ),
    ("Navigation", "menu", "Onglets, fil d'Ariane et étapes."),
    (
        "Pagination",
        "more-horizontal",
        "Parcourir une longue liste, page par page.",
    ),
    ("Carrousel", "image", "Faire défiler des diapositives."),
    (
        "Animation",
        "play",
        "Entrées et boucles, par simple attribut.",
    ),
    (
        "Événements",
        "info",
        "Survol, appui, focus, clics : tout arrive dans update.",
    ),
    ("Couches", "copy", "Dialogues, menus et infobulles."),
];

css_file!(mod css, "kit_gallery/gallery.css");

struct Gallery {
    name: String,
    bio: String,
    genre: Option<u64>,
    terms: bool,
    notify: bool,
    quality: Option<u64>,
    volume: f64,
    tab: usize,
    page: usize,
    dialog: bool,
    menu: bool,
    section: usize,
    fold: usize,
    slide: usize,
    replay: u64,
    follow: bool,
    log: Vec<(&'static str, bool)>,
    dark: bool,
    accent: u64,
    toast: bool,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            name: String::new(),
            bio: String::new(),
            genre: Some(1),
            terms: true,
            notify: false,
            quality: Some(2),
            volume: 40.,
            tab: 0,
            page: 4,
            dialog: false,
            menu: false,
            section: 0,
            fold: 0,
            slide: 0,
            replay: 0,
            follow: false,
            log: Vec::new(),
            dark: false,
            accent: 0,
            toast: false,
        }
    }
}

impl Application for Gallery {
    type Message = Msg;

    fn fonts(&self) -> &[FontAsset] {
        &[FontAsset {
            family: "Inter",
            data: include_bytes!("../tests/fixtures/fonts/inter.woff2"),
        }]
    }

    fn color_preferences(&self) -> ColorPreferences {
        ColorPreferences {
            scheme: if self.dark {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            },
            high_contrast: false,
        }
    }

    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Save => self.toast = true,
            Msg::Name(v) => self.name = v,
            Msg::Bio(v) => self.bio = v,
            Msg::Genre(v) => self.genre = Some(v),
            Msg::Terms(v) => self.terms = v,
            Msg::Notify(v) => self.notify = v,
            Msg::Quality(v) => self.quality = Some(v),
            Msg::Volume(v) => self.volume = v,
            Msg::Tab(v) => self.tab = v,
            Msg::Page(v) => self.page = v,
            Msg::Dialog(v) => self.dialog = v,
            Msg::Menu => self.menu = !self.menu,
            Msg::Section(v) => self.section = v,
            Msg::Fold(v) => self.fold = v,
            Msg::Slide(v) => self.slide = v,
            Msg::Replay => self.replay += 1,
            Msg::Follow => self.follow = !self.follow,
            Msg::Log(name, value) => {
                self.log.push((name, value));
                let excess = self.log.len().saturating_sub(7);
                self.log.drain(..excess);
            }
            Msg::Clear => self.log.clear(),
            Msg::Dark(v) => self.dark = v,
            Msg::Accent(v) => {
                self.accent = v;
                set_theme(KitTheme::default().primary(Color::hex(ACCENTS[v as usize])));
            }
            Msg::Toast => self.toast = false,
        }
        true
    }

    fn view(&self, _: &ViewContext) -> Element {
        ui_file!("kit_gallery/page.ui", styles = css)
    }
}

fn main() -> karbon_ui::Result<()> {
    let mut app = Gallery::default();
    let (mut export, mut size) = (None, (1280, 900));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--export" => export = args.next(),
            "--dark" => app.dark = true,
            "--dialog" => app.dialog = true,
            "--section" => {
                app.section = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| *n < SECTIONS.len())
                    .ok_or_else(|| Error::application("--section expects 0 to 11"))?;
            }
            "--size" => {
                let text = args.next().unwrap_or_default();
                let (w, h) = text
                    .split_once('x')
                    .ok_or_else(|| Error::application("--size expects 1280x900"))?;
                size = (
                    w.parse().map_err(Error::application)?,
                    h.parse().map_err(Error::application)?,
                );
            }
            _ => {}
        }
    }
    if let Some(path) = export {
        let image = diagnostics::render_preview(
            BenchmarkOptions {
                width: size.0,
                height: size.1,
                scale: 1.,
                title_bar: false,
                reduced_motion: true,
                renderer: RenderMode::Software,
                ..Default::default()
            },
            app,
        )?;
        return diagnostics::write_png(&image, std::path::Path::new(&path));
    }
    platform::run(
        WindowOptions {
            title: "Karbon UI · galerie",
            width: 1200,
            height: 860,
            ..WindowOptions::DEFAULT
        },
        app,
    )
}
