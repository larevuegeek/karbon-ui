//! Lecteur : morceau chargé, ligne d’état, panneau « Ensuite », sorties audio et réseau.

/// The track loaded in the player (`$player`) and shown by the resume hero.
pub struct Fiche {
    pub titre: &'static str,
    pub artiste: &'static str,
    pub album: &'static str,
    pub annee: &'static str,
    pub duree: f64,
    pub pochette: &'static str,
}
pub static FICHE: Fiche = Fiche {
    titre: "Stairway To Heaven",
    artiste: "Led Zeppelin",
    album: "Led Zeppelin IV (Deluxe Edition)",
    annee: "1971",
    duree: 482.83,
    pochette: "zeppelin4",
};

/// Status line of the player (format, quality, rate · size, path, output).
pub struct Statut {
    pub format: &'static str,
    pub qualite: &'static str,
    /// `→ 48 kHz` when the output resamples; empty otherwise.
    pub sortie: &'static str,
    pub debit: &'static str,
    pub dossier: &'static str,
    pub fichier: &'static str,
}
pub static STATUT: Statut = Statut {
    format: "FLAC",
    qualite: "16 bit · 44,1 kHz",
    sortie: "",
    debit: "833 kb/s · 48 Mo",
    dossier: r"D:\Music\FLAC\Led Zeppelin\Led Zeppelin - 1971 - Led Zeppelin IV (Deluxe Edition)\",
    fichier: "4 Stairway To Heaven.flac",
};

/// A card of the "Ensuite" panel (`Carte` in HomeResume.svelte).
pub struct Carte {
    pub cle: &'static str,
    pub titre: &'static str,
    pub artiste: &'static str,
    pub duree: f64,
    pub pochette: &'static str,
}
pub static ENSUITE: [Carte; 4] = [
    Carte {
        cle: "f:1",
        titre: "Hornbeam",
        artiste: "Neneh Cherry",
        duree: 302.53,
        pochette: "man",
    },
    Carte {
        cle: "f:2",
        titre: "The National Anthem",
        artiste: "Radiohead",
        duree: 347.37,
        pochette: "radiohead_best2",
    },
    Carte {
        cle: "f:3",
        titre: "Strategie D'un Pion",
        artiste: "IAM",
        duree: 274.79,
        pochette: "iam_anthologie",
    },
    Carte {
        cle: "f:4",
        titre: "Andy Warhol (Live at New Haven Veterans Memorial Coliseum, New Haven, CT, 8/23/1994) (2019 Remaster)",
        artiste: "Stone Temple Pilots",
        duree: 153.6,
        pochette: "stp_purple",
    },
];

/// An audio output (`describeOutput` of `helper/audio/deviceLabel.ts`): name, detail,
/// icon, Hi-Res tier (else "Qualité CD").
pub struct Sortie {
    pub nom: &'static str,
    pub detail: &'static str,
    pub icone: &'static str,
    pub hires: bool,
}
pub static SORTIES: [Sortie; 4] = [
    Sortie {
        nom: "Pebble V3",
        detail: "Haut-parleurs · USB",
        icone: "material-symbols:speaker-outline-rounded",
        hires: false,
    },
    Sortie {
        nom: "PRO X 2 LIGHTSPEED",
        detail: "Haut-parleurs · USB",
        icone: "material-symbols:speaker-outline-rounded",
        hires: false,
    },
    Sortie {
        nom: "LG HDR 4K",
        detail: "NVIDIA High Definition Audio · HDMI",
        icone: "material-symbols:tv-outline-rounded",
        hires: false,
    },
    Sortie {
        nom: "Realtek Digital Output",
        detail: "Realtek(R) Audio · S/PDIF",
        icone: "material-symbols:speaker-outline-rounded",
        hires: true,
    },
];

/// Network renderers found (AirPlay): name, detail.
pub static LECTEURS_RESEAU: [(&str, &str); 2] = [
    ("Bureau", "AirPlay · AppleTV14,1"),
    ("ZimaOS AirPlay", "AirPlay · ShairportSync"),
];

/// Services of `player/NetworkMenu.svelte`: icon, title.
pub static SERVICES_RESEAU: [(&str, &str); 3] = [
    ("material-symbols-light:cast-rounded", "Lecteurs UPnP/DLNA"),
    ("material-symbols-light:airplay-rounded", "AirPlay (v1)"),
    (
        "material-symbols-light:folder-shared-outline-rounded",
        "Serveur DLNA",
    ),
];
