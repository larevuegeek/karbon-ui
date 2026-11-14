//! Icône de la zone de notification (`Shell_NotifyIconW`, comportement de la version 4),
//! posée, remplacée ou retirée par `WindowCommand::Tray`. Elle est rattachée à la fenêtre
//! qui reçoit ses messages, même masquée ; reposée quand l'Explorateur redémarre
//! (`TaskbarCreated`) ; retirée à la destruction de la fenêtre. Le clic et le choix d'une
//! entrée du menu reviennent à `Application::tray_event`.
use super::{
    super::{ffi::Hwnd, imaging::DrawingEngine, present::runtime},
    icon::{self, Icon},
    state::WindowLocal,
};
use crate::window::{TrayEvent, TrayIcon, TrayMenuItem};
use std::{cell::RefCell, ptr::null, sync::OnceLock};
use windows_sys::Win32::UI::{
    HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi},
    Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
        NIM_SETVERSION, NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
    },
    WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetSystemMetrics, MF_GRAYED, MF_SEPARATOR,
        MF_STRING, PostMessageW, RegisterWindowMessageW, SM_CXSMICON, SM_MENUDROPALIGNMENT,
        SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
        TPM_RIGHTALIGN, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_APP, WM_CONTEXTMENU, WM_NULL,
    },
};

/// Message de rappel de l'icône (après les réveils de la boucle, des tâches et des images).
pub(crate) const TRAY_MESSAGE: u32 = WM_APP + 4;
/// Une seule icône par fenêtre.
const TRAY_ID: u32 = 1;
/// `NIN_KEYSELECT` de shellapi.h (`NIN_SELECT | NINF_KEY`), absent de windows-sys.
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

/// Icône posée par l'application : son image native et sa description (infobulle, menu).
pub(crate) struct NativeTray {
    icon: Icon,
    tray: TrayIcon,
}

static TRAY: WindowLocal<RefCell<Option<NativeTray>>> = WindowLocal::new(|state| &state.tray);

/// Pose, remplace ou retire (`None`) l'icône de `window`. Une image illisible est une
/// erreur et laisse l'icône précédente en place.
pub(crate) fn set(
    window: Hwnd,
    decoder: &DrawingEngine,
    tray: Option<TrayIcon>,
) -> crate::Result<()> {
    let Some(tray) = tray else {
        remove(window);
        return Ok(());
    };
    // Taille des petites icônes au DPI du système : celle de la zone de notification.
    // SAFETY : simples lectures de métriques système.
    let edge = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) }.max(16);
    let native = NativeTray {
        icon: icon::from_image(decoder, &tray.image, edge as u32)?,
        tray,
    };
    let shown = TRAY.with(|slot| slot.borrow().is_some());
    if shown {
        notify(NIM_MODIFY, &data(window, Some(&native)));
    } else {
        add(window, &native);
    }
    // Gardée même si l'Explorateur est absent : `TaskbarCreated` la reposera.
    let previous = TRAY.with(|slot| slot.borrow_mut().replace(native));
    drop(previous);
    Ok(())
}

/// Retire l'icône de `window`, s'il y en a une ; sans effet la seconde fois.
pub(crate) fn remove(window: Hwnd) {
    if let Some(native) = TRAY.with(|slot| slot.borrow_mut().take()) {
        notify(NIM_DELETE, &data(window, None));
        drop(native);
    }
}

/// L'Explorateur a redémarré (`TaskbarCreated`) : la zone de notification est neuve.
pub(crate) fn restore(window: Hwnd) {
    TRAY.with(|slot| {
        if let Some(native) = slot.borrow().as_ref() {
            add(window, native);
        }
    });
}

/// Identifiant du message diffusé quand la barre des tâches est (re)créée.
pub(crate) fn taskbar_created() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| {
        let name: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();
        // SAFETY : nom terminé par zéro, valide pendant l'appel.
        unsafe { RegisterWindowMessageW(name.as_ptr()) }
    })
}

