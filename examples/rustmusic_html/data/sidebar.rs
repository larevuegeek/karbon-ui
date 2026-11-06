//! Barre latérale : navigation, onglets et bibliothèques, playlists, podcasts, sources.

/// SidebarNav: the home tile, before the library tabs.
pub static ACCUEIL: (&str, &str, &str) =
    ("home", "Accueil", "material-symbols:home-outline-rounded");

/// The library tabs (`ONGLETS_BIBLIOTHEQUE` of `config/libraryTabs.ts`): key, label,
/// icon; `MusicApp::onglets` says which ones are shown.
pub static ONGLETS: [(&str, &str, &str); 8] = [
    ("tracks", "Morceaux", "material-symbols:music-note-rounded"),
    (
        "artists",
        "Artistes",
        "material-symbols:mic-external-on-outline-rounded",
    ),
    ("albums", "Albums", "material-symbols:album-outline-rounded"),
    ("genres", "Genres", "material-symbols:sell-outline-rounded"),
    (
        "years",
        "Années",
        "material-symbols:calendar-month-outline-rounded",
    ),
    (
        "folders",
        "Dossiers",
        "material-symbols:folder-outline-rounded",
    ),
    ("mixes", "Mix", "material-symbols:shuffle-rounded"),
    (
        "playlists",
        "Playlists",
        "material-symbols:queue-music-rounded",
    ),
];

/// SidebarFooter: last scan of the library.
pub static SCAN: &str = "Scannée il y a 4 j";

/// A playlist row of the sidebar or a `PlaylistCard` of the home page.
pub struct Playlist {
    pub id: u64,
    pub nom: &'static str,
    pub sous_titre: &'static str,
    pub couleur: &'static str,
    pub icone: &'static str,
    pub auto: bool,
    pub couvertures: &'static [&'static str],
}

/// Sidebar section "Playlists": pinned playlists, then liked and recent.
pub static SIDEBAR_PLAYLISTS: [Playlist; 4] = [
    Playlist {
        id: 1,
        nom: "Les plus écoutés",
        sous_titre: "Playlist auto · 47 titres",
        couleur: "#22c55e",
        icone: "material-symbols:auto-awesome-rounded",
        auto: true,
        couvertures: &["pablo_honey", "facelift", "ten", "bssm"],
    },
    Playlist {
        id: 2,
        nom: "Grunge",
        sous_titre: "Playlist · vide",
        couleur: "#8b5cf6",
        icone: "mynaui:lightning",
        auto: false,
        couvertures: &[],
    },
    Playlist {
        id: 3,
        nom: "Titres likés",
        sous_titre: "Playlist · 3 titres",
        couleur: "#f43f5e",
        icone: "material-symbols:favorite-rounded",
        auto: false,
        couvertures: &["reggatta"],
    },
    Playlist {
        id: 4,
        nom: "Récemment joués",
        sous_titre: "Historique · 50 titres",
        couleur: "#0ea5e9",
        icone: "material-symbols:history-rounded",
        auto: false,
        couvertures: &["zeppelin4", "bohemian", "iz_it_a_crime", "aaliyah"],
    },
];

/// Sidebar section "Écoute en ligne": the four latest podcast covers.
pub static PODCASTS: [&str; 4] = [
    "podcast_gt",
    "podcast_legend",
    "podcast_crime",
    "podcast_hondelatte",
];
pub static PODCASTS_DETAIL: &str = "138 nouveaux épisodes";

/// A source of `sidebar/SourceSwitcher.svelte` (state of the captures: one Navidrome
/// server, unreachable).
pub struct Source {
    pub nom: &'static str,
    pub icone: &'static str,
    /// Tint of the icon tile: `local`, `serveur` or `toutes` (TEINTES).
    pub teinte: &'static str,
    pub detail: &'static str,
    /// Status dot of a server (`StatusDot`): `Some(false)` when unreachable.
    pub en_ligne: Option<bool>,
}
pub static SOURCES: [Source; 3] = [
    Source {
        nom: "Cet ordinateur",
        icone: "material-symbols:computer-outline-rounded",
        teinte: "local",
        detail: "4 bibliothèques · 14 231 titres",
        en_ligne: None,
    },
    Source {
        nom: "Navidrome · localhost",
        icone: "material-symbols:cloud-outline-rounded",
        teinte: "serveur",
        detail: "Injoignable · 10 179 titres",
        en_ligne: Some(false),
    },
    Source {
        nom: "Toutes les sources",
        icone: "material-symbols:stacks-outline-rounded",
        teinte: "toutes",
        detail: "24 410 titres",
        en_ligne: None,
    },
];

/// Library tab placements of `LibraryTabsOptions.svelte`: label, left bar lit, top bar lit.
pub static PLACEMENTS_ONGLETS: [(&str, bool, bool); 3] = [
    ("À gauche", true, false),
    ("En haut", false, true),
    ("Les deux", true, true),
];

/// Built-in shortcuts of `PlaylistZoneMenu.svelte`: title, icon (the sidebar rows of
/// the same name).
pub static RACCOURCIS: [(&str, &str); 2] = [
    ("Titres likés", "mynaui:heart-solid"),
    ("Récemment joués", "mynaui:clock-8"),
];

/// A library of `LibrarySelector.svelte` (local source of the captures): name, tracks
/// (formatted for the button, raw count for the list), albums, default library.
pub struct Bibliotheque {
    pub nom: &'static str,
    pub titres: &'static str,
    pub pistes: u32,
    pub albums: &'static str,
    pub defaut: bool,
}
pub static BIBLIOTHEQUES: [Bibliotheque; 4] = [
    Bibliotheque {
        nom: "Musique HD",
        titres: "9\u{202f}440 titres",
        pistes: 9440,
        albums: "602 albums",
        defaut: true,
    },
    Bibliotheque {
        nom: "DSD",
        titres: "451 titres",
        pistes: 451,
        albums: "43 albums",
        defaut: false,
    },
    Bibliotheque {
        nom: "Musique MP3",
        titres: "4\u{202f}340 titres",
        pistes: 4340,
        albums: "283 albums",
        defaut: false,
    },
    Bibliotheque {
        nom: "Test",
        titres: "0 titres",
        pistes: 0,
        albums: "0 albums",
        defaut: false,
    },
];
