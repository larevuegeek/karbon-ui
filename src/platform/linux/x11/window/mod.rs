//! Fenêtre X11 : fenêtre de premier niveau (visuel de la racine, ou ARGB 32 bits pour une
//! fenêtre transparente), propriétés du gestionnaire de fenêtres (titre, classe, PID,
//! taille minimale, fermeture et `_NET_WM_PING`, décorations Motif), taille reçue en
//! pixels, états EWMH (agrandie, plein écran, réduite) et session commune
//! (`shell/session.rs`).
//!
//! Décorations : avec `title_bar`, le gestionnaire dessine la barre de titre (la barre du
//! moteur reste masquée) ; sans, la fenêtre demande à n'en avoir aucune (`_MOTIF_WM_HINTS`),
//! comme une fenêtre sans cadre.
mod change;
mod paint;

pub(super) use paint::shared_gpu;
mod top_level;
pub(crate) use top_level::*;
