//! Locale-aware formatting of numbers, durations, relative times and dates, like the
//! web's `Intl` API, for the five built-in languages of [`Locale`].
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
//! assert_eq!(Locale::English.relative_time(8 * 60), "8 minutes ago");
//! # intl::set_locale(Locale::English);
//! ```
//!
//! Words that belong to the application (`12 tracks`) stay in its translations; use
//! [`plural`] to choose between the singular and plural forms.
// Organisation : `locale` (langue courante et données de chaque langue), `format`
// (nombres, horloges, durées, temps écoulé, pluriel), `date` (heure locale du système et
// dates), `current` (fonctions libres dans la langue courante).
mod current;
mod date;
mod format;
mod locale;
#[cfg(test)]
mod tests;
pub use current::*;
pub use date::DateTime;
pub use locale::{Locale, Plural, locale, set_locale};
