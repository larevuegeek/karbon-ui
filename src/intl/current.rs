//! Raccourcis des méthodes de [`Locale`] dans la langue fixée par l'application.
use super::{Locale, Plural, locale};
use std::time::SystemTime;

/// [`Locale::number`] in the current locale: `9,440`, `9 440`, `9.440`.
pub fn number(value: i64) -> String {
    locale().number(value)
}
/// [`Locale::decimal`] in the current locale: `44.1`, `44,1`.
pub fn decimal(value: f64, fraction_digits: usize) -> String {
    locale().decimal(value, fraction_digits)
}
/// [`Locale::clock`]: `8:02`, `1:02:03`.
pub fn clock(seconds: f64) -> String {
    Locale::English.clock(seconds)
}
/// [`Locale::clock_padded`]: `08:02`, `1:02:03`.
pub fn clock_padded(seconds: f64) -> String {
    Locale::English.clock_padded(seconds)
}
/// [`Locale::duration`] in the current locale: `3 h 05 min`.
pub fn duration(seconds: f64) -> String {
    locale().duration(seconds)
}
/// [`Locale::relative_time`] in the current locale: `8 minutes ago`, `yesterday`.
pub fn relative_time(elapsed_seconds: i64) -> String {
    locale().relative_time(elapsed_seconds)
}
/// [`Locale::relative_time_short`] in the current locale: `3 min ago`.
pub fn relative_time_short(elapsed_seconds: i64) -> String {
    locale().relative_time_short(elapsed_seconds)
}
/// Seconds elapsed from `then` to now (negative for a future time), for
/// [`relative_time`].
pub fn seconds_since(then: SystemTime) -> i64 {
    match SystemTime::now().duration_since(then) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}
/// [`Locale::date_time`] in the current locale and the computer's time zone.
pub fn date_time(time: SystemTime) -> String {
    locale().date_time(time)
}
/// [`Locale::plural`] in the current locale.
pub fn plural(count: i64) -> Plural {
    locale().plural(count)
}
