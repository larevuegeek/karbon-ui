//! Cadence d'une image hors Windows (voir `platform/backend.rs`). Active par défaut là où
//! la fin de l'attente de l'écran est exacte : le rappel de trame de Wayland (mesuré le
//! 2026-11-11, Radeon 760M, 60 Hz : pire écart en défilant 54,8 → 43,3 ms, à-coups 28 →
//! 26 en 30 s). Ailleurs (X11, macOS), l'attente n'est qu'estimée : la cadence s'y
//! demande par `KARBON_FRAME_PACING=1` tant qu'elle n'y est pas mesurée.
use std::sync::OnceLock;

/// `KARBON_FRAME_PACING` : `1` impose la cadence, `0` la retire, absent ou autre valeur :
/// le choix par défaut.
fn forced() -> Option<bool> {
    static FORCED: OnceLock<Option<bool>> = OnceLock::new();
    *FORCED.get_or_init(
        || match std::env::var_os("KARBON_FRAME_PACING")?.to_str()? {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        },
    )
}

/// La cadence s'applique ; `exact` : le backend connaît la fin de l'attente de l'écran
/// (rappel de trame du compositeur).
pub(in crate::platform) fn enabled(exact: bool) -> bool {
    forced().unwrap_or(exact)
}
