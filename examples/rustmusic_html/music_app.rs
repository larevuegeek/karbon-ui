pub(super) use super::assets::Assets;
pub(super) use super::color::*;
pub(super) use super::components::*;
use super::*;
pub(super) use karbon_ui::{
    application::Resources,
    ui::prelude::*,
    ui::runtime::Key,
    window::{WindowAction, WindowCommand, WindowId},
};
pub(super) use std::sync::Arc;

#[rustfmt::skip]
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Noop, Play, Like, Shuffle, Repeat, Mute, ToggleTheme, ToggleFooter,
    /// Title bar button: collapses or expands the sidebar (`sidebar_collapsed`).
    ToggleSidebar,
    Search(String), Seek(f64), Volume(f64),
    Minimize, Maximize, Close,
    /// Opens a menu or panel, or closes it when it is already open (its trigger).
    Menu(Menu),
    /// `onclose` of the open popover.
    CloseMenu,
    /// Sleep timer choice (`None` cancels it); closes the panel.
    Veille(Option<Veille>),
    /// Custom sleep duration typed in the panel, applied by `VeillePerso`.
    VeilleSaisie(String),
    VeillePerso,
    /// Index in `data::SOURCES`.
    ChoisirSource(usize),
    /// Index in `data::BIBLIOTHEQUES`.
    ChoisirBibliotheque(usize),
    /// Library sections: placement (0 left, 1 top, 2 both), shown section (index in
    /// `data::ONGLETS`), Fichier / Dossier row hidden.
    PlacementOnglets(u8),
    BasculerOnglet(usize),
    MasquerOuvrir,
    /// Built-in shortcut of the playlists ("Titres likés", "Récemment joués").
    BasculerRaccourci(usize),
    /// Index in `data::SORTIES`.
    ChoisirSortie(usize),
    /// `wasapi_exclusive` and `dsd_dop` of the output menu.
    BasculerExclusif,
    BasculerDop,
    /// Network service (index in `data::SERVICES_RESEAU`).
    BasculerReseau(usize),
    /// Key pressed in the search field (`handleKeydown`: Up, Down, Enter).
    SearchKey(Key),
    /// Search result chosen (index in `MusicApp::resultats`).
    SearchPick(usize),
}

