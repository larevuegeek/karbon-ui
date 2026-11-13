//! Outillage de la sonde native : seconde fenêtre, accès aux fenêtres AppKit, exécution
//! différée hors des rappels, options des fenêtres de test, garde de fin et chien de garde.
use crate::probe::{HEIGHT, Log, WIDTH};
use karbon_ui::{
    application::{Application, ViewContext},
    ui::prelude::*,
    window::{WindowId, WindowOptions},
};
use objc2::{MainThreadMarker, msg_send, rc::Retained, runtime::AnyObject, sel};
use objc2_app_kit::{NSApplication, NSWindow};
use objc2_foundation::NSObjectNSDelayedPerforming;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
/// Seconde fenêtre, sans contenu interactif.
pub(crate) struct Second;
impl Application for Second {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        hide_windows();
        column("second", [text("t", "Seconde fenêtre")])
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}
thread_local! {
    pub(crate) static LOG: std::cell::RefCell<Option<Arc<Mutex<Log>>>> = const { std::cell::RefCell::new(None) };
}
pub(crate) fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("la sonde tourne sur le thread principal")
}
pub(crate) fn windows() -> Vec<Retained<NSWindow>> {
    NSApplication::sharedApplication(mtm()).windows().to_vec()
}
/// Fenêtre visible de ce titre (`NSApp.windows` liste aussi des fenêtres d'AppKit et des
/// fenêtres fermées pas encore libérées).
pub(crate) fn visible(title: &str) -> bool {
    windows()
        .iter()
        .any(|w| w.isVisible() && w.title().to_string() == title)
}
/// Titre de la fenêtre opaque qui couvre la fenêtre principale (laissée visible).
pub(crate) const COVER: &str = "couverture";
pub(crate) fn hide_windows() {
    for window in windows()
        .into_iter()
        .filter(|w| w.title().to_string() != COVER)
    {
        window.setAlphaValue(0.);
    }
}
pub(crate) fn window(_: WindowId) -> Retained<NSWindow> {
    windows()
        .into_iter()
        .find(|w| w.title().to_string() == "sonde")
        .expect("fenêtre de la sonde")
}
type Job = Box<dyn FnOnce()>;
thread_local! {
    static QUEUE: std::cell::RefCell<Vec<Job>> = const { std::cell::RefCell::new(Vec::new()) };
    static TIMER: Retained<Timer> = Timer::new(mtm());
}
objc2::define_class!(
    // SAFETY : NSObject n'impose rien ; pas de `Drop`.
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "KarbonUiProbeTimer"]
    struct Timer;

    impl Timer {
        #[unsafe(method(fire:))]
        fn fire(&self, _argument: Option<&AnyObject>) {
            let jobs = QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
            for job in jobs {
                job();
            }
        }
    }
);
impl Timer {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = objc2::MainThreadOnly::alloc(mtm).set_ivars(());
        // SAFETY : initialiseur de NSObject.
        unsafe { msg_send![super(this), init] }
    }
}
/// Exécute `job` au prochain tour de la boucle AppKit, hors du rappel en cours (la
/// session de la fenêtre y est libre), comme un événement du système.
pub(crate) fn deferred(job: impl FnOnce() + 'static) {
    QUEUE.with(|q| q.borrow_mut().push(Box::new(job)));
    // SAFETY : `fire:` est défini ci-dessus ; aucun argument.
    TIMER.with(|t| unsafe { t.performSelector_withObject_afterDelay(sel!(fire:), None, 0.) });
}
pub(crate) fn options(title: &'static str) -> WindowOptions {
    WindowOptions {
        title,
        width: WIDTH,
        height: HEIGHT,
        min_width: 200,
        min_height: 200,
        title_bar: false,
        activate: false,
        // Au-dessus des fenêtres ordinaires des autres applications sans activer celle-ci
        // (M7) : sinon, une fenêtre couverte par celles de l'utilisateur ne peint pas
        // (M5) et le résultat dépendrait de son écran.
        topmost: true,
        ..WindowOptions::DEFAULT
    }
}
/// La sonde est allée au bout : sans cela, une sortie du processus en cours de route
/// (`terminate:` qui ne rend pas la main) serait un faux succès.
pub(crate) static FINISHED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
/// Dernière étape atteinte, lue par le chien de garde.
pub(crate) static PROGRESS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Chien de garde : une boucle qui ne se réveille plus bloquerait le test pour toujours.
pub(crate) fn watchdog() {
    std::thread::spawn(|| {
        let mut last = (0, std::time::Instant::now());
        loop {
            std::thread::sleep(Duration::from_millis(500));
            if FINISHED.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            let step = PROGRESS.load(std::sync::atomic::Ordering::SeqCst);
            if step != last.0 {
                last = (step, std::time::Instant::now());
            } else if last.1.elapsed() > Duration::from_secs(20) {
                eprintln!("sonde macOS : bloquée à l'étape {step} (boucle sans réveil)");
                std::process::abort();
            }
        }
    });
}
pub(crate) extern "C" fn unfinished() {
    if !FINISHED.load(std::sync::atomic::Ordering::SeqCst) {
        eprintln!("sonde macOS : processus terminé avant la fin de la sonde");
        std::process::abort();
    }
}
