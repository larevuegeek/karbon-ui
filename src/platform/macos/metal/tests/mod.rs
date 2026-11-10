//! Tests du renderer Metal : chaque scène est rendue par Metal (texture relue) et par
//! Core Graphics (`graphics::Renderer::render`), puis les pixels sont comparés avec une
//! tolérance d'anticrénelage, à 1x et 2x.
//!
//! Sans périphérique Metal (machine virtuelle, intégration continue sans GPU), les tests
//! se sautent en le signalant ; `KARBON_REQUIRE_METAL=1` en fait des échecs.
//! `KARBON_METAL_DUMP=<dossier>` y écrit les deux images de chaque comparaison.
mod comparison;
mod effects;
mod filters;
mod layers;
mod shapes;
mod storage;
mod support;
mod text;
pub(crate) use comparison::*;
