//! Traduction d'une scène en couches d'instances et en étapes ordonnées, prêtes à envoyer
//! au GPU. Copie propre à Linux de `macos/metal/encode/` (même logique, mêmes données).
//!
//! La pile des portées suit `graphics/canvas.rs` : une découpe rectangulaire sous une
//! transformation qui garde les axes devient un rectangle en pixels (ciseau du lot et
//! couverture anticrénelée dans le shader) ; une découpe arrondie ou tournée devient un
//! maillon de la chaîne évaluée par le shader. Un effet
//! compose sa transformation ; son opacité s'applique directement à un élément seul, et un
//! groupe de plusieurs éléments est rendu dans une couche intermédiaire limitée à ses
//! bornes, recomposée avec l'opacité ; un contenu d'opacité nulle est sauté
//! (`scopes.rs`). Groupes filtrés, `backdrop-filter` et masques : `effects.rs`.
//!
//! Le texte (`text.rs`, remplissage du texte compris) est mis en page par le renderer
//! Cairo prêté au constructeur, ses glyphes tirés de l'atlas ; les images et les
//! rastérisations (`raster.rs`) sont préparées par ce même renderer et gardées en
//! textures.
//!
//! Ordre de rendu (`Step`) : une couche est dessinée quand elle est refermée, avant que sa
//! recomposition n'entre dans la couche parente, et avant les passages de filtre qui la
//! lisent ; la cible finale est dessinée en dernier.
mod effects;
mod layers;
mod raster;
mod scopes;
mod shapes;
mod text;
pub(crate) use layers::*;
