//! Galerie du kit : écrans d'exemple complets et composants par type.
//! `cargo run --release --example kit_gallery`.
//! Aperçu sans fenêtre : `-- --export galerie.png [--size 1320x860] [--scale 2] [--section 0]
//! [--dark]` (logiciel, sauf `--renderer=gpu|auto`). Les autres options sont celles de
//! [`WindowOptions::from_args`] (`--profile`, `--renderer=…`).
#[path = "kit_gallery/data.rs"]
mod data;
use data::*;
use karbon_ui::{
    Error, diagnostics,
    diagnostics::BenchmarkOptions,
    platform,
    ui::{
        kit::{ICON_NAMES, prelude::*},
        scene::PointF,
    },
    window::{RenderMode, WindowOptions},
};

css_file!(mod css, "kit_gallery/gallery.css");

#[derive(Clone)]
enum Msg {
    Section(usize),
    Save,
    Toast,
    Dark(bool),
    Accent(u64),
    Dialog(bool),
    /// Tiroir latéral : `Some(true)` à droite, `Some(false)` à gauche, `None` fermé.
    Drawer(Option<bool>),
    Menu,
    // Clic droit : demande du contenu, fermeture, entrée choisie.
    OpenContext(ContextMenuRequest),
    CloseContext,
    Pick(&'static str),
    // Composants.
    Name(String),
    Bio(String),
    Genre(u64),
    Terms(bool),
    Notify(bool),
    Quality(u64),
    Volume(f64),
    Tab(usize),
    Tab2(usize),
    Page(usize),
    Fold(usize),
    Slide(usize),
    Slide2(usize),
    Replay,
    Follow,
    Log(&'static str, bool),
    Clear,
    // Tableau de bord, boutique.
    Range(usize),
    Category(usize),
    Favorite(usize),
    AddToCart(usize),
    EmptyCart,
    // Lecteur.
    Play,
    Next,
    Previous,
    Track(usize),
    Seek(f64),
    Like,
    // Tarifs.
    Billing(usize),
    Plan(usize),
    // Connexion.
    Email(String),
    Password(String),
    ShowPassword,
    Remember(bool),
    SignIn,
    // Réglages, messagerie.
    SettingsTab(usize),
    Security(bool),
    Weekly(bool),
    Conversation(usize),
    Draft(String),
    Send,
}

#[derive(Default)]
struct Gallery {
    section: usize,
    dark: bool,
    accent: u64,
    dialog: bool,
    drawer: Option<bool>,
    menu: bool,
    /// Position du menu contextuel ouvert.
    context_at: Option<PointF>,
    /// Dernière entrée choisie dans le menu contextuel.
    picked: &'static str,
    toast: bool,
    name: String,
    bio: String,
    genre: Option<u64>,
    terms: bool,
    notify: bool,
    quality: Option<u64>,
    volume: f64,
    tab: usize,
    tab2: usize,
    page: usize,
    fold: usize,
    slide: usize,
    slide2: usize,
    replay: u64,
    follow: bool,
    log: Vec<(&'static str, bool)>,
    range: usize,
    category: usize,
    favorites: [bool; PRODUCTS.len()],
    cart: u32,
    cart_total: u32,
    playing: bool,
    track: usize,
    position: f64,
    liked: bool,
    billing: usize,
    plan: usize,
    email: String,
    password: String,
    show_password: bool,
    remember: bool,
    tried: bool,
    signed_in: bool,
    settings_tab: usize,
    security: bool,
    weekly: bool,
    conversation: usize,
    draft: String,
    messages: [Vec<String>; CONVERSATIONS.len()],
}

impl Gallery {
    fn new() -> Self {
        Self {
            genre: Some(1),
            terms: true,
            quality: Some(1),
            volume: 40.,
            page: 4,
            position: 96.,
            plan: 0,
            security: true,
            ..Self::default()
        }
    }
    fn visible_products(&self) -> Vec<(usize, &'static Product)> {
        PRODUCTS
            .iter()
            .enumerate()
            .filter(|(_, p)| self.category == 0 || p.1 == self.category)
            .collect()
    }
    fn email_error(&self) -> &'static str {
        if self.tried && !self.email.contains('@') {
            "Saisissez une adresse valide."
        } else {
            ""
        }
    }
    fn password_error(&self) -> &'static str {
        if self.tried && self.password.len() < 6 {
            "Six caractères au minimum."
        } else {
            ""
        }
    }
    fn change_track(&mut self, track: usize) {
        self.track = track;
        self.position = 0.;
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
            Msg::Section(v) => self.section = v,
            Msg::Save => self.toast = true,
            Msg::Toast => self.toast = false,
            Msg::Dark(v) => self.dark = v,
            Msg::Accent(v) => {
                self.accent = v;
                set_theme(KitTheme::default().primary(Color::hex(ACCENTS[v as usize])));
            }
            Msg::Dialog(v) => self.dialog = v,
            Msg::Drawer(v) => self.drawer = v,
            Msg::Menu => self.menu = !self.menu,
            Msg::OpenContext(request) => self.context_at = Some(request.position),
            Msg::CloseContext => self.context_at = None,
            Msg::Pick(entry) => self.picked = entry,
            Msg::Name(v) => self.name = v,
            Msg::Bio(v) => self.bio = v,
            Msg::Genre(v) => self.genre = Some(v),
            Msg::Terms(v) => self.terms = v,
            Msg::Notify(v) => self.notify = v,
            Msg::Quality(v) => self.quality = Some(v),
            Msg::Volume(v) => self.volume = v,
            Msg::Tab(v) => self.tab = v,
            Msg::Tab2(v) => self.tab2 = v,
            Msg::Page(v) => self.page = v,
            Msg::Fold(v) => self.fold = v,
            Msg::Slide(v) => self.slide = v,
            Msg::Slide2(v) => self.slide2 = v,
            Msg::Replay => self.replay += 1,
            Msg::Follow => self.follow = !self.follow,
            Msg::Log(name, value) => {
                self.log.push((name, value));
                let excess = self.log.len().saturating_sub(7);
                self.log.drain(..excess);
            }
            Msg::Clear => self.log.clear(),
            Msg::Range(v) => self.range = v,
            Msg::Category(v) => self.category = v,
            Msg::Favorite(i) => self.favorites[i] = !self.favorites[i],
            Msg::AddToCart(i) => {
                self.cart += 1;
                self.cart_total += PRODUCTS[i].2;
            }
            Msg::EmptyCart => (self.cart, self.cart_total) = (0, 0),
            Msg::Play => self.playing = !self.playing,
            Msg::Next => self.change_track((self.track + 1) % TRACKS.len()),
            Msg::Previous => self.change_track((self.track + TRACKS.len() - 1) % TRACKS.len()),
            Msg::Track(i) => self.change_track(i),
            Msg::Seek(v) => self.position = v,
            Msg::Like => self.liked = !self.liked,
            Msg::Billing(v) => self.billing = v,
            Msg::Plan(v) => self.plan = v,
            Msg::Email(v) => self.email = v,
            Msg::Password(v) => self.password = v,
            Msg::ShowPassword => self.show_password = !self.show_password,
            Msg::Remember(v) => self.remember = v,
            Msg::SignIn => {
                self.tried = true;
                self.signed_in = self.email.contains('@') && self.password.len() >= 6;
            }
            Msg::SettingsTab(v) => self.settings_tab = v,
            Msg::Security(v) => self.security = v,
            Msg::Weekly(v) => self.weekly = v,
            Msg::Conversation(v) => self.conversation = v,
            Msg::Draft(v) => self.draft = v,
            Msg::Send => {
                if !self.draft.trim().is_empty() {
                    let text = std::mem::take(&mut self.draft);
                    self.messages[self.conversation].push(text);
                }
            }
        }
        true
    }

