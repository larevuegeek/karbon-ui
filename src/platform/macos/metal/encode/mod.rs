//! Traduction d'une scène en couches d'instances et en étapes ordonnées, prêtes à envoyer
//! au GPU.
//!
//! La pile des portées suit `graphics/canvas.rs` : une découpe rectangulaire sous une
//! transformation qui garde les axes devient un rectangle en pixels (scissor du lot et
//! couverture anticrénelée dans le shader) ; une découpe arrondie ou tournée, ou un masque
//! en dégradé, devient un maillon de la chaîne évaluée par le shader. Un effet compose sa
//! transformation ; son opacité s'applique directement à un élément seul, et un groupe de
//! plusieurs éléments est rendu dans une texture intermédiaire limitée à ses bornes,
//! recomposée avec l'opacité ; un contenu d'opacité nulle est sauté (`scopes.rs`). Les
//! groupes filtrés, `backdrop-filter` et les masques sont dans `effects.rs`, le texte
//! rempli dans `text.rs`.
//!
//! Ordre de rendu (`Step`) : une couche est dessinée quand elle est refermée, avant que sa
//! recomposition n'entre dans la couche parente ; la cible finale est dessinée en un seul
//! passage, en dernier. Un `backdrop-filter` ne relit pas la cible : ce qu'elle a reçu
//! jusque-là dans sa zone est redessiné dans une couche de la taille de cette zone
//! (`effects.rs`), ce qui évite de couper le passage de la cible (une écriture et une
//! relecture de toute la fenêtre par coupure).
mod effects;
mod layers;
mod raster;
mod scopes;
mod shapes;
mod text;
pub(crate) use layers::*;
