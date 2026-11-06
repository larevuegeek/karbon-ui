//! Accueil : salutation, statistiques, mix du jour, playlists, genres, ajouts récents
//! et « Récemment joués ».
use super::{player::Fiche, sidebar::Playlist};

/// `greeting()` of HomeHeader.svelte at the time of the reference captures (evening).
pub static GREETING: &str = "Bonsoir";

/// `LibraryStats` of the selected library. `Intl.NumberFormat("fr")` groups thousands
/// with a narrow no-break space (U+202F).
pub struct Stat {
    pub n: &'static str,
    pub libelle: &'static str,
}
pub static STATS: [Stat; 3] = [
    Stat {
        n: "9\u{202f}440",
        libelle: "titres",
    },
    Stat {
        n: "602",
        libelle: "albums",
    },
    Stat {
        n: "995",
        libelle: "artistes",
    },
];

/// `SpecMix` + `preview()` of a "Pour vous" mix.
pub struct Mix {
    pub cle: &'static str,
    pub kind: &'static str,
    pub nom: &'static str,
    pub h: f32,
    pub pochettes: [&'static str; 4],
    pub description: &'static str,
    pub meta: &'static str,
}
pub static MIXES: [Mix; 4] = [
    Mix {
        cle: "genre:pop",
        kind: "Mix du jour",
        nom: "Pop",
        h: 305., // hue("pop")
        pochettes: ["beatles_past1", "adele25", "divide", "overexposed"],
        description: "The Beatles, Ed Sheeran, Britney Spears et d'autres",
        meta: "50 titres · 3 h 13 min",
    },
    Mix {
        cle: "genre:rock",
        kind: "Mix du jour",
        nom: "Rock",
        h: 341., // hue("rock")
        pochettes: ["rush", "no_line", "lights_out", "u2_best"],
        description: "Måneskin, U2, Nirvana et d'autres",
        meta: "50 titres · 3 h 29 min",
    },
    Mix {
        cle: "oublies",
        kind: "Redécouvrir",
        nom: "Oubliés",
        h: 200.,
        pochettes: ["slim_shady", "ballo", "top50", "stp100"],
        description: "Daft Punk, Robbie Williams, U2 et d'autres",
        meta: "50 titres · 3 h 10 min",
    },
    Mix {
        cle: "hires",
        kind: "Qualité studio",
        nom: "Hi-Res",
        h: 150.,
        pochettes: ["zimmer_live", "bohemian", "flower_moon", "peaky"],
        description: "Hans Zimmer, Anna Calvi, Michael Jackson et d'autres",
        meta: "50 titres · 2 h 48 min",
    },
];

/// `HomePlaylists`: the profile's playlists, pinned first.
pub static HOME_PLAYLISTS: [Playlist; 6] = [
    Playlist {
        id: 1,
        nom: "Les plus écoutés",
        sous_titre: "Playlist auto · 47 titres",
        couleur: "#22c55e",
        icone: "material-symbols:auto-awesome-rounded",
        auto: true,
        couvertures: &["nrj", "now", "snoop", "queen"],
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
        id: 5,
        nom: "Bandes originales",
        sous_titre: "Playlist · 64 titres",
        couleur: "#f59e0b",
        icone: "material-symbols:queue-music-rounded",
        auto: false,
        couvertures: &["jurassic", "lotr", "madmax", "it"],
    },
    Playlist {
        id: 6,
        nom: "Années 80",
        sous_titre: "Playlist auto · 120 titres",
        couleur: "#ec4899",
        icone: "material-symbols:auto-awesome-rounded",
        auto: true,
        couvertures: &["bowie", "bonjovi", "aerosmith", "queen"],
    },
    Playlist {
        id: 7,
        nom: "Soirée",
        sous_titre: "Playlist · 32 titres",
        couleur: "#0ea5e9",
        icone: "material-symbols:queue-music-rounded",
        auto: false,
        couvertures: &["katy"],
    },
    Playlist {
        id: 8,
        nom: "Acoustique",
        sous_titre: "Playlist · 18 titres",
        couleur: "#14b8a6",
        icone: "material-symbols:queue-music-rounded",
        auto: false,
        couvertures: &["unplugged", "sting", "adele", "coldplay"],
    },
];

/// `GenreView`, sorted by track count.
pub struct Genre {
    pub name: &'static str,
    pub total: &'static str,
    pub icone: &'static str,
}
pub static GENRES: [Genre; 6] = [
    Genre {
        name: "Pop",
        total: "2 184",
        icone: "ph:sparkle-duotone",
    },
    Genre {
        name: "Rock",
        total: "1 902",
        icone: "ph:guitar-duotone",
    },
    Genre {
        name: "Bande originale",
        total: "1 266",
        icone: "ph:film-slate-duotone",
    },
    Genre {
        name: "Électro",
        total: "845",
        icone: "ph:waveform-duotone",
    },
    Genre {
        name: "Grunge",
        total: "512",
        icone: "ph:drop-duotone",
    },
    Genre {
        name: "Chanson française",
        total: "438",
        icone: "ph:microphone-duotone",
    },
];

/// `AlbumListView`: recently added albums, and the album strip of "Récemment joués".
pub struct Album {
    pub id: u64,
    pub titre: &'static str,
    pub artiste: &'static str,
    pub annee: &'static str,
    pub pochette: &'static str,
    pub pistes: u32,
    pub ecoute: &'static str,
}
pub static RECENTLY_ADDED: [Album; 6] = [
    Album {
        id: 1,
        titre: "GIMME GIMME",
        artiste: "Katy Perry",
        annee: "2024",
        pochette: "katy",
        pistes: 1,
        ecoute: "",
    },
    Album {
        id: 2,
        titre: "It (Original Motion Picture Soundtrack)",
        artiste: "Benjamin Wallfisch",
        annee: "2017",
        pochette: "it",
        pistes: 0,
        ecoute: "",
    },
    Album {
        id: 3,
        titre: "Mad Max: Fury Road",
        artiste: "Junkie XL",
        annee: "2015",
        pochette: "madmax",
        pistes: 0,
        ecoute: "",
    },
    Album {
        id: 4,
        titre: "Viva la Vida",
        artiste: "Coldplay",
        annee: "2008",
        pochette: "coldplay",
        pistes: 0,
        ecoute: "",
    },
    Album {
        id: 5,
        titre: "MTV Unplugged in New York",
        artiste: "Nirvana",
        annee: "1994",
        pochette: "unplugged",
        pistes: 0,
        ecoute: "",
    },
    Album {
        id: 6,
        titre: "Jurassic Park",
        artiste: "John Williams",
        annee: "1993",
        pochette: "jurassic",
        pistes: 0,
        ecoute: "",
    },
];

/// The last track played before the current one (`vedette` in HomeRecents).
pub static VEDETTE: Fiche = Fiche {
    titre: "Smells Like Teen Spirit",
    artiste: "Nirvana",
    album: "Incesticide",
    annee: "1992",
    duree: 301.,
    pochette: "incesticide",
};
pub static VEDETTE_ECOUTE: &str = "il y a 2 h";

/// `groupByAlbum(historique).slice(1, 6)`: albums played before.
pub static BANDE: [Album; 5] = [
    Album {
        id: 11,
        titre: "Jurassic Park",
        artiste: "John Williams",
        annee: "1993",
        pochette: "jurassic",
        pistes: 12,
        ecoute: "il y a 5 h",
    },
    Album {
        id: 12,
        titre: "The Lord of the Rings",
        artiste: "Howard Shore",
        annee: "2001",
        pochette: "lotr",
        pistes: 4,
        ecoute: "il y a 1 j",
    },
    Album {
        id: 13,
        titre: "Doggystyle",
        artiste: "Snoop Dogg",
        annee: "1993",
        pochette: "snoop",
        pistes: 7,
        ecoute: "il y a 2 j",
    },
    Album {
        id: 14,
        titre: "Crush",
        artiste: "Bon Jovi",
        annee: "2000",
        pochette: "bonjovi",
        pistes: 3,
        ecoute: "il y a 3 j",
    },
    Album {
        id: 15,
        titre: "Brand New Day",
        artiste: "Sting",
        annee: "1999",
        pochette: "sting",
        pistes: 2,
        ecoute: "il y a 4 j",
    },
];

/// `trackCount()` of HomeRecents.svelte.
pub fn track_count(n: u32) -> String {
    if n == 1 {
        "1 titre".into()
    } else {
        format!("{n} titres")
    }
}

/// Actions of a daily mix (`actions/mix/MixAction.ts`): icon, label.
pub static MIX_ACTIONS: [(&str, &str); 2] = [
    ("material-symbols:refresh-rounded", "Nouveau tirage"),
    (
        "material-symbols:playlist-add-rounded",
        "Garder en playlist",
    ),
];
