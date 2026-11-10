//! Rastériseur logiciel minimal d'une `Scene`, pour les aperçus et les tests headless.
//!
//! Un échantillon par pixel (au centre), sans anticrénelage : le résultat est exact et
//! reproductible sur toutes les plateformes. Chaque forme est testée dans son repère
//! local (transformation inverse), ce qui gère translations, échelles et rotations.
//! Couverture : rectangles arrondis pleins ou tracés, lignes, chemins, images (plus
//! proche voisin), clips rectangulaires ou arrondis, opacité de groupe, dessins SVG
//! compilés (remplissage et tracé). Approximations documentées : dégradés peints avec la
//! moyenne de leurs arrêts, filtres ignorés, icônes `Vector` non dessinées, jonctions SVG
//! en onglet ou biseau arrondies, glyphes remplacés par des boîtes aux métriques de
//! `HeadlessTextMeasurer`.
mod backdrop;
mod canvas;
mod glyphs;
mod paint;
mod shape;
mod svg;
#[cfg(test)]
mod tests;
pub(crate) use canvas::*;
