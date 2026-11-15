//! Les traductions d'une application : une table par langue, lue dans un fichier JSON
//! (objets imbriqués, clés pointées), avec une langue de repli.
use super::Locale;
use crate::json::Value;
use std::collections::HashMap;

/// The texts of an application in each of its languages, read from JSON files like the
/// ones of `i18next` or `svelte-i18n`: nested objects whose keys join with dots.
///
/// ```json
/// { "nav": { "home": "Home", "tracks": "{n} tracks" } }
/// ```
///
/// gives the keys `nav.home` and `nav.tracks`. A key missing in a language is looked up
/// in the fallback language (the first one added, unless
/// [`set_fallback`](Self::set_fallback) chose another), then shown as is. Install the
/// catalog once with [`set_catalog`](super::set_catalog), then translate with
/// [`t`](super::t), [`t_with`](super::t_with) and [`t_count`](super::t_count) in the
/// current [`locale`](super::locale).
///
/// ```
/// use karbon_ui::intl::{Catalog, Locale};
///
/// let mut catalog = Catalog::new();
/// catalog.add(Locale::French, r#"{"nav": {"home": "Accueil", "back": "Retour"}}"#).unwrap();
/// catalog.add(Locale::English, r#"{"nav": {"home": "Home"}}"#).unwrap();
/// assert_eq!(catalog.text(Locale::English, "nav.home"), "Home");
/// assert_eq!(catalog.text(Locale::English, "nav.back"), "Retour"); // fallback: French
/// assert_eq!(catalog.text(Locale::English, "nav.missing"), "nav.missing");
/// ```
#[derive(Debug, Default, Clone)]
pub struct Catalog {
    tables: Vec<(Locale, HashMap<String, String>)>,
    fallback: Option<Locale>,
}

impl Catalog {
    /// Empty catalog.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds the texts of `locale` from a JSON object, replacing those already added for
    /// it. Strings are kept; numbers, booleans, `null` and arrays are ignored.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) when `json` is not a JSON
    /// object; the catalog is unchanged.
    pub fn add(&mut self, locale: Locale, json: &str) -> crate::Result<()> {
        let Some(value @ Value::Object(_)) = crate::json::parse(json) else {
            return Err(crate::Error::invalid_input(
                "translations must be a JSON object",
            ));
        };
        let mut table = HashMap::new();
        flatten(&value, &mut String::new(), &mut table);
        self.tables.retain(|(l, _)| *l != locale);
        self.tables.push((locale, table));
        self.fallback.get_or_insert(locale);
        Ok(())
    }
    /// Language whose texts stand in for the keys missing in another one.
    pub fn set_fallback(&mut self, locale: Locale) {
        self.fallback = Some(locale);
    }
    /// Languages with texts, in the order they were added.
    pub fn locales(&self) -> Vec<Locale> {
        self.tables.iter().map(|(l, _)| *l).collect()
    }
    /// Text of `key` in `locale`, else in the fallback language.
    pub fn get(&self, locale: Locale, key: &str) -> Option<&str> {
        let find = |wanted: Locale| {
            self.tables
                .iter()
                .find(|(l, _)| *l == wanted)
                .and_then(|(_, table)| table.get(key))
                .map(String::as_str)
        };
        find(locale).or_else(|| self.fallback.and_then(find))
    }
    /// Text of `key` in `locale`, else in the fallback language, else the key itself.
    pub fn text(&self, locale: Locale, key: &str) -> String {
        self.get(locale, key).unwrap_or(key).to_string()
    }
}

/// Feuilles de chaînes de l'arbre, clés jointes par des points.
fn flatten(value: &Value, prefix: &mut String, table: &mut HashMap<String, String>) {
    match value {
        Value::Object(members) => {
            for (key, child) in members {
                let length = prefix.len();
                if !prefix.is_empty() {
                    prefix.push('.');
                }
                prefix.push_str(key);
                flatten(child, prefix, table);
                prefix.truncate(length);
            }
        }
        Value::String(text) => {
            table.insert(prefix.clone(), text.clone());
        }
        _ => {}
    }
}
