//! Classe et création d'une fenêtre native avec son chemin de présentation.
use super::super::{
    events::window_proc,
    ffi::{Hwnd, last_error, win32_error},
    graphics::Renderer,
    present::composition::{self, Path, Presenter},
};
use crate::window::WindowOptions;
use std::{
    ptr::{null, null_mut},
    sync::OnceLock,
};
use windows_sys::Win32::{
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        HiDpi::GetDpiForSystem,
        WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, IDC_ARROW, LoadCursorW, RegisterClassW, WNDCLASSW,
            WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
            WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
        },
    },
};

static CLASS_READY: OnceLock<Result<(), i32>> = OnceLock::new();

/// Crée la fenêtre avec son chemin de présentation. La composition est validée jusqu'à
/// sa première chaîne d'échange ; en mode Auto, un échec recrée une fenêtre layered.
pub(super) fn create_presented_window(
    options: WindowOptions,
    renderer: &mut Renderer,
) -> crate::Result<(Hwnd, Presenter)> {
    if composition::requested(&options) == Path::Composition {
        let error = match create_window(options, Path::Composition) {
            Ok(window) => match composition::attach(window, renderer, &options) {
                Ok(visual) => return Ok((window, Presenter::Composition(visual))),
                Err(error) => {
                    // Fenêtre pas encore enregistrée : aucun état applicatif à libérer.
                    // SAFETY : fenêtre créée ci-dessus sur ce thread ; Windows valide le handle.
                    unsafe { DestroyWindow(window) };
                    error
                }
            },
            Err(error) => error,
        };
        if !composition::falls_back(&options) {
            return Err(error);
        }
        eprintln!(
            "karbon-ui: DirectComposition unavailable ({error}), falling back to layered presentation"
        );
    }
    Ok((
        create_window(options, Path::Layered)?,
        Presenter::Layered(None),
    ))
}

fn create_window(options: WindowOptions, path: Path) -> crate::Result<Hwnd> {
    let class_name: Vec<u16> = "KarbonUiWindow\0".encode_utf16().collect();
    // SAFETY : un nom nul désigne le module de l'exécutable courant.
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err(last_error("GetModuleHandleW"));
    }
    // IDC_ARROW est un identifiant de ressource, selon MAKEINTRESOURCE.
    // SAFETY : curseur système partagé, sans module propriétaire.
    let cursor = unsafe { LoadCursorW(null_mut(), IDC_ARROW) };
    if cursor.is_null() {
        return Err(last_error("LoadCursorW"));
    }
    let class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: null_mut(),
        hCursor: cursor,
        hbrBackground: null_mut(), // Le renderer fournit tous les pixels, alpha compris.
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
    };
    if let Err(code) = CLASS_READY.get_or_init(|| {
        // SAFETY : `class` et son nom terminé par zéro restent valides pendant l'appel.
        if unsafe { RegisterClassW(&class) } == 0 {
            Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(1))
        } else {
            Ok(())
        }
    }) {
        return Err(win32_error("RegisterClassW", *code as u32));
    }
    let title: Vec<u16> = options.title.encode_utf16().chain(Some(0)).collect();
    // SAFETY : simple lecture du DPI système.
    let scale = unsafe { GetDpiForSystem() } as f32 / 96.;
    let extended = match path {
        // Alpha par pixel fourni par UpdateLayeredWindow.
        Path::Layered => WS_EX_LAYERED,
        // Aucune surface de redirection GDI : DirectComposition fournit tout le contenu.
        Path::Composition => WS_EX_NOREDIRECTIONBITMAP,
    } | if options.topmost { WS_EX_TOPMOST } else { 0 }
        | if options.activate {
            WS_EX_APPWINDOW
        } else {
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
        }
        | if options.click_through {
            WS_EX_TRANSPARENT
        } else {
            0
        };
    // SAFETY : les textes et la classe restent valides pendant cet appel.
    let window = unsafe {
        CreateWindowExW(
            extended,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX,
            (options.x as f32 * scale).round() as i32,
            (options.y as f32 * scale).round() as i32,
            (options.width as f32 * scale).round() as i32,
            (options.height as f32 * scale).round() as i32,
            null_mut(),
            null_mut(),
            instance,
            null(),
        )
    };
    if window.is_null() {
        return Err(last_error("CreateWindowExW"));
    }
    Ok(window)
}
