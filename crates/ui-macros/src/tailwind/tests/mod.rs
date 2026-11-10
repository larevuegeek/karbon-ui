//! Tests du compilateur Tailwind : déclarations produites par des listes de classes
//! (thème par défaut, ordre, variantes, propriétés composées, couches) et diagnostics.
//! `oracle` compare un échantillon d'utilitaires à la feuille générée par Tailwind.
mod cases;
pub(super) mod oracle;
