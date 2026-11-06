//! Ce que les boucles Wayland (`linux/wayland/`) et X11 (`linux/x11/`) partagent : une
//! seule boucle par thread, réveil depuis les tâches de fond (`eventfd`), clavier
//! xkbcommon (`keyboard.rs` : touches du moteur, raccourcis d'édition, touches mortes,
//! répétition), défilement (`scroll.rs`), composition des méthodes d'entrée
//! (`composition.rs`), barre de titre et bords du moteur (`chrome.rs`) et fenêtre vue du
//! moteur (`session.rs` : application, renderers, tâches, réveils).
pub(super) mod chrome;
pub(super) mod composition;
pub(super) mod keyboard;
#[cfg(test)]
pub(super) mod probe;
pub(super) mod scroll;
pub(super) mod session;

use std::{
    cell::Cell,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::Arc,
};

thread_local! {
    static RUNNING: Cell<bool> = const { Cell::new(false) };
}
/// Boucle de fenêtres en cours sur ce thread, jusqu'à sa destruction.
pub(in crate::platform::linux) struct RunGuard(());
impl RunGuard {
    /// Marque la boucle en cours ; erreur si une autre l'est déjà sur ce thread.
    pub fn enter() -> crate::Result<Self> {
        if RUNNING.with(|r| r.replace(true)) {
            return Err(crate::Error::AlreadyRunning);
        }
        Ok(Self(()))
    }
}
impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.with(|r| r.set(false));
    }
}
/// Réveil de la boucle depuis un autre thread (`eventfd`).
pub(in crate::platform::linux) struct Wake(Arc<OwnedFd>);
impl Wake {
    pub fn new() -> crate::Result<Self> {
        // SAFETY : création d'un descripteur possédé ci-dessous.
        let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY : descripteur fraîchement créé.
        Ok(Self(Arc::new(unsafe { OwnedFd::from_raw_fd(fd) })))
    }
    /// Fonction de réveil des tâches d'une fenêtre (thread-safe).
    pub fn waker(&self) -> impl Fn() + Send + Sync + 'static {
        let fd = Arc::clone(&self.0);
        move || {
            let one = 1u64;
            // SAFETY : écriture de 8 octets sur un eventfd vivant (gardé par l'`Arc`).
            unsafe { libc::write(fd.as_raw_fd(), (&raw const one).cast(), 8) };
        }
    }
    /// Descripteur à surveiller (`poll`, lisible après un réveil).
    pub fn fd(&self) -> std::os::fd::RawFd {
        self.0.as_raw_fd()
    }
    /// Remet le compteur à zéro après un réveil.
    pub fn clear(&self) {
        let mut value = 0u64;
        // SAFETY : lecture de 8 octets sur un eventfd non bloquant.
        unsafe { libc::read(self.0.as_raw_fd(), (&raw mut value).cast(), 8) };
    }
}
/// Identifiant de l'application pour le système (associé au fichier `.desktop`) : nom de
/// l'exécutable.
pub(in crate::platform::linux) fn app_id() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "karbon-ui".into())
}
