//! Suite native du backend macOS : vraie boucle AppKit sur le thread principal (AppKit
//! l'exige ; d'où une crate de test sans harnais, `harness = false`).
//!
//! Une application de sonde enchaîne ses étapes par des tâches de fond (réveil du thread
//! UI depuis un autre thread) et envoie à sa fenêtre de vrais événements AppKit : clic,
//! frappes, texte marqué d'IME. Les événements sont remis de façon différée
//! (`performSelector:afterDelay:`), hors de tout rappel de l'application, comme ceux du
//! système. Les fenêtres sont rendues transparentes dès leur première image.
//!
//! Couvert : erreurs de validation, chargement, rendu et présentation, clic et capture,
//! saisie de texte et composition, tâches et réveil, ouverture et fermeture d'une seconde
//! fenêtre, thème système suivi (apparence de la fenêtre changée), boucle déjà en cours,
//! sortie.

#[cfg(target_os = "macos")]
mod appkit;
#[cfg(target_os = "macos")]
mod harness;
#[cfg(target_os = "macos")]
mod layers;
#[cfg(target_os = "macos")]
mod probe;

fn main() {
    #[cfg(target_os = "macos")]
    probe::run();
}
