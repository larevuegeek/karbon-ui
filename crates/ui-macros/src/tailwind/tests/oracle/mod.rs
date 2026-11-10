//! Oracle : les déclarations produites pour des utilitaires sont comparées à celles de
//! la feuille que Tailwind 4.3.3 a réellement générée pour RustMusic.
//!
//! Les deux côtés passent par la même normalisation : variables résolues (thème et
//! valeurs initiales `@property` de la feuille générée pour l'oracle), `rem` en px,
//! propriétés logiques (`padding-inline`…) en côtés physiques, propriétés composées
//! (`translate`, `transition-*`, `box-shadow`) en propriétés du moteur, puis
//! traduction par le compilateur CSS du moteur ; les nombres du code obtenu sont
//! comparés à 1 % près. `oracle.css` reprend un extrait de la feuille générée (thème,
//! `@property` et règles des utilitaires testés) ; la feuille complète, si elle est
//! présente, sert au relevé de `tools::oracle` sur tous les jetons acceptés.
mod comparison;
mod tokens;
pub(in crate::tailwind) use comparison::*;
