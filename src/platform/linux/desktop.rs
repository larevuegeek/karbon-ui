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
//!
//! Le thème vient du portail (`portal/appearance.rs`), relu au même moment, sur un thread
//! à part car l'appel peut attendre le démarrage du portail.
use super::{
    ffi::{gio::Settings, pango::family_of},
    portal::{Waker, prefers_dark},
};
use std::{
    cell::Cell,
    ffi::CStr,
    sync::{
        Mutex, Once, OnceLock, PoisonError,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

/// Attente de la première réponse du portail, pour que la première image ait le bon thème.
const FIRST_READ_WAIT: Duration = Duration::from_millis(100);
/// Thème sombre du bureau, dernière valeur reçue du portail (clair sans portail).
static DARK: AtomicBool = AtomicBool::new(false);
/// Réveil de la boucle de fenêtres, quand le thème reçu a changé.
static WAKE: Mutex<Option<Waker>> = Mutex::new(None);

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
/// Le bureau est en thème sombre.
pub(super) fn dark() -> bool {
    DARK.load(Ordering::Relaxed)
}
/// Relit le thème sur un thread à part, et l'attend au plus `wait`.
fn read_dark(wait: Duration) {
    let (done, finished) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("karbon-ui-color-scheme".into())
        .spawn(move || {
            if let Some(dark) = prefers_dark()
                && DARK.swap(dark, Ordering::Relaxed) != dark
                && let Some(wake) = WAKE.lock().unwrap_or_else(PoisonError::into_inner).as_ref()
            {
                wake();
            }
            let _ = done.send(());
        });
    if spawned.is_ok() && !wait.is_zero() {
        let _ = finished.recv_timeout(wait);
    }
}
/// Une fenêtre s'ouvre : `wake` réveillera la boucle quand le thème change. La première
/// lecture du processus est attendue un court instant.
pub(super) fn follow_color_scheme(wake: Waker) {
    *WAKE.lock().unwrap_or_else(PoisonError::into_inner) = Some(wake);
    static FIRST: Once = Once::new();
    FIRST.call_once(|| read_dark(FIRST_READ_WAIT));
}
/// Une fenêtre reprend le focus : les réglages ont pu changer entre-temps.
pub(super) fn refresh() {
    REDUCED_MOTION.with(|cached| cached.set(None));
    read_dark(Duration::ZERO);
}