    fn view(&self, _: &ViewContext) -> Element {
        // Les couleurs du thème, pour `gallery.css`.
        let variables = karbon_ui::ui::kit::variables();
        let cx = TemplateContext::default().with_variables(&variables);
        ui_file!("kit_gallery/page.ui", context = cx, styles = css)
    }
}

fn main() -> karbon_ui::Result<()> {
    let mut app = Gallery::new();
    let (mut export, mut size, mut scale) = (None, (1320, 860), 1.);
    let mut native = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--export" => export = args.next(),
            "--dark" => app.dark = true,
            "--dialog" => app.dialog = true,
            "--drawer" => app.drawer = Some(true),
            "--section" => {
                app.section = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| *n < SECTIONS.len())
                    .ok_or_else(|| Error::application("--section expects a section number"))?;
            }
            "--size" => {
                let text = args.next().unwrap_or_default();
                let (w, h) = text
                    .split_once('x')
                    .ok_or_else(|| Error::application("--size expects 1320x860"))?;
                size = (
                    w.parse().map_err(Error::application)?,
                    h.parse().map_err(Error::application)?,
                );
            }
            "--scale" => {
                scale = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n: &f32| n.is_finite() && *n > 0.)
                    .ok_or_else(|| Error::application("--scale expects a factor, e.g. 2"))?;
            }
            _ => native.push(arg),
        }
    }
    let chosen = native.iter().any(|a| a.starts_with("--renderer="));
    let options = WindowOptions::from_args(native)?;
    if let Some(path) = export {
        let image = diagnostics::render_preview(
            BenchmarkOptions {
                width: size.0,
                height: size.1,
                scale,
                title_bar: false,
                reduced_motion: true,
                renderer: if chosen {
                    options.renderer
                } else {
                    RenderMode::Software
                },
                ..Default::default()
            },
            app,
        )?;
        return diagnostics::write_png(&image, std::path::Path::new(&path));
    }
    platform::run(
        WindowOptions {
            title: "Karbon UI · galerie",
            width: 1280,
            height: 880,
            ..options
        },
        app,
    )
}
