//! Positions des pièces du balisage et fichiers lus, pour situer les diagnostics.
use crate::Error;
use proc_macro::Span;
use std::cell::RefCell;

thread_local! {
    /// Fichiers lus par l'invocation de macro en cours (index = `Loc::file`).
    static FILES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
/// Enregistre un nom de fichier pour les diagnostics et renvoie son index.
pub fn register_file(display: &str) -> u16 {
    FILES.with(|f| {
        let mut f = f.borrow_mut();
        if let Some(i) = f.iter().position(|d| d == display) {
            return i as u16;
        }
        f.push(display.to_owned());
        (f.len() - 1) as u16
    })
}
/// Position d'une pièce : span pour rustc, ligne/colonne pour les fichiers `.ui`.
#[derive(Clone, Copy)]
pub struct Loc {
    pub span: Span,
    /// 0 pour un `ui!` en ligne (rustc situe déjà le span).
    pub line: u32,
    pub col: u32,
    pub file: u16,
}
impl Loc {
    pub fn error(&self, message: impl std::fmt::Display) -> Error {
        if self.line == 0 {
            return Error::new(message.to_string(), self.span);
        }
        let file = FILES.with(|f| f.borrow().get(self.file as usize).cloned());
        Error::new(
            format!(
                "{}:{}:{}: {message}",
                file.unwrap_or_default(),
                self.line,
                self.col
            ),
            self.span,
        )
    }
}
