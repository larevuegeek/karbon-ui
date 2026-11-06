//! En-tête : menu du profil et index de la recherche.

/// Items of the profile menu (`header/ProfilSelectorInput.svelte`): icon, label.
pub static PROFIL_MENU: [(&str, &str); 5] = [
    ("lucide:users", "Changer de profil"),
    ("lucide:bar-chart-3", "Statistiques"),
    ("lucide:settings", "Paramètres"),
    ("lucide:message-circle", "Feedback"),
    ("lucide:info", "À propos"),
];

/// Kind of a search result (`result_type`): badge and fallback icon (`getTypeLabel`,
/// `getIcon` of SearchAutocomplete.svelte).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeResultat {
    Morceau,
    Album,
    Artiste,
    Compositeur,
}
impl TypeResultat {
    pub fn libelle(self) -> &'static str {
        match self {
            Self::Morceau => "Morceau",
            Self::Album => "Album",
            Self::Artiste => "Artiste",
            Self::Compositeur => "Compositeur",
        }
    }
    pub fn icone(self) -> &'static str {
        match self {
            Self::Morceau => "lucide:music",
            Self::Album => "lucide:disc-album",
            Self::Artiste => "lucide:mic-2",
            Self::Compositeur => "material-symbols:stylus-note-rounded",
        }
    }
}
/// A search result: title, subtitle (empty for artists and composers), kind, cover.
pub struct Resultat {
    pub titre: &'static str,
    pub sous_titre: &'static str,
    pub genre: TypeResultat,
    pub pochette: Option<&'static str>,
}
const fn resultat(
    titre: &'static str,
    sous_titre: &'static str,
    genre: TypeResultat,
    pochette: Option<&'static str>,
) -> Resultat {
    Resultat {
        titre,
        sous_titre,
        genre,
        pochette,
    }
}

/// The library index searched by the field: the results of « que » in the capture
/// (the backend search is replaced by a substring match, accents ignored).
pub static RESULTATS: [Resultat; 12] = [
    resultat(
        "Kings and Queens",
        "30 Seconds to Mars",
        TypeResultat::Morceau,
        Some("s_kings_30stm"),
    ),
    resultat(
        "Down With The Clique",
        "Aaliyah",
        TypeResultat::Morceau,
        Some("s_aaliyah_age"),
    ),
    resultat(
        "Kings And Queens",
        "Aerosmith",
        TypeResultat::Morceau,
        Some("s_aerosmith"),
    ),
    resultat(
        "Meteque et Mat",
        "Akhenaton",
        TypeResultat::Morceau,
        Some("s_meteque"),
    ),
    resultat(
        "Que du love",
        "Angèle",
        TypeResultat::Morceau,
        Some("s_angele"),
    ),
    resultat(
        "Meteque et Mat",
        "Akhenaton",
        TypeResultat::Album,
        Some("s_meteque"),
    ),
    resultat(
        "Mes Courants Electriques...",
        "Alizée",
        TypeResultat::Album,
        Some("s_alizee"),
    ),
    resultat(
        "Quelques Gouttes Suffisent...",
        "Arsenik",
        TypeResultat::Album,
        None,
    ),
    resultat(
        "Nicky Jam & Enrique Iglesias",
        "",
        TypeResultat::Artiste,
        None,
    ),
    resultat("Melissa Musique", "", TypeResultat::Artiste, None),
    resultat("Quentin Mosimann", "", TypeResultat::Compositeur, None),
    resultat("Consuelo Velázquez", "", TypeResultat::Compositeur, None),
];
