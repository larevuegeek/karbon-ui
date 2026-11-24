//! Peinture de l'arbre d'éléments en commandes de scène : parcours (ordre de peinture,
//! éléments fixes, effets, filtres, masques, rognages) et chrome de la fenêtre.
mod border;
mod chrome;
mod cull;
mod decoration;
mod kept;
mod surface;
pub(super) use chrome::paint_chrome;
pub(crate) use kept::KeptPaints;
mod node;
pub(crate) use node::*;
