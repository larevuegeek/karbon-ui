//! Pont UI Automation. Les callbacks COM lisent un snapshot ; les mutations
//! repassent sur le thread UI par une file possédée, sans pointeur de pile envoyé.
//!
//! `tree` décrit les nœuds publiés, `bridge` relie les fournisseurs COM au thread UI,
//! `sync` reconstruit le snapshot depuis le runtime et `notify` émet les événements ;
//! `provider` et les modules de patterns implémentent les interfaces COM.
mod bridge;
mod choices;
mod expand;
mod ffi;
mod notify;
mod provider;
mod range_value;
mod sync;
#[cfg(test)]
mod tests;
mod text;
mod tooltip;
mod tree;
pub(crate) use sync::{Synced, sync, timer_fired};
mod lifecycle;
pub(crate) use lifecycle::*;
