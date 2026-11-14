//! Icône de la fenêtre et icône de la zone de notification, sur une fenêtre cachée : icônes
//! posées aux deux tailles, pixels convertis pour Windows, clic et menu remis à
//! l'application. L'icône n'est pas ajoutée au vrai Shell : les messages de rappel sont
//! envoyés directement à la fenêtre.
use super::super::{
    imaging::DrawingEngine,
    window::{icon, tray},
};
use super::*;
use crate::{
    ui::scene::Image,
    window::{TrayEvent, TrayMenuItem},
};
use windows_sys::Win32::UI::{
    Shell::NIN_SELECT,
    WindowsAndMessaging::{DestroyWindow, ICON_BIG, ICON_SMALL, WM_GETICON},
};

const COVER: &[u8] = include_bytes!("../../../../tests/fixtures/images/cover.png");

struct TrayWatcher {
    events: Rc<RefCell<Vec<TrayEvent>>>,
}
impl Application for TrayWatcher {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        widgets::text(1, "Icône")
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn tray_event(&mut self, event: TrayEvent) -> bool {
        self.events.borrow_mut().push(event);
        false
    }
}

fn hidden(app: impl Application, icon: Option<&'static [u8]>) -> (ComApartment, Session, Hwnd) {
    let (apartment, session) = session();
    let options = WindowOptions {
        title: "Karbon UI - icône",
        width: 240,
        height: 200,
        min_width: 120,
        min_height: 120,
        activate: false,
        renderer: RenderMode::Software,
        icon,
        ..WindowOptions::DEFAULT
    };
    let window = session
        .open(WindowDescriptor::new(WindowId(1), options, app), false)
        .unwrap();
    (apartment, session, window)
}

#[test]
fn the_window_icon_is_set_at_both_system_sizes() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let (_apartment, _session, window) = hidden(TrayWatcher { events }, Some(COVER));
    for kind in [ICON_BIG, ICON_SMALL] {
        // SAFETY : fenêtre de ce thread ; WM_GETICON ne lit que ses arguments.
        let icon = unsafe { SendMessageW(window, WM_GETICON, kind as usize, 0) };
        assert_ne!(icon, 0, "icône {kind} posée");
    }
    // SAFETY : fenêtre de ce thread, détruite une fois.
    unsafe { DestroyWindow(window) };
}

#[test]
fn tray_clicks_reach_the_application_even_without_an_icon_shown() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let (_apartment, _session, window) = hidden(
        TrayWatcher {
            events: Rc::clone(&events),
        },
        None,
    );
    // Rappel du Shell (version 4) : événement dans le mot bas de `lparam`.
    // SAFETY : fenêtre de ce thread ; message traité de façon synchrone.
    unsafe { SendMessageW(window, tray::TRAY_MESSAGE, 0, NIN_SELECT as isize) };
    // NIN_KEYSELECT (Entrée sur l'icône) : la même activation.
    unsafe { SendMessageW(window, tray::TRAY_MESSAGE, 0, (NIN_SELECT | 1) as isize) };
    assert_eq!(
        *events.borrow(),
        [TrayEvent::Activated, TrayEvent::Activated]
    );
    // SAFETY : fenêtre de ce thread, détruite une fois.
    unsafe { DestroyWindow(window) };
}

#[test]
fn menu_commands_designate_enabled_entries_only() {
    let menu = [
        TrayMenuItem::Item {
            id: 10,
            label: "Afficher".into(),
            enabled: true,
        },
        TrayMenuItem::Separator,
        TrayMenuItem::Item {
            id: 11,
            label: "Grisé".into(),
            enabled: false,
        },
        TrayMenuItem::Item {
            id: 12,
            label: "Quitter".into(),
            enabled: true,
        },
    ];
    // Commande = rang + 1 ; 0 : menu fermé sans choix.
    assert_eq!(tray::chosen_item(&menu, 1), Some(10));
    assert_eq!(tray::chosen_item(&menu, 4), Some(12));
    assert_eq!(tray::chosen_item(&menu, 0), None);
    assert_eq!(tray::chosen_item(&menu, 2), None);
    assert_eq!(tray::chosen_item(&menu, 3), None);
    assert_eq!(tray::chosen_item(&menu, 9), None);
    assert_eq!(tray::chosen_item(&menu, -1), None);
}

#[test]
fn icon_pixels_lose_premultiplication_and_are_centred() {
    // 2 × 1 pixels BGRA prémultipliés : rouge à moitié transparent, puis transparent.
    let image = Image::new(2, 1, vec![0, 0, 128, 128, 0, 0, 0, 0]).unwrap();
    let pixels = icon::square_straight_alpha(&image, 4);
    assert_eq!(pixels.len(), 4 * 4 * 4);
    // Centrée : colonne 1, ligne 1 (marges de 1 à gauche, 1,5 arrondi à 1 en haut).
    let at = |x: usize, y: usize| &pixels[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4];
    assert_eq!(at(1, 1), [0, 0, 255, 128]);
    assert_eq!(at(2, 1), [0, 0, 0, 0]);
    assert_eq!(at(0, 0), [0, 0, 0, 0]);
}

#[test]
fn icons_are_built_from_encoded_images_and_reject_garbage() {
    let _apartment = ComApartment::initialize().unwrap();
    let decoder = DrawingEngine::initialize().unwrap();
    assert!(
        !icon::from_image(&decoder, COVER, 24)
            .unwrap()
            .handle()
            .is_null()
    );
    assert!(icon::from_image(&decoder, b"not an image", 24).is_err());
}
