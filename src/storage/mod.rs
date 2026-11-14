//! Persistent key → string store of an application, like the web's `localStorage`:
//! [`LocalStorage`], and the process-wide store [`local`].
// Organisation : `local` (magasin, écriture atomique, magasin du processus), `json`
// (lecture et écriture du fichier).
mod json;
mod local;
#[cfg(test)]
mod tests;
pub use local::{LocalStorage, MAX_KEY_BYTES, MAX_TOTAL_BYTES, local, open_local};
