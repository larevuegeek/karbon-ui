//! Translations and locale-aware formatting of numbers, sizes, durations, relative times
//! and dates, like the web's `Intl` API, for the five built-in languages of [`Locale`].
//!
//! Set the language once (at start-up, then whenever the user changes it), and format
//! with the free functions; each one is also a method of [`Locale`] for an explicit
//! language.
//!
//! ```
//! use karbon_ui::intl::{self, Locale};
//!
//! intl::set_locale(Locale::parse("fr-FR"));
//! assert_eq!(intl::number(9440), "9\u{202f}440");
//! assert_eq!(intl::duration(3. * 3600. + 13. * 60.), "3 h 13 min");
//! assert_eq!(intl::relative_time(8 * 60), "il y a 8 minutes");
//! assert_eq!(intl::bytes(12 * 1024 * 1024), "12 Mo");
//! assert_eq!(Locale::English.relative_time(8 * 60), "8 minutes ago");
//! # intl::set_locale(Locale::English);
//! ```
//!
//! Words that belong to the application (`12 tracks`) come from its translations: a
//! [`Catalog`] read from JSON files, installed with [`set_catalog`] and used by [`t`],
//! [`t_with`] and [`t_count`].
// Organisation : `locale` (langue courante et données de chaque langue), `format`
// (nombres, tailles, horloges, durées, temps écoulé, pluriel), `date` (heure locale du
// système et dates), `current` (fonctions libres dans la langue courante), `catalog`
// (tables de traductions), `translate` (catalogue du processus et `t`).
mod catalog;
mod current;
mod date;
mod format;
mod locale;
#[cfg(test)]
mod tests;
mod translate;
pub use catalog::Catalog;
pub use current::*;
pub use date::DateTime;
pub use locale::{Locale, Plural, locale, set_locale};
pub use translate::{catalog, set_catalog, t, t_count, t_with};