/// Message de rappel de l'icône (`TRAY_MESSAGE`) : avec la version 4, le mot bas de
/// `lparam` est l'événement et `wparam` porte la position du pointeur (écran, physique).
pub(crate) fn message(window: Hwnd, wparam: usize, lparam: isize) -> isize {
    match lparam as u32 & 0xFFFF {
        // Clic principal, ou Entrée et Espace sur l'icône sélectionnée au clavier.
        NIN_SELECT | NIN_KEYSELECT => runtime::tray_event(window, TrayEvent::Activated),
        WM_CONTEXTMENU => {
            let x = i32::from(wparam as u16 as i16);
            let y = i32::from((wparam >> 16) as u16 as i16);
            if let Some(id) = show_menu(window, x, y) {
                runtime::tray_event(window, TrayEvent::MenuItem(id));
            }
        }
        _ => {}
    }
    0
}

fn add(window: Hwnd, native: &NativeTray) {
    let mut data = data(window, Some(native));
    if notify(NIM_ADD, &data) {
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        notify(NIM_SETVERSION, &data);
    }
}

fn notify(operation: u32, data: &NOTIFYICONDATAW) -> bool {
    // SAFETY : structure locale complète, `cbSize` renseigné.
    unsafe { Shell_NotifyIconW(operation, data) != 0 }
}

/// Description de l'icône pour le Shell ; sans `native`, seule l'identité (retrait).
fn data(window: Hwnd, native: Option<&NativeTray>) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: TRAY_ID,
        ..Default::default()
    };
    if let Some(native) = native {
        // NIF_SHOWTIP : la version 4 n'affiche l'infobulle standard qu'avec lui.
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = TRAY_MESSAGE;
        data.hIcon = native.icon.handle();
        let room = data.szTip.len() - 1;
        for (slot, unit) in data
            .szTip
            .iter_mut()
            .zip(native.tray.tooltip.encode_utf16().take(room))
        {
            *slot = unit;
        }
    }
    data
}

/// Ouvre le menu de l'icône en (`x`, `y`) et rend l'identifiant de l'entrée choisie.
fn show_menu(window: Hwnd, x: i32, y: i32) -> Option<u32> {
    let items = TRAY.with(|slot| slot.borrow().as_ref().map(|n| n.tray.menu.clone()))?;
    if items.is_empty() {
        return None;
    }
    // SAFETY : menu créé, rempli, montré puis détruit ici ; textes terminés par zéro et
    // vivants pendant chaque appel ; fenêtre valide pendant son message.
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return None;
        }
        for (index, item) in items.iter().enumerate() {
            match item {
                TrayMenuItem::Item { label, enabled, .. } => {
                    let text: Vec<u16> = label.encode_utf16().chain(Some(0)).collect();
                    let flags = MF_STRING | if *enabled { 0 } else { MF_GRAYED };
                    AppendMenuW(menu, flags, index + 1, text.as_ptr());
                }
                TrayMenuItem::Separator => {
                    AppendMenuW(menu, MF_SEPARATOR, 0, null());
                }
            }
        }
        // Clair ou sombre comme le système (et les menus des autres icônes).
        super::menu_theme::follow_system(window);
        // Sans premier plan, le menu ne se fermerait pas quand on clique ailleurs.
        SetForegroundWindow(window);
        let align = if GetSystemMetrics(SM_MENUDROPALIGNMENT) != 0 {
            TPM_RIGHTALIGN
        } else {
            TPM_LEFTALIGN
        };
        let command = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | align,
            x,
            y,
            window,
            null(),
        );
        // Recommandé par la documentation du Shell : le menu se ferme au clic suivant.
        PostMessageW(window, WM_NULL, 0, 0);
        DestroyMenu(menu);
        chosen_item(&items, command)
    }
}

/// Entrée désignée par la commande d'un menu (rang + 1, `0` : menu fermé sans choix).
pub(crate) fn chosen_item(items: &[TrayMenuItem], command: i32) -> Option<u32> {
    let index = usize::try_from(command).ok()?.checked_sub(1)?;
    match items.get(index)? {
        TrayMenuItem::Item {
            id, enabled: true, ..
        } => Some(*id),
        _ => None,
    }
}
