//! Menus, popins and panels of the home page: which one is open, and the static
//! choices they show.
use super::{MusicApp, minutes_seconds};

/// Sleep timer mode (`sleepTimer.store`): a duration or the end of the track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Veille {
    Minutes(u32),
    FinMorceau,
}

impl MusicApp {
    /// `formatSleepRemaining` of the remaining time (the timer does not run here).
    pub fn veille_reste(&self) -> Option<String> {
        match self.veille {
            Some(Veille::Minutes(m)) => Some(minutes_seconds(f64::from(m) * 60.)),
            _ => None,
        }
    }
    /// A sidebar playlist row is shown: "Titres likés" and "Récemment joués" follow their
    /// shortcut toggle (`PlaylistZoneMenu`).
    pub fn raccourci_visible(&self, nom: &str) -> bool {
        super::data::RACCOURCIS
            .iter()
            .zip(self.raccourcis)
            .all(|((titre, _), visible)| visible || *titre != nom)
    }
    /// Search results for the field (two characters at least, like the Svelte): the
    /// static index filtered by substring, case and accents ignored.
    pub fn resultats(&self) -> Vec<&'static super::data::Resultat> {
        fn plier(texte: &str) -> String {
            texte
                .chars()
                .flat_map(char::to_lowercase)
                .map(|c| match c {
                    'à' | 'â' | 'ä' | 'á' => 'a',
                    'é' | 'è' | 'ê' | 'ë' => 'e',
                    'î' | 'ï' | 'í' => 'i',
                    'ô' | 'ö' | 'ó' => 'o',
                    'ù' | 'û' | 'ü' | 'ú' => 'u',
                    'ç' => 'c',
                    _ => c,
                })
                .collect()
        }
        let requete = plier(self.search.trim());
        if requete.chars().count() < 2 {
            return Vec::new();
        }
        super::data::RESULTATS
            .iter()
            .filter(|r| plier(r.titre).contains(&requete) || plier(r.sous_titre).contains(&requete))
            .collect()
    }
    /// Title of the "Réseau" pill: the services that run.
    pub fn reseau_resume(&self) -> String {
        let actifs = super::data::SERVICES_RESEAU
            .iter()
            .zip(self.reseau)
            .filter(|(_, actif)| *actif)
            .map(|(service, _)| service.1)
            .collect::<Vec<_>>();
        actifs.join(" · ")
    }
    /// Status line of a network service (`statusLabel` of NetworkMenu.svelte).
    pub fn reseau_statut(&self, i: usize) -> &'static str {
        match (i, self.reseau[i]) {
            (_, false) => "Désactivé",
            (2, true) => "En ligne · 10.0.0.121:8200",
            _ => "Prêt : à choisir dans Sortie audio",
        }
    }
    /// Title of the sleep timer button and subtitle of its panel.
    pub fn veille_titre(&self) -> String {
        match (self.veille, self.veille_reste()) {
            (Some(Veille::FinMorceau), _) => "Arrêt en fin de morceau".into(),
            (_, Some(reste)) => format!("Veille dans {reste}"),
            _ => "Minuteur de veille".into(),
        }
    }
}

/// Menus, popins and panels reachable from the home page (one open at a time, like
/// `k-popover`). The variants follow the reference captures (`menu-NN-*.png`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    /// `player/SleepTimerButton.svelte`
    Minuteur,
    /// `header/ProfilSelectorInput.svelte`
    Profil,
    /// `sidebar/SourceSwitcher.svelte`
    Source,
    /// "…" next to Fichier / Dossier (`ui/contextmenu/OpenZoneMenu.svelte`)
    Ouvrir,
    /// `library/common/LibrarySelector.svelte`
    Bibliotheque,
    /// Display options of the sidebar navigation
    Affichage,
    /// "…" of a mix card (`mix/MixCard.svelte`), by index in `data::MIXES`
    Mix(usize),
    /// Audio output (`player/PlayerSoundBar.svelte`)
    Sortie,
    /// Queue panel (`queue/QueuePanel.svelte`)
    File,
    /// Lyrics panel (`lyrics/LyricsPanel.svelte`)
    Paroles,
    /// UPnP/DLNA players (`player/NetworkMenu.svelte`)
    Reseau,
    /// What's new popin (`whatsnew/WhatsNewPopin.svelte`)
    Nouveautes,
    /// Search suggestions (`search/SearchAutocomplete.svelte`)
    Recherche,
}
impl Menu {
    /// Names accepted by `--open` (export with a menu open).
    pub const NAMES: [(&'static str, Menu); 13] = [
        ("sleep", Menu::Minuteur),
        ("profile", Menu::Profil),
        ("source", Menu::Source),
        ("open", Menu::Ouvrir),
        ("library", Menu::Bibliotheque),
        ("display", Menu::Affichage),
        ("mix", Menu::Mix(0)),
        ("output", Menu::Sortie),
        ("queue", Menu::File),
        ("lyrics", Menu::Paroles),
        ("network", Menu::Reseau),
        ("whatsnew", Menu::Nouveautes),
        ("search", Menu::Recherche),
    ];
    /// The menu named `name` in [`Menu::NAMES`].
    pub fn parse(name: &str) -> Option<Menu> {
        Self::NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, menu)| *menu)
    }
}
