//! Un fichier de thème suivi pendant que l'application tourne (KUI-050) : relu dès qu'il
//! change sur le disque, pour que la couleur ou la taille de texte modifiée s'applique à
//! l'image suivante, sans redémarrer.
use super::EditorPalette;
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

/// A theme file in JSON (see [`EditorPalette::with_json`]) watched while the application
/// runs: [`poll`](Self::poll) reads it again when its modification time or its size
/// changed, and gives the new palette, laid over `base`. Call it from a timer or before
/// building the view; rebuild when it returns a palette.
///
/// ```no_run
/// use karbon_ui::ui::editor::{EditorPalette, ThemeWatch};
///
/// let mut theme = ThemeWatch::new("theme.json", EditorPalette::DARK);
/// if let Some(Ok(palette)) = theme.poll() {
///     // build the next view with `palette`
///     # let _ = palette;
/// }
/// ```
pub struct ThemeWatch {
    path: PathBuf,
    base: EditorPalette,
    /// Date et taille lues la dernière fois (`None` : fichier absent).
    seen: Option<Option<(SystemTime, u64)>>,
}

impl ThemeWatch {
    /// Watch the file at `path`, whose values change `base`. Nothing is read yet: the
    /// first [`poll`](Self::poll) reads it.
    pub fn new(path: impl Into<PathBuf>, base: EditorPalette) -> Self {
        Self {
            path: path.into(),
            base,
            seen: None,
        }
    }

    /// The watched file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `None` when the file did not change since the last call. Otherwise the palette it
    /// now gives (`base` when the file was removed), or the reason it cannot be read (the
    /// application keeps its palette and shows the reason).
    pub fn poll(&mut self) -> Option<Result<EditorPalette, String>> {
        let stamp = std::fs::metadata(&self.path)
            .ok()
            .map(|m| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()));
        if self.seen == Some(stamp) {
            return None;
        }
        self.seen = Some(stamp);
        if stamp.is_none() {
            return Some(Ok(self.base));
        }
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) => return Some(Err(format!("{}: {error}", self.path.display()))),
        };
        Some(self.base.with_json(&text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Color;

    #[test]
    fn a_changed_theme_file_is_read_again_and_an_unchanged_one_is_not() {
        let folder = std::env::temp_dir().join(format!("karbon-theme-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("theme.json");
        let mut watch = ThemeWatch::new(&path, EditorPalette::DARK);
        // Pas de fichier : la palette de base, une fois.
        assert_eq!(watch.poll(), Some(Ok(EditorPalette::DARK)));
        assert_eq!(watch.poll(), None);
        std::fs::write(&path, r##"{ "accent": "#FF8800" }"##).unwrap();
        let first = watch.poll().unwrap().unwrap();
        assert_eq!(first.accent, Color::hex(0xFF8800));
        // Témoin : rien n'a changé, rien n'est relu.
        assert_eq!(watch.poll(), None);
        // Une autre taille de fichier suffit à le relire, même dans la même seconde.
        std::fs::write(&path, r##"{ "accent": "#00FF88", "font_size": 14 }"##).unwrap();
        let second = watch.poll().unwrap().unwrap();
        assert_eq!(
            (second.accent, second.font_size),
            (Color::hex(0x00FF88), 14.)
        );
        std::fs::write(&path, "{ \"font_size\": 99 }").unwrap();
        assert!(matches!(watch.poll(), Some(Err(_))));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
