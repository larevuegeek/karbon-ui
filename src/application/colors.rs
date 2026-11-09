//! Préférences de couleurs d'une application, lues par `prefers-color-scheme` et
//! `prefers-contrast` des feuilles de styles.

/// Colour scheme used by the `prefers-color-scheme` media feature.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    /// Follow the system setting (light where the backend cannot read it).
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// What [`Application::color_preferences`](super::Application::color_preferences) reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColorPreferences {
    /// Scheme matched by `prefers-color-scheme`.
    pub scheme: ColorScheme,
    /// `prefers-contrast: more` matches.
    pub high_contrast: bool,
}
