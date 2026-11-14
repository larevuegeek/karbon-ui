//! Parties pures de l'icône : conversion des pixels, identifiants du menu. Le service D-Bus
//! se vérifie à la main (voir `docs/windows.md`, zone de notification).
use super::{dbusmenu, pixmap};
use crate::{
    ui::scene::Image,
    window::{TrayEvent, TrayMenuItem},
};
use std::cell::RefCell;

#[test]
fn pixels_become_straight_argb() {
    // BGRA prémultiplié : rouge à moitié transparent, puis transparent.
    let image = Image::new(2, 1, vec![0, 0, 128, 128, 0, 0, 0, 0]).unwrap();
    let pixels: Vec<[u8; 4]> = pixmap::straight_pixels(&image).collect();
    assert_eq!(pixels, [[128, 255, 0, 0], [0, 0, 0, 0]]);
}

#[test]
fn menu_ids_follow_the_rank_and_report_the_application_id() {
    let menu = [
        TrayMenuItem::Item {
            id: 40,
            label: "Show_all".into(),
            enabled: true,
        },
        TrayMenuItem::Separator,
        TrayMenuItem::Item {
            id: 41,
            label: "Greyed".into(),
            enabled: false,
        },
    ];
    assert_eq!(dbusmenu::entry(&menu, 0), None);
    assert_eq!(dbusmenu::entry(&menu, 2), Some(&TrayMenuItem::Separator));
    assert_eq!(dbusmenu::entry(&menu, 9), None);
    let events = RefCell::new(Vec::new());
    let notify = |event| events.borrow_mut().push(event);
    dbusmenu::clicked(&menu, 1, "clicked", &notify);
    dbusmenu::clicked(&menu, 1, "hovered", &notify);
    dbusmenu::clicked(&menu, 2, "clicked", &notify);
    dbusmenu::clicked(&menu, 3, "clicked", &notify);
    assert_eq!(*events.borrow(), [TrayEvent::MenuItem(40)]);
    // `_` marque un raccourci dans dbusmenu : doublé pour s'afficher.
    assert_eq!(dbusmenu::mnemonic_free("Show_all"), "Show__all");
}
