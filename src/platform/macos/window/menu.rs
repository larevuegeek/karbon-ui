//! Barre de menus de l'application : menu de l'application (masquer, quitter), Édition
//! (annuler, couper, copier, coller, tout sélectionner) et Fenêtre (réduire, agrandir,
//! fermer). Libellés en français ou en anglais selon la langue préférée du système.
//!
//! Les actions sont envoyées sans cible : AppKit les remet au premier répondeur, la vue
//! de la fenêtre active (`view.rs`), qui les transmet au moteur.
use objc2::{MainThreadMarker, MainThreadOnly, runtime::Sel, sel};
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{NSLocale, NSProcessInfo, NSString};

fn french() -> bool {
    NSLocale::preferredLanguages()
        .firstObject()
        .is_some_and(|l| l.to_string().starts_with("fr"))
}
fn item(menu: &NSMenu, title: &str, action: Sel, key: &str, modifiers: NSEventModifierFlags) {
    let mtm = MainThreadMarker::from(menu);
    // SAFETY : titre, action et raccourci valides ; l'action est envoyée sans cible.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(key),
        )
    };
    item.setKeyEquivalentModifierMask(modifiers);
    menu.addItem(&item);
}
fn submenu(bar: &NSMenu, title: &str, mtm: MainThreadMarker) -> objc2::rc::Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
    let holder = NSMenuItem::new(mtm);
    holder.setSubmenu(Some(&menu));
    bar.addItem(&holder);
    menu
}
/// Installe la barre de menus de l'application.
pub(super) fn install(app: &NSApplication, mtm: MainThreadMarker) {
    let fr = french();
    let t = |fr_label: &'static str, en_label: &'static str| if fr { fr_label } else { en_label };
    let name = NSProcessInfo::processInfo().processName().to_string();
    let command = NSEventModifierFlags::Command;
    let bar = NSMenu::new(mtm);
    let application = submenu(&bar, &name, mtm);
    item(
        &application,
        &format!("{} {name}", t("Masquer", "Hide")),
        sel!(hide:),
        "h",
        command,
    );
    item(
        &application,
        t("Masquer les autres", "Hide Others"),
        sel!(hideOtherApplications:),
        "h",
        command | NSEventModifierFlags::Option,
    );
    application.addItem(&NSMenuItem::separatorItem(mtm));
    item(
        &application,
        &format!("{} {name}", t("Quitter", "Quit")),
        sel!(karbonQuit:),
        "q",
        command,
    );
    let edit = submenu(&bar, t("Édition", "Edit"), mtm);
    item(&edit, t("Annuler", "Undo"), sel!(undo:), "z", command);
    item(
        &edit,
        t("Rétablir", "Redo"),
        sel!(redo:),
        "z",
        command | NSEventModifierFlags::Shift,
    );
    edit.addItem(&NSMenuItem::separatorItem(mtm));
    item(&edit, t("Couper", "Cut"), sel!(cut:), "x", command);
    item(&edit, t("Copier", "Copy"), sel!(copy:), "c", command);
    item(&edit, t("Coller", "Paste"), sel!(paste:), "v", command);
    item(
        &edit,
        t("Tout sélectionner", "Select All"),
        sel!(selectAll:),
        "a",
        command,
    );
    let window = submenu(&bar, t("Fenêtre", "Window"), mtm);
    item(
        &window,
        t("Réduire", "Minimize"),
        sel!(karbonMinimize:),
        "m",
        command,
    );
    item(
        &window,
        t("Agrandir", "Zoom"),
        sel!(karbonZoom:),
        "",
        command,
    );
    item(
        &window,
        t("Fermer", "Close"),
        sel!(karbonClose:),
        "w",
        command,
    );
    app.setMainMenu(Some(&bar));
    app.setWindowsMenu(Some(&window));
}
