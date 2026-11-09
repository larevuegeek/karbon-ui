//! Essai de la cadence d'une image hors Windows (voir `platform/backend.rs`) : à activer
//! par `KARBON_FRAME_PACING=1` tant que le gain n'est pas mesuré sur un vrai écran.
use std::sync::OnceLock;

/// La cadence d'essai est demandée.
pub(in crate::platform) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("KARBON_FRAME_PACING").is_some_and(|v| v == "1"))
}
