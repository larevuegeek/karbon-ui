//! Trace du démarrage, sur demande : `KARBON_STARTUP_TRACE=1` écrit sur la sortie
//! d'erreur le temps écoulé depuis la création du processus, et la mémoire privée du
//! processus, à chaque étape (COM,
//! GDI+, périphérique, fenêtre, polices, chargement de l'application, première
//! préparation, rendu, `Present`). `KARBON_STARTUP_TRACE=exit` quitte en plus la
//! boucle de messages après le premier `Present`, pour mesurer des lancements en série.
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentProcess, GetProcessTimes},
    UI::WindowsAndMessaging::PostQuitMessage,
};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Off,
    Print,
    Exit,
}
fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("KARBON_STARTUP_TRACE").as_deref() {
        Ok("exit") => Mode::Exit,
        Ok("" | "0") | Err(_) => Mode::Off,
        Ok(_) => Mode::Print,
    })
}
/// Création du processus, en temps système.
fn created() -> Option<SystemTime> {
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY : pseudo-handle du processus courant ; quatre structures locales valides.
    if unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return None;
    }
    // FILETIME : unités de 100 ns depuis 1601 ; 11 644 473 600 s avant l'époque Unix.
    let ticks = u64::from(created.dwLowDateTime) | (u64::from(created.dwHighDateTime) << 32);
    let unix = ticks.checked_sub(11_644_473_600 * 10_000_000)?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_nanos(unix * 100))
}
/// Millisecondes écoulées depuis la création du processus.
fn elapsed_ms() -> f64 {
    static CREATED: OnceLock<Option<SystemTime>> = OnceLock::new();
    CREATED
        .get_or_init(created)
        .and_then(|c| SystemTime::now().duration_since(c).ok())
        .map_or(0., |d| d.as_secs_f64() * 1000.)
}
/// Note une étape du démarrage (sans effet hors trace).
pub(crate) fn mark(step: &str) {
    if mode() != Mode::Off {
        let private =
            super::super::offscreen::sample().map_or(0., |s| s.private_bytes as f64 / 1048576.);
        eprintln!(
            "karbon-startup: {:8.1} ms  {private:6.1} MiB  {step}",
            elapsed_ms()
        );
    }
}
/// Appelé après chaque soumission d'image : seule la première est notée.
pub(crate) fn presented() {
    static FIRST: AtomicBool = AtomicBool::new(false);
    if mode() == Mode::Off || FIRST.swap(true, Ordering::Relaxed) {
        return;
    }
    mark("first present");
    if mode() == Mode::Exit {
        // SAFETY : dépose WM_QUIT dans la file du thread courant (thread UI).
        unsafe { PostQuitMessage(0) };
    }
}
