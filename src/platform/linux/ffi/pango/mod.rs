//! Pango (mise en page et mise en forme par HarfBuzz) et son pont Cairo : carte des
//! polices fontconfig, contexte, mises en page, descriptions de polices et attributs.
//! Les structures publiques de Pango lues par le backend sont transcrites dans
//! `types.rs`.
mod types;
pub(in crate::platform::linux) use types::*;
mod bindings;
pub(in crate::platform::linux) use bindings::*;
