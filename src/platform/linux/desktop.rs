//! Réglages du bureau lus par GSettings : polices d'interface et à chasse fixe (familles
//! génériques `system-ui` et `monospace`, `fonts.rs`) et animations (mouvement réduit,
//! comme « Réduire les animations » de macOS et de Windows).
//!
//! Le schéma suit le bureau en cours (`XDG_CURRENT_DESKTOP`) : celui de GNOME pour GNOME
//! et les bureaux qui le reprennent (Unity, Cinnamon, Budgie, Pantheon, Zorin), celui de
//! MATE pour MATE. Ailleurs (KDE, XFCE…), les génériques de fontconfig restent et les
//! animations sont permises : le schéma de GNOME, souvent installé, n'y décrit pas le
//! bureau. Les polices sont lues une fois par processus (un changement suit au lancement
//! suivant) ; le mouvement réduit est relu quand une fenêtre reprend le focus.
use super::ffi::{gio::Settings, pango::family_of};
use std::{cell::Cell, ffi::CStr, sync::OnceLock};

/// Polices du bureau (familles, sans taille ni style).
#[derive(Default, Debug)]
pub(super) struct Fonts {
    pub interface: Option<String>,
    pub monospace: Option<String>,
}
/// Schéma des réglages d'interface du bureau en cours, s'il en a un.
fn schema() -> Option<&'static CStr> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let has = |name: &str| desktop.split(':').any(|d| d.contains(name));
    if has("mate") {
        Some(c"org.mate.interface")
    } else if ["gnome", "unity", "cinnamon", "budgie", "pantheon", "zorin"]
        .iter()
        .any(|d| has(d))
    {
        Some(c"org.gnome.desktop.interface")
    } else {
        None
    }
}
/// Polices du bureau, lues au premier appel.
pub(super) fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        let Some(settings) = schema().and_then(Settings::open) else {
            return Fonts::default();
        };
        let family = |key: &CStr| settings.string(key).as_deref().and_then(family_of);
        Fonts {
            interface: family(c"font-name"),
            monospace: family(c"monospace-font-name"),
        }
    })
}
thread_local! {
    /// Mouvement réduit du bureau (`None` : à relire).
    static REDUCED_MOTION: Cell<Option<bool>> = const { Cell::new(None) };
}
/// Le bureau demande des animations réduites (animations désactivées).
pub(super) fn reduced_motion() -> bool {
    REDUCED_MOTION.with(|cached| {
        cached.get().unwrap_or_else(|| {
            let animations = schema()
                .and_then(Settings::open)
                .and_then(|s| s.boolean(c"enable-animations"))
                .unwrap_or(true);
            cached.set(Some(!animations));
            !animations
        })
    })
}
/// Une fenêtre reprend le focus : les réglages ont pu changer entre-temps.
pub(super) fn refresh() {
    REDUCED_MOTION.with(|cached| cached.set(None));
}
