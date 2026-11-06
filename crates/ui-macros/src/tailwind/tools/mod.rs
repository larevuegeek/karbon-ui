//! Outils de mesure (tests ignorés), lancés à la main pour régénérer les chiffres de
//! `docs/tailwind.md` ; ils lisent le projet RustMusic (variable `RUSTMUSIC`) et ne
//! vérifient pas le comportement du compilateur.
//!
//! ```text
//! cargo test -p karbon-ui-macros --lib tailwind::tools -- --ignored --nocapture
//! ```
//!
//! - `coverage` : part des classes Tailwind de RustMusic acceptées (rapport Markdown) ;
//! - `oracle` : utilitaires acceptés comparés à la feuille complète générée par Tailwind.
mod coverage;
mod oracle;
