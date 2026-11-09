//! Préférences de couleurs effectives pendant la construction de la vue.
use std::cell::Cell;
thread_local! {
    static SCHEME: Cell<(bool, bool)> = const { Cell::new((false, false)) };
}
/// Effective colour preferences while a view is built, as `(dark, high_contrast)`: the
/// values matched by `prefers-color-scheme` and `prefers-contrast`, with
/// [`ColorScheme::System`](crate::application::ColorScheme::System) resolved.
pub fn color_scheme() -> (bool, bool) {
    SCHEME.with(Cell::get)
}
pub(crate) fn set_color_scheme(value: (bool, bool)) {
    SCHEME.with(|s| s.set(value));
}
