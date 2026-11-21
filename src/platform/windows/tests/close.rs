//! Fermeture demandée par l'utilisateur (WM_CLOSE : Alt+F4, barre des tâches) : une
//! application qui la refuse garde sa fenêtre, les autres ferment comme avant. Le message
//! est envoyé à une fenêtre cachée du test, rien ne passe par le bureau.
use super::super::window::{state, take_error};
use super::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyWindow, IsWindow, WM_CLOSE};

/// Une application qui refuse de fermer et compte les demandes.
struct Unsaved(Rc<Cell<u32>>);
impl Application for Unsaved {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        widgets::text(1, "Changements non enregistrés")
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn close_requested(&mut self) -> bool {
        self.0.set(self.0.get() + 1);
        false
    }
}

#[test]
fn a_window_whose_application_refuses_to_close_stays_open() {
    let (_apartment, session) = session();
    let asked = Rc::new(Cell::new(0));
    let mut refusing = descriptor(1, app());
    refusing.application = Box::new(Unsaved(Rc::clone(&asked)));
    let window = session.open(refusing, false).unwrap();
    let plain = session.open(descriptor(2, app()), false).unwrap();
    {
        let _scope = state::enter(window);
        // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
        unsafe { SendMessageW(window, WM_CLOSE, 0, 0) };
    }
    assert_eq!(asked.get(), 1);
    // SAFETY : simple test de validité d'un handle.
    assert_ne!(unsafe { IsWindow(window) }, 0);
    {
        let _scope = state::enter(plain);
        // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
        unsafe { SendMessageW(plain, WM_CLOSE, 0, 0) };
    }
    // SAFETY : simple test de validité d'un handle.
    assert_eq!(unsafe { IsWindow(plain) }, 0);
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    unsafe {
        DestroyWindow(window);
    }
}
