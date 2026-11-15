//! Le catalogue du processus et les fonctions de traduction dans la langue courante.
use super::{Catalog, Plural, locale, number, plural};
use std::sync::{Arc, RwLock};

static CATALOG: RwLock<Option<Arc<Catalog>>> = RwLock::new(None);

/// Installs the translations used by [`t`], [`t_with`] and [`t_count`], for the whole
/// process (once at start-up, before the first view).
pub fn set_catalog(catalog: Catalog) {
    *CATALOG.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(catalog));
}

/// The catalog installed with [`set_catalog`], if any.
pub fn catalog() -> Option<Arc<Catalog>> {
    CATALOG.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Text of `key` in the current [`locale`], else in the catalog's fallback language,
/// else the key itself (also without a catalog), like `$t` in web frameworks.
///
/// ```
/// use karbon_ui::intl::{self, Catalog, Locale};
///
/// let mut catalog = Catalog::new();
/// catalog.add(Locale::English, r#"{"player": {"next": "Next"}}"#).unwrap();
/// catalog.add(Locale::French, r#"{"player": {"next": "Suivant"}}"#).unwrap();
/// intl::set_catalog(catalog);
/// intl::set_locale(Locale::French);
/// assert_eq!(intl::t("player.next"), "Suivant");
/// # intl::set_locale(Locale::English);
/// ```
pub fn t(key: &str) -> String {
    match catalog() {
        Some(catalog) => catalog.text(locale(), key),
        None => key.to_string(),
    }
}

/// [`t`] with each `{name}` of the text replaced by its value.
///
/// ```
/// use karbon_ui::intl::{self, Catalog, Locale};
///
/// let mut catalog = Catalog::new();
/// catalog.add(Locale::English, r#"{"album": {"by": "Other albums by {a}"}}"#).unwrap();
/// intl::set_catalog(catalog);
/// assert_eq!(intl::t_with("album.by", &[("a", "Björk")]), "Other albums by Björk");
/// ```
pub fn t_with(key: &str, values: &[(&str, &str)]) -> String {
    let mut text = t(key);
    for (name, value) in values {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// The singular (`one`) or plural (`other`) text for `count` in the current language,
/// with `{n}` replaced by the count in the locale's digits: `1 track`, `9,440 tracks`.
///
/// ```
/// use karbon_ui::intl::{self, Catalog, Locale};
///
/// let mut catalog = Catalog::new();
/// catalog.add(Locale::English, r#"{"track_one": "{n} track", "tracks_n": "{n} tracks"}"#).unwrap();
/// intl::set_catalog(catalog);
/// intl::set_locale(Locale::English);
/// assert_eq!(intl::t_count(9440, "track_one", "tracks_n"), "9,440 tracks");
/// assert_eq!(intl::t_count(1, "track_one", "tracks_n"), "1 track");
/// ```
pub fn t_count(count: i64, one: &str, other: &str) -> String {
    let key = if plural(count) == Plural::One {
        one
    } else {
        other
    };
    t_with(key, &[("n", &number(count))])
}
