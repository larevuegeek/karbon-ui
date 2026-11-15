//! Lecture de JSON écrite à la main (aucun crate tiers), partagée par les modules qui
//! lisent des fichiers : `storage` (magasin de préférences) et `intl` (traductions).
// Organisation : `value` (l'arbre lu), `reader` (analyse du texte).
mod reader;
#[cfg(test)]
mod tests;
mod value;
pub(crate) use reader::parse;
pub(crate) use value::Value;
