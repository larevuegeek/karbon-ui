//! Délégué de l'application (M4 de la revue). Sans lui, « Quitter » depuis le Dock, une
//! fermeture de session ou `terminate:` terminaient le processus au milieu de la boucle :
//! `run_windows` ne rendait pas la main et les applications n'étaient pas détruites.
//!
//! - Demande de l'utilisateur (Dock, menu, ⌘Q) : la boucle se termine comme sur
//!   `WindowCommand::Exit`, AppKit ne termine rien (`NSTerminateCancel`) et `run_windows`
//!   rend `Ok`.
//! - Fermeture de session, redémarrage, extinction : refuser bloquerait la déconnexion.
//!   La boucle se termine d'abord (fenêtres fermées, applications détruites), puis AppKit
//!   termine le processus (`NSTerminateLater`, puis `replyToApplicationShouldTerminate`).
//!
//! L'action « Quitter » de la barre de menus arrive aussi ici (le délégué est dans la chaîne
//! des répondeurs) : elle reste active sans fenêtre au premier plan.
use crate::window::WindowCommand;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::AnyObject,
};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationTerminateReply};
use objc2_foundation::{NSObject, NSObjectProtocol};
use std::cell::Cell;

/// Raisons d'un « Quitter » venu du système (`kAEQuitReason` de l'événement Apple) :
/// déconnexion, redémarrage, extinction (avec ou sans dialogue).
const SYSTEM_REASONS: [u32; 6] = [
    u32::from_be_bytes(*b"logo"),
    u32::from_be_bytes(*b"rlgo"),
    u32::from_be_bytes(*b"rrst"),
    u32::from_be_bytes(*b"rsdn"),
    u32::from_be_bytes(*b"rest"),
    u32::from_be_bytes(*b"shut"),
];

/// État du délégué : une terminaison par le système attend la fin de la boucle.
#[derive(Default)]
pub(super) struct Terminate {
    later: Cell<bool>,
}

define_class!(
    // SAFETY : NSObject n'impose rien aux sous-classes ; pas de `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KarbonUiAppDelegate"]
    #[ivars = Terminate]
    pub(super) struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _sender: &NSApplication) -> NSApplicationTerminateReply {
            super::command(WindowCommand::Exit);
            super::wake();
            if system_quit() {
                self.ivars().later.set(true);
                NSApplicationTerminateReply::TerminateLater
            } else {
                NSApplicationTerminateReply::TerminateCancel
            }
        }
    }

    impl AppDelegate {
        #[unsafe(method(karbonQuit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            super::command(WindowCommand::Exit);
        }
    }
);

impl AppDelegate {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Terminate::default());
        // SAFETY : initialiseur de NSObject.
        unsafe { msg_send![super(this), init] }
    }
    /// La boucle est finie : une terminaison par le système peut maintenant avoir lieu
    /// (AppKit termine alors le processus).
    pub(super) fn finish(&self, app: &NSApplication) {
        if self.ivars().later.replace(false) {
            app.replyToApplicationShouldTerminate(true);
        }
    }
}

/// L'événement Apple en cours (`quit`) vient du système : déconnexion, redémarrage ou
/// extinction.
fn system_quit() -> bool {
    // SAFETY : messages de NSAppleEventManager et NSAppleEventDescriptor, sans argument
    // pointeur hors de leurs objets ; nil géré à chaque étape.
    unsafe {
        let manager: *mut AnyObject =
            msg_send![objc2::class!(NSAppleEventManager), sharedAppleEventManager];
        if manager.is_null() {
            return false;
        }
        let event: *mut AnyObject = msg_send![manager, currentAppleEvent];
        if event.is_null() {
            return false;
        }
        let reason: *mut AnyObject = msg_send![
            event,
            attributeDescriptorForKeyword: u32::from_be_bytes(*b"why?")
        ];
        if reason.is_null() {
            return false;
        }
        let code: u32 = msg_send![reason, enumCodeValue];
        SYSTEM_REASONS.contains(&code)
    }
}
