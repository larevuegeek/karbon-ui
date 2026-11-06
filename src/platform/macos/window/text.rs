//! Saisie de texte par `NSTextInputClient` : texte inséré, texte marqué (touches mortes,
//! IME, accents par appui long) et position du caret pour la fenêtre des candidats.
//!
//! `keyDown:` confie à `interpretKeyEvents:` tout ce qui n'est pas une touche ou un
//! raccourci du moteur ; AppKit rappelle alors ces fonctions. Une composition suit la
//! séquence du moteur : `CompositionStart`, `CompositionUpdate`…, `CompositionCommit`,
//! `CompositionEnd`.
use super::{session::Session, with};
use crate::window::WindowId;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSAttributedString, NSNotFound, NSRange, NSRect, NSString};

/// Chaîne d'un argument AppKit (`NSString` ou `NSAttributedString`).
pub(super) fn string(object: &AnyObject) -> String {
    if let Some(attributed) = object.downcast_ref::<NSAttributedString>() {
        return attributed.string().to_string();
    }
    object
        .downcast_ref::<NSString>()
        .map(|s| s.to_string())
        .unwrap_or_default()
}
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}
pub(super) fn insert(id: WindowId, text: String, replacement: NSRange) {
    // Accent choisi par appui long : AppKit remplace le caractère déjà inséré.
    let replace = if replacement.location != NSNotFound as usize && replacement.length > 0 {
        replacement.length
    } else {
        0
    };
    with(id, |s| s.insert_text(text, replace));
}
pub(super) fn set_marked(id: WindowId, text: String, selected: NSRange) {
    let cursor = if selected.location == NSNotFound as usize {
        utf16_len(&text)
    } else {
        selected.location.min(utf16_len(&text))
    };
    with(id, |s| s.set_marked(text, cursor));
}
pub(super) fn unmark(id: WindowId) {
    with(id, |s| s.unmark());
}
pub(super) fn marked_range(id: WindowId) -> NSRange {
    match with(id, |s| Ok(s.marked().map(utf16_len))).flatten() {
        Some(length) => NSRange::new(0, length),
        None => NSRange::new(NSNotFound as usize, 0),
    }
}
pub(super) fn selected_range(id: WindowId) -> NSRange {
    let length = with(id, |s| Ok(s.marked().map(utf16_len)))
        .flatten()
        .unwrap_or(0);
    NSRange::new(length, 0)
}
/// Caret du champ focalisé, dans le repère de la vue (y vers le bas).
pub(super) fn caret(id: WindowId) -> Option<NSRect> {
    let r = with(id, |s| Ok(s.caret())).flatten()?;
    Some(NSRect::new(
        objc2_foundation::NSPoint::new(f64::from(r.left), f64::from(r.top)),
        objc2_foundation::NSSize::new(
            f64::from(r.right - r.left).max(1.),
            f64::from(r.bottom - r.top).max(1.),
        ),
    ))
}

impl Session {
    /// Texte marqué de la composition en cours.
    pub fn marked(&self) -> Option<&str> {
        self.marked.as_deref()
    }
    /// Rectangle du caret du champ focalisé (repère de la vue, y vers le bas).
    pub fn caret(&self) -> Option<crate::ui::scene::RectF> {
        self.runtime.focused_input_geometry().map(|g| g.caret)
    }
    /// Texte marqué (touche morte, IME) : début, mise à jour ou annulation (texte vide)
    /// de la composition. Sans champ focalisé, rien n'est composé.
    pub fn set_marked(&mut self, text: String, cursor_utf16: usize) -> crate::Result<()> {
        if text.is_empty() {
            if self.marked.take().is_some() {
                self.input(crate::ui::runtime::UiEvent::CompositionEnd)?;
            }
            return Ok(());
        }
        if self.runtime.focused_input_geometry().is_none() {
            return Ok(());
        }
        if self.marked.is_none() {
            self.input(crate::ui::runtime::UiEvent::CompositionStart)?;
        }
        self.marked = Some(text.clone());
        self.input(crate::ui::runtime::UiEvent::CompositionUpdate {
            value: text.into(),
            cursor_utf16,
        })
        .map(drop)
    }
    /// Texte validé : fin de la composition en cours, ou texte saisi directement.
    /// `replace` caractères précédents sont d'abord effacés (accent choisi par appui long).
    pub fn insert_text(&mut self, text: String, replace: usize) -> crate::Result<()> {
        if self.marked.take().is_some() {
            self.input(crate::ui::runtime::UiEvent::CompositionCommit(text.into()))?;
            return self
                .input(crate::ui::runtime::UiEvent::CompositionEnd)
                .map(drop);
        }
        for _ in 0..replace {
            let key = crate::ui::runtime::Key::Backspace;
            self.input(crate::ui::runtime::UiEvent::KeyDown {
                key,
                shift: false,
                repeat: false,
            })?;
            self.input(crate::ui::runtime::UiEvent::KeyUp(key))?;
        }
        self.input(crate::ui::runtime::UiEvent::Text(text.into()))
            .map(drop)
    }
    /// Validation du texte marqué (perte du focus, clic ailleurs).
    pub fn unmark(&mut self) -> crate::Result<()> {
        match self.marked.take() {
            Some(text) => {
                self.input(crate::ui::runtime::UiEvent::CompositionCommit(text.into()))?;
                self.input(crate::ui::runtime::UiEvent::CompositionEnd)
                    .map(drop)
            }
            None => Ok(()),
        }
    }
}