pub struct MusicApp {
    pub(super) assets: Option<Arc<Assets>>,
    pub playing: bool,
    pub liked: bool,
    pub shuffle: bool,
    pub repeat: bool,
    pub muted: bool,
    pub dark: bool,
    pub footer_collapsed: bool,
    /// `sidebar_collapsed` setting (`replie` in the Svelte components): icon-only
    /// column of 76 px instead of 288 px.
    pub sidebar_collapsed: bool,
    pub volume: f64,
    pub position: f64,
    pub search: String,
    /// Highlighted search result (`selectedIndex`, -1 = `None`).
    pub suggestion: Option<usize>,
    /// Open menu or panel (one at a time, like `k-popover`).
    pub menu: Option<Menu>,
    pub veille: Option<Veille>,
    pub veille_saisie: String,
    pub source: usize,
    pub bibliotheque: usize,
    /// `library_tabs_position`: 0 left (sidebar tiles), 1 top, 2 both.
    pub placement_onglets: u8,
    /// `library_tabs`: sections shown, in `data::ONGLETS` order.
    pub onglets: [bool; 8],
    /// `show_open_buttons`
    pub boutons_ouvrir: bool,
    /// `show_liked_in_playlists`, `show_recent_in_playlists`
    pub raccourcis: [bool; 2],
    pub sortie: usize,
    pub exclusif: bool,
    pub dop: bool,
    /// UPnP/DLNA players, AirPlay, DLNA sharing (`data::SERVICES_RESEAU`).
    pub reseau: [bool; 3],
    pub(super) commands: Vec<WindowCommand>,
}
impl MusicApp {
    /// The page with `menu` open (exports compared with the reference captures).
    pub fn with_menu(menu: Option<Menu>) -> Self {
        Self {
            menu,
            // Capture menu-search : « que » tapé dans le champ.
            search: if menu == Some(Menu::Recherche) {
                "que".into()
            } else {
                String::new()
            },
            ..Self::default()
        }
    }
    /// Same page with the sidebar collapsed (`--sidebar collapsed`) or expanded.
    pub fn with_sidebar(mut self, collapsed: bool) -> Self {
        self.sidebar_collapsed = collapsed;
        self
    }
    pub(super) fn open(&self, menu: Menu) -> bool {
        self.menu == Some(menu)
    }
}
impl Default for MusicApp {
    fn default() -> Self {
        Self {
            assets: None,
            playing: false,
            liked: false,
            shuffle: false,
            repeat: false,
            muted: false,
            dark: true,
            footer_collapsed: true,
            sidebar_collapsed: false,
            volume: 80.,
            position: 0.,
            search: String::new(),
            suggestion: None,
            menu: None,
            veille: None,
            veille_saisie: "20".into(),
            source: 0,
            bibliotheque: 0,
            placement_onglets: 0,
            // Morceaux, Artistes, Albums, Genres, Dossiers (captures) ; Années, Mix et
            // Playlists masqués.
            onglets: [true, true, true, true, false, true, false, false],
            boutons_ouvrir: true,
            raccourcis: [true, true],
            sortie: 0,
            exclusif: false,
            dop: true,
            reseau: [true; 3],
            commands: Vec::new(),
        }
    }
}
impl Application for MusicApp {
    type Message = Msg;
    fn fonts(&self) -> &[FontAsset] {
        &assets::FONTS
    }
    fn load(&mut self, resources: &dyn Resources) -> karbon_ui::Result<()> {
        self.assets = Some(Arc::new(Assets::load(resources)?));
        Ok(())
    }
    fn view(&self, _: &ViewContext) -> Element {
        let assets = self
            .assets
            .as_deref()
            .expect("Application::load precedes view");
        let cx = TemplateContext::new(assets, "rustmusic");
        let cover = |name: &str| assets.cover(name);
        // style:width={`calc((100% - 12px) * ${value / 100} + 6px)`} du Svelte : la valeur
        // passe par la propriété héritée --volume, le calc() reste dans la feuille.
        let volume = if self.muted { 0. } else { self.volume / 100. } as f32;
        ui_file!("ui/layout.ui", context = cx, styles = css)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Noop => return false,
            Msg::Play => self.playing = !self.playing,
            Msg::Like => self.liked = !self.liked,
            Msg::Shuffle => self.shuffle = !self.shuffle,
            Msg::Repeat => self.repeat = !self.repeat,
            Msg::Mute => self.muted = !self.muted,
            Msg::ToggleTheme => self.dark = !self.dark,
            Msg::ToggleFooter => self.footer_collapsed = !self.footer_collapsed,
            Msg::ToggleSidebar => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                // Repliée, la barre n'a plus ni Fichier / Dossier ni titre de section :
                // leurs menus « … » perdent leur bouton.
                if matches!(self.menu, Some(Menu::Ouvrir | Menu::Affichage)) {
                    self.menu = None;
                }
            }
            Msg::Search(value) => {
                // handleInput + doSearch : la liste s'ouvre dès deux caractères et des résultats.
                self.search = value;
                self.suggestion = None;
                let ouverte = !self.resultats().is_empty();
                if ouverte {
                    self.menu = Some(Menu::Recherche);
                } else if self.open(Menu::Recherche) {
                    self.menu = None;
                }
            }
            Msg::SearchKey(key) if self.open(Menu::Recherche) => {
                let n = self.resultats().len();
                match key {
                    Key::Down => {
                        self.suggestion = Some(self.suggestion.map_or(0, |i| (i + 1).min(n - 1)))
                    }
                    Key::Up => self.suggestion = self.suggestion.and_then(|i| i.checked_sub(1)),
                    // Sans suggestion choisie : la page de résultats (non traduite), puis fermer.
                    Key::Enter => self.menu = None,
                    _ => return false,
                }
            }
            Msg::SearchKey(_) => return false,
            Msg::SearchPick(_) => {
                // Lecture ou navigation vers le résultat : non traduites ici.
                self.menu = None;
                self.suggestion = None;
            }
            Msg::Menu(menu) => self.menu = if self.open(menu) { None } else { Some(menu) },
            Msg::CloseMenu => self.menu = None,
            Msg::Veille(veille) => {
                self.veille = veille;
                self.menu = None;
            }
            Msg::VeilleSaisie(value) => self.veille_saisie = value,
            Msg::ChoisirSource(i) => {
                self.source = i;
                self.menu = None;
            }
            Msg::ChoisirBibliotheque(i) => {
                self.bibliotheque = i;
                self.menu = None;
            }
            Msg::PlacementOnglets(p) => self.placement_onglets = p,
            Msg::BasculerOnglet(i) => {
                // La dernière section affichée reste (bouton désactivé dans le Svelte).
                if !self.onglets[i] || self.onglets.iter().filter(|o| **o).count() > 1 {
                    self.onglets[i] = !self.onglets[i];
                }
            }
            Msg::MasquerOuvrir => {
                self.boutons_ouvrir = false;
                self.menu = None;
            }
            Msg::BasculerRaccourci(i) => self.raccourcis[i] = !self.raccourcis[i],
            Msg::ChoisirSortie(i) => {
                self.sortie = i;
                self.menu = None;
            }
            Msg::BasculerExclusif => self.exclusif = !self.exclusif,
            Msg::BasculerDop => self.dop = !self.dop,
            Msg::BasculerReseau(i) => self.reseau[i] = !self.reseau[i],
            Msg::VeillePerso => {
                // Math.max(1, Math.min(600, Math.round(customMin)))
                let minutes = self.veille_saisie.trim().parse::<u32>().unwrap_or(20);
                self.veille = Some(Veille::Minutes(minutes.clamp(1, 600)));
                self.menu = None;
            }
            Msg::Seek(value) => self.position = value,
            Msg::Volume(value) => {
                self.volume = value;
                self.muted = false;
            }
            Msg::Minimize | Msg::Maximize | Msg::Close => {
                let action = match message {
                    Msg::Minimize => WindowAction::Minimize,
                    Msg::Maximize => WindowAction::ToggleMaximize,
                    _ => WindowAction::Close,
                };
                self.commands
                    .push(WindowCommand::Action(WindowId::MAIN, action));
                return false;
            }
        }
        true
    }
    fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.commands)
    }
}

/// `formatTime` of `src/lib/helper/tools/dateTools.ts`: `mm:ss`.
pub fn format_time(seconds: f64) -> String {
    let s = seconds.max(0.) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// `minutesSeconds` of the player: `m:ss`.
pub fn minutes_seconds(seconds: f64) -> String {
    let s = seconds.max(0.) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}
