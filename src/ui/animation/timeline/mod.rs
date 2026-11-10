//! État conservé uniquement pour les éléments qui optent pour un effet.
mod ghost;
mod paths;
#[cfg(test)]
mod tests;
mod track;
mod value;
pub(crate) use value::Interpolate;
mod state;
pub(crate) use state::*;
