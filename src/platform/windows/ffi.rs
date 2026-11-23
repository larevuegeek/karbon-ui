//! Types partagés du backend Win32. Les déclarations viennent de windows-sys,
//! généré par Microsoft depuis les métadonnées du SDK : aucune signature recopiée ici.
//! Aussi : échelle DPI d'une fenêtre et appartement COM du thread.
pub(super) use crate::ui::geometry::{Point, Rect};
use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{InvalidateRect, ScreenToClient},
    UI::HiDpi::GetDpiForWindow,
};

// Identifiant opaque d'une fenêtre : son contenu est géré par Windows.
pub(super) type Hwnd = HWND;

/// Échec Win32 de `operation`, d'après `GetLastError`, converti en HRESULT.
pub(super) fn last_error(operation: &'static str) -> crate::Error {
    let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    win32_error(operation, code as u32)
}
/// Erreur `Platform` pour un code Win32, selon `HRESULT_FROM_WIN32` (0 devient E_FAIL).
pub(super) fn win32_error(operation: &'static str, code: u32) -> crate::Error {
    crate::Error::Platform {
        operation,
        code: hresult_from_win32(code),
    }
}
pub(super) const fn hresult_from_win32(code: u32) -> i32 {
    const E_FAIL: u32 = 0x8000_4005;
    if code == 0 {
        E_FAIL as i32
    } else if code as i32 <= 0 {
        code as i32
    } else {
        ((code & 0xFFFF) | 0x8007_0000) as i32
    }
}

// Conversions explicites entre la géométrie du moteur et les structures Windows.
pub(super) const fn rect_from(area: RECT) -> Rect {
    Rect {
        left: area.left,
        top: area.top,
        right: area.right,
        bottom: area.bottom,
    }
}
pub(super) const fn win_rect(area: Rect) -> RECT {
    RECT {
        left: area.left,
        top: area.top,
        right: area.right,
        bottom: area.bottom,
    }
}
pub(super) const fn win_point(point: Point) -> POINT {
    POINT {
        x: point.x,
        y: point.y,
    }
}

/// Zone client en pixels physiques ; `None` si Windows refuse la lecture.
pub(super) fn client_area(window: Hwnd) -> Option<Rect> {
    let mut area = RECT::default();
    // SAFETY : `area` est une structure locale valide en écriture pendant l'appel.
    let read =
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(window, &mut area) };
    (read != 0).then(|| rect_from(area))
}

/// Convertit un point écran en coordonnées client ; `None` en cas d'échec.
pub(super) fn screen_to_client(window: Hwnd, point: Point) -> Option<Point> {
    let mut point = win_point(point);
    // SAFETY : `point` est une structure locale valide en lecture et écriture.
    (unsafe { ScreenToClient(window, &mut point) } != 0).then_some(Point {
        x: point.x,
        y: point.y,
    })
}

/// Échelle DPI de la fenêtre (1,0 pour 96 ppp).
pub(super) fn scale(window: Hwnd) -> f32 {
    // SAFETY : simple lecture ; Windows renvoie 0 pour un handle invalide.
    (unsafe { GetDpiForWindow(window) }.max(96) as f32) / 96. * zoom::of(window)
}

/// Zoom de l'interface choisi par l'application de chaque fenêtre
/// (`Application::interface_zoom`), multiplié à l'échelle de l'écran par `scale` ; 1 tant
/// que l'application n'en demande pas d'autre.
pub(super) mod zoom {
    use super::Hwnd;
    use std::cell::RefCell;

    thread_local! {
        static ZOOMS: RefCell<Vec<(Hwnd, f32)>> = const { RefCell::new(Vec::new()) };
    }

    pub(in crate::platform::windows) fn of(window: Hwnd) -> f32 {
        ZOOMS.with(|z| {
            z.borrow()
                .iter()
                .find(|(w, _)| *w == window)
                .map_or(1., |(_, z)| *z)
        })
    }

    /// Retient le zoom de `window` ; `true` quand il a changé (tout est à refaire).
    pub(in crate::platform::windows) fn set(window: Hwnd, zoom: f32) -> bool {
        if (of(window) - zoom).abs() < 1e-6 {
            return false;
        }
        ZOOMS.with(|z| {
            let mut zooms = z.borrow_mut();
            zooms.retain(|(w, _)| *w != window);
            if zoom != 1. {
                zooms.push((window, zoom));
            }
        });
        true
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn a_window_keeps_its_zoom_until_it_changes() {
            let window = 0x1234usize as super::Hwnd;
            assert_eq!(super::of(window), 1.);
            assert!(super::set(window, 1.25));
            assert_eq!(super::of(window), 1.25);
            assert!(!super::set(window, 1.25));
            // Revenu à 1 : oublié, comme une fenêtre qui n'en a jamais demandé.
            assert!(super::set(window, 1.));
            assert_eq!(super::of(window), 1.);
        }
    }
}

/// Demande un redessin complet, sans effacement du fond.
pub(super) fn invalidate(window: Hwnd) {
    // SAFETY : un rectangle nul désigne toute la zone client ; Windows valide le handle.
    unsafe {
        InvalidateRect(
            window,
            std::ptr::null(),
            windows_sys::Win32::Foundation::FALSE,
        )
    };
}

/// Appartement COM monothread du thread courant, libéré au drop s'il a été initialisé ici.
pub(super) struct ComApartment {
    owned: bool,
}
impl ComApartment {
    pub(super) fn initialize() -> crate::Result<Self> {
        // SAFETY: initialisation STA du thread courant, équilibrée par Drop si possédée.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result.is_err() && result != RPC_E_CHANGED_MODE {
            return Err(crate::Error::Platform {
                operation: "CoInitializeEx",
                code: result.0,
            });
        }
        Ok(Self {
            owned: result.is_ok(),
        })
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: équilibre l'appel CoInitializeEx réussi sur ce même thread.
            unsafe { CoUninitialize() };
        }
    }
}
