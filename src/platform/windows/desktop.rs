//! Le bureau sous Windows : les écrans (`EnumDisplayMonitors`) et la boîte de message du
//! système (`MessageBoxW`), pour `window::monitors` et `window::message_box`.
use crate::window::{MessageAnswer, MessageButtons, MessageIcon, Monitor};
use windows_sys::Win32::{
    Foundation::{LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::{
            IDCANCEL, IDNO, IDOK, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION,
            MB_ICONWARNING, MB_OK, MB_OKCANCEL, MB_SETFOREGROUND, MB_TASKMODAL, MB_YESNO,
            MB_YESNOCANCEL, MONITORINFOF_PRIMARY, MessageBoxW,
        },
    },
};

/// Ce que décrit un écran, lu au système.
fn describe(monitor: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY : structure locale complète, cbSize renseigné ; l'écran vient de
    // l'énumération en cours.
    if unsafe { GetMonitorInfoW(monitor, (&raw mut info).cast()) } == 0 {
        return None;
    }
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    // SAFETY : sorties locales ; un échec laisse 96.
    unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let r = info.monitorInfo.rcMonitor;
    let w = info.monitorInfo.rcWork;
    let end = info
        .szDevice
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(info.szDevice.len());
    Some(Monitor {
        name: String::from_utf16_lossy(&info.szDevice[..end]),
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
        work_area: (w.left, w.top, w.right - w.left, w.bottom - w.top),
        scale: dpi_x.max(96) as f32 / 96.,
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
    // SAFETY : `data` est l'adresse du vecteur passé par `monitors`, vivant pendant
    // l'énumération, qui se fait sur ce fil.
    let found = unsafe { &mut *(data as *mut Vec<Monitor>) };
    found.extend(describe(monitor));
    1
}

pub(crate) fn monitors() -> crate::Result<Vec<Monitor>> {
    let mut found: Vec<Monitor> = Vec::new();
    // SAFETY : tout le bureau, rappel ci-dessus, donnée : le vecteur local.
    let ok = unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(collect),
            (&raw mut found) as LPARAM,
        )
    };
    if ok == 0 {
        return Err(super::ffi::last_error("EnumDisplayMonitors"));
    }
    found.sort_by_key(|m| !m.primary);
    Ok(found)
}

/// Les styles de la boîte pour `icon` et `buttons`.
fn style(icon: MessageIcon, buttons: MessageButtons) -> u32 {
    let icon = match icon {
        MessageIcon::Information => MB_ICONINFORMATION,
        MessageIcon::Warning => MB_ICONWARNING,
        MessageIcon::Error => MB_ICONERROR,
        MessageIcon::Question => MB_ICONQUESTION,
    };
    let buttons = match buttons {
        MessageButtons::Ok => MB_OK,
        MessageButtons::OkCancel => MB_OKCANCEL,
        MessageButtons::YesNo => MB_YESNO,
        MessageButtons::YesNoCancel => MB_YESNOCANCEL,
    };
    icon | buttons | MB_TASKMODAL | MB_SETFOREGROUND
}

/// La réponse de la boîte d'après le bouton rendu par le système.
fn answer(code: i32) -> MessageAnswer {
    match code {
        IDOK => MessageAnswer::Ok,
        IDYES => MessageAnswer::Yes,
        IDNO => MessageAnswer::No,
        IDCANCEL => MessageAnswer::Cancel,
        _ => MessageAnswer::Cancel,
    }
}

pub(crate) fn message_box(
    title: &str,
    text: &str,
    icon: MessageIcon,
    buttons: MessageButtons,
) -> crate::Result<MessageAnswer> {
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (title, text) = (wide(title), wide(text));
    // SAFETY : textes terminés par un nul, lus pendant l'appel ; boîte sans
    // propriétaire, modale aux fenêtres de ce programme (MB_TASKMODAL).
    let code = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            style(icon, buttons),
        )
    };
    if code == 0 {
        return Err(super::ffi::last_error("MessageBoxW"));
    }
    Ok(answer(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_displays_are_listed_with_the_main_one_first() {
        let found = monitors().unwrap();
        assert!(!found.is_empty());
        assert!(found[0].primary);
        assert_eq!(found.iter().filter(|m| m.primary).count(), 1);
        for m in &found {
            assert!(m.width > 0 && m.height > 0 && m.scale >= 1., "{m:?}");
            let (_, _, w, h) = m.work_area;
            assert!(w <= m.width && h <= m.height, "{m:?}");
        }
    }

    #[test]
    fn styles_and_answers_follow_the_buttons() {
        assert_eq!(
            style(MessageIcon::Question, MessageButtons::YesNo),
            MB_ICONQUESTION | MB_YESNO | MB_TASKMODAL | MB_SETFOREGROUND
        );
        assert_eq!(answer(IDYES), MessageAnswer::Yes);
        assert_eq!(answer(IDNO), MessageAnswer::No);
        assert_eq!(answer(IDCANCEL), MessageAnswer::Cancel);
        assert_eq!(answer(IDOK), MessageAnswer::Ok);
    }
}
