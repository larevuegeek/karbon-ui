//! Feuille stricte (`css_file!` sans option) : chaque classe devient une fonction de
//! style typée ; la description de la feuille est transmise aux templates par la macro
//! `__karbon_classes!` du module généré.
mod load;
mod strict;
pub(crate) use strict::*;
