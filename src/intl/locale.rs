//! Langue de mise en forme : celle fixée par l'application (une valeur pour tout le
//! processus, lue par les fonctions libres du module), et les données propres à chaque
//! langue (séparateurs, unités, mots du temps relatif, mois).
use std::sync::atomic::{AtomicU8, Ordering};

/// Language used to format numbers, durations and dates, like a BCP 47 tag given to
/// the web's `Intl` API.
///
/// The formatting methods are also available as free functions of [`crate::intl`],
/// which use the locale set with [`set_locale`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Locale {
    /// English (United States conventions: `9,440`, `Nov 7, 2026, 4:20 AM`).
    #[default]
    English,
    /// French (`9 440`, `7 nov. 2026, 04:20`).
    French,
    /// German (`9.440`, `07.11.2026, 04:20`).
    German,
    /// Spanish (`9440`, `12.345`, `7 nov 2026, 4:20`).
    Spanish,
    /// Italian (`9.440`, `7 nov 2026, 04:20`).
    Italian,
}

impl Locale {
    /// Every supported locale.
    pub const ALL: [Self; 5] = [
        Self::English,
        Self::French,
        Self::German,
        Self::Spanish,
        Self::Italian,
    ];
    /// Locale of a language tag: `"fr"`, `"fr-CA"`, `"de_DE.UTF-8"`… The language part
    /// decides; an unsupported language gives [`Locale::English`].
    ///
    /// ```
    /// use karbon_ui::intl::Locale;
    ///
    /// assert_eq!(Locale::parse("fr-FR"), Locale::French);
    /// assert_eq!(Locale::parse("de_DE.UTF-8"), Locale::German);
    /// assert_eq!(Locale::parse("ja"), Locale::English);
    /// ```
    pub fn parse(tag: &str) -> Self {
        let language = tag
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match language.as_str() {
            "fr" => Self::French,
            "de" => Self::German,
            "es" => Self::Spanish,
            "it" => Self::Italian,
            _ => Self::English,
        }
    }
    /// Two-letter language code (`"en"`, `"fr"`…).
    pub fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
            Self::German => "de",
            Self::Spanish => "es",
            Self::Italian => "it",
        }
    }
    pub(super) fn data(self) -> &'static Data {
        match self {
            Self::English => &ENGLISH,
            Self::French => &FRENCH,
            Self::German => &GERMAN,
            Self::Spanish => &SPANISH,
            Self::Italian => &ITALIAN,
        }
    }
    fn index(self) -> u8 {
        Self::ALL.iter().position(|l| *l == self).unwrap_or(0) as u8
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Sets the locale used by the free functions of [`crate::intl`], for the whole process
/// (call it at start-up and when the user changes the language).
pub fn set_locale(locale: Locale) {
    CURRENT.store(locale.index(), Ordering::Relaxed);
}
/// Locale set with [`set_locale`]; [`Locale::English`] until then.
pub fn locale() -> Locale {
    Locale::ALL[usize::from(CURRENT.load(Ordering::Relaxed)) % Locale::ALL.len()]
}

/// Grammatical number of a count, to choose between two translations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Plural {
    /// Singular form (`1 track`; in French also `0 titre`).
    One,
    /// Plural form.
    Other,
}

/// Mot singulier et pluriel.
pub(super) type Word = (&'static str, &'static str);

/// Données d'une langue, reprises des formats courts et moyens de CLDR.
pub(super) struct Data {
    pub group: &'static str,
    pub decimal: char,
    /// Chiffres à partir desquels on groupe : 4 (`9,440`), ou 5 en espagnol (`9440`).
    pub min_grouping: usize,
    /// Jours, heures, minutes d'une durée (`25 j 6 h`, `3 h 05 min`).
    pub duration_units: [&'static str; 3],
    /// Temps relatif long : modèle (`{}` = nombre et unité), « maintenant », « hier »,
    /// minutes, heures, jours, mois, ans.
    pub ago: &'static str,
    pub now: &'static str,
    pub yesterday: &'static str,
    pub units: [Word; 5],
    /// Temps relatif court : « à l'instant », puis minutes, heures, jours abrégés.
    pub just_now: &'static str,
    pub short_units: [&'static str; 3],
    pub months: [&'static str; 12],
    /// Pluriel dès 2 (français) plutôt que dès 0 et 2 (autres langues).
    pub zero_is_singular: bool,
}

const ENGLISH: Data = Data {
    group: ",",
    decimal: '.',
    min_grouping: 4,
    duration_units: ["d", "h", "min"],
    ago: "{} ago",
    now: "now",
    yesterday: "yesterday",
    units: [
        ("minute", "minutes"),
        ("hour", "hours"),
        ("day", "days"),
        ("month", "months"),
        ("year", "years"),
    ],
    just_now: "just now",
    short_units: ["min", "h", "d"],
    months: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    zero_is_singular: false,
};
const FRENCH: Data = Data {
    group: "\u{202f}",
    decimal: ',',
    min_grouping: 4,
    duration_units: ["j", "h", "min"],
    ago: "il y a {}",
    now: "maintenant",
    yesterday: "hier",
    units: [
        ("minute", "minutes"),
        ("heure", "heures"),
        ("jour", "jours"),
        ("mois", "mois"),
        ("an", "ans"),
    ],
    just_now: "à l’instant",
    short_units: ["min", "h", "j"],
    months: [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
        "déc.",
    ],
    zero_is_singular: true,
};
const GERMAN: Data = Data {
    group: ".",
    decimal: ',',
    min_grouping: 4,
    duration_units: ["T.", "Std.", "Min."],
    ago: "vor {}",
    now: "jetzt",
    yesterday: "gestern",
    units: [
        ("Minute", "Minuten"),
        ("Stunde", "Stunden"),
        ("Tag", "Tagen"),
        ("Monat", "Monaten"),
        ("Jahr", "Jahren"),
    ],
    just_now: "gerade eben",
    short_units: ["Min.", "Std.", "T."],
    months: [
        "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.",
        "Dez.",
    ],
    zero_is_singular: false,
};
const SPANISH: Data = Data {
    group: ".",
    decimal: ',',
    min_grouping: 5,
    duration_units: ["d", "h", "min"],
    ago: "hace {}",
    now: "ahora",
    yesterday: "ayer",
    units: [
        ("minuto", "minutos"),
        ("hora", "horas"),
        ("día", "días"),
        ("mes", "meses"),
        ("año", "años"),
    ],
    just_now: "justo ahora",
    short_units: ["min", "h", "d"],
    months: [
        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
    ],
    zero_is_singular: false,
};
const ITALIAN: Data = Data {
    group: ".",
    decimal: ',',
    min_grouping: 4,
    duration_units: ["g", "h", "min"],
    ago: "{} fa",
    now: "ora",
    yesterday: "ieri",
    units: [
        ("minuto", "minuti"),
        ("ora", "ore"),
        ("giorno", "giorni"),
        ("mese", "mesi"),
        ("anno", "anni"),
    ],
    just_now: "proprio ora",
    short_units: ["min", "h", "g"],
    months: [
        "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
    ],
    zero_is_singular: false,
};
