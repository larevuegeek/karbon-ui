//! Ombre explicite : une fenêtre transparente, non interactive, derrière la principale.
//! Le dégradé est calculé en Rust et envoyé à Windows comme une image BGRA.
use super::super::ffi::{Hwnd, Point, Rect, client_area, last_error, rect_from, scale, win_point};
use crate::ui::{
    geometry::Size,
    rendering::shadow::{SHADOW_MARGIN, generate_pixels},
};
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows_sys::Win32::{
    Foundation::{POINT, RECT, SIZE},
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
        CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
        HGDIOBJ, RGBQUAD, SelectObject,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        HiDpi::{
            DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            SetThreadDpiAwarenessContext,
        },
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, HTTRANSPARENT, IsIconic,
            IsWindowVisible, IsZoomed, MA_NOACTIVATE, RegisterClassW, SW_HIDE, SWP_NOACTIVATE,
            SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SetWindowPos, ShowWindow, ULW_ALPHA,
            UpdateLayeredWindow, WM_MOUSEACTIVATE, WM_NCHITTEST, WNDCLASSW, WS_EX_LAYERED,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

static SHADOW: super::state::WindowLocal<RefCell<Option<ShadowWindow>>> =
    super::state::WindowLocal::new(|state| &state.shadow);
static CLASS_READY: OnceLock<bool> = OnceLock::new();

pub(crate) struct ShadowWindow {
    window: Hwnd,
    image: Option<(i32, i32, i32)>, // Taille du corps et échelle, pas sa position.
}

// Restaurer le contexte DPI même si une opération échoue plus bas.
struct DpiContext(DPI_AWARENESS_CONTEXT);
impl Drop for DpiContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY : contexte renvoyé par SetThreadDpiAwarenessContext sur ce thread.
            unsafe { SetThreadDpiAwarenessContext(self.0) };
        }
    }
}

// Ces ressources sont toujours retirées du DC avant d'être libérées.
struct Surface {
    context: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}
impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY : objets créés par upload_image et possédés ici ; chaque handle est
        // libéré une seule fois, après avoir rendu au DC sa sélection initiale.
        unsafe {
            if !self.previous.is_null() {
                SelectObject(self.context, self.previous);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
            DeleteDC(self.context);
        }
    }
}

unsafe extern "system" fn shadow_proc(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match message {
        WM_NCHITTEST => HTTRANSPARENT as isize, // Aucun clic n'est intercepté.
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize, // Ne jamais prendre le focus.
        // SAFETY : relais des arguments reçus de Windows pour cette même fenêtre.
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn create(position: &Rect) -> crate::Result<ShadowWindow> {
    // SAFETY : un nom nul désigne le module de l'exécutable courant.
    let instance = unsafe { GetModuleHandleW(null()) };
    let name: Vec<u16> = "KarbonUiShadow\0".encode_utf16().collect();
    let ready = CLASS_READY.get_or_init(|| {
        let class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(shadow_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: name.as_ptr(),
        };
        // SAFETY : `class` et son nom terminé par zéro restent valides pendant l'appel.
        unsafe { RegisterClassW(&class) != 0 }
    });
    if !ready {
        return Err(last_error("RegisterClassW (shadow)"));
    }
    // Une couche graphique sans entrée dans la barre des tâches et sans interaction
    // avec la souris.
    // SAFETY : la classe et le nom restent valides pendant cet appel.
    let window = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            name.as_ptr(),
            null(),
            WS_POPUP,
            position.left,
            position.top,
            1,
            1,
            null_mut(),
            null_mut(),
            instance,
            null(),
        )
    };
    if window.is_null() {
        return Err(last_error("CreateWindowExW"));
    }
    Ok(ShadowWindow {
        window,
        image: None,
    })
}

fn upload_image(window: Hwnd, position: &Point, size: &Size, pixels: &[u32]) -> crate::Result<()> {
    // SAFETY : un DC nul demande un DC mémoire compatible avec l'écran.
    let context = unsafe { CreateCompatibleDC(null_mut()) };
    if context.is_null() {
        return Err(last_error("CreateCompatibleDC"));
    }
    let mut surface = Surface {
        context,
        bitmap: null_mut(),
        previous: null_mut(),
    };
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size.width,
            biHeight: -size.height, // Lignes de haut en bas.
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default()],
    };
    let mut destination = null_mut();
    // SAFETY : `info` décrit un DIB 32 bits complet ; Windows écrit l'adresse des pixels.
    surface.bitmap = unsafe {
        CreateDIBSection(
            context,
            &info,
            DIB_RGB_COLORS,
            &mut destination,
            null_mut(),
            0,
        )
    };
    if surface.bitmap.is_null() || destination.is_null() {
        return Err(last_error("CreateDIBSection"));
    }
    // SAFETY : DC et bitmap valides, créés ci-dessus.
    surface.previous = unsafe { SelectObject(context, surface.bitmap) };
    if surface.previous.is_null() {
        return Err(last_error("SelectObject"));
    }
    // SAFETY : le DIB possède exactement largeur * hauteur pixels de 32 bits.
    unsafe {
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), destination.cast::<u32>(), pixels.len())
    };
    let position = win_point(*position);
    let size = SIZE {
        cx: size.width,
        cy: size.height,
    };
    let origin = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    // SAFETY : structures locales valides pendant l'appel ; le DC contient le DIB.
    if unsafe {
        UpdateLayeredWindow(
            window,
            null_mut(),
            &position,
            &size,
            context,
            &origin,
            0,
            &blend,
            ULW_ALPHA,
        )
    } == 0
    {
        return Err(last_error("UpdateLayeredWindow"));
    }
    Ok(()) // La fenêtre conserve l'image ; le DIB temporaire est libéré.
}

pub(crate) fn update(main_window: Hwnd) {
    // SAFETY : simples lectures d'état ; Windows valide le handle.
    let visible = unsafe {
        IsWindowVisible(main_window) != 0
            && IsIconic(main_window) == 0
            && IsZoomed(main_window) == 0
    } && !super::change::fullscreen();
    if !visible {
        SHADOW.with(|state| {
            if let Ok(state) = state.try_borrow()
                && let Some(shadow) = state.as_ref()
            {
                // SAFETY : fenêtre d'ombre possédée par cet état, sur ce thread.
                unsafe { ShowWindow(shadow.window, SW_HIDE) };
            }
        });
        return;
    }

    // La fenêtre principale utilise des coordonnées logiques ; la couche alpha
    // travaille en pixels physiques pour rester nette avec la mise à l'échelle.
    if client_area(main_window).is_none_or(|client| client.right <= 0) {
        return;
    }
    // SAFETY : constante prédéfinie ; le contexte précédent est restauré par DpiContext.
    let previous_dpi =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if previous_dpi.is_null() {
        eprintln!(
            "karbon-ui: shadow DPI context unavailable: {}",
            last_error("SetThreadDpiAwarenessContext")
        );
        return;
    }
    let _dpi = DpiContext(previous_dpi);
    let mut area = RECT::default();
    // SAFETY : `area` est une structure locale valide en écriture.
    if unsafe { GetWindowRect(main_window, &mut area) } == 0 {
        return;
    }
    let position = rect_from(area);
    let width = position.right - position.left;
    let height = position.bottom - position.top;
    if width <= 0 || height <= 0 {
        return;
    }
    let scale = scale(main_window);
    let margin = (SHADOW_MARGIN * scale).ceil() as i32;
    let destination = Point {
        x: position.left - margin,
        y: position.top - margin,
    };

    SHADOW.with(|state| {
        // Un appel Windows peut provoquer un message imbriqué : éviter tout emprunt réentrant.
        let Ok(mut state) = state.try_borrow_mut() else {
            return;
        };
        if state.is_none() {
            match create(&position) {
                Ok(shadow) => *state = Some(shadow),
                Err(error) => {
                    eprintln!("karbon-ui: creating the window shadow failed: {error}");
                    return;
                }
            }
        }
        let Some(shadow) = state.as_mut() else { return };
        let key = (width, height, (scale * 100.0).round() as i32);
        if shadow.image != Some(key) && super::state::sizing() {
            // Redimensionnement à la souris : recalculer et envoyer l'image (fenêtre
            // entière plus marges) à chaque pas coûtait ~20 ms. L'ombre native de DWM
            // reste visible ; la nôtre revient à WM_EXITSIZEMOVE.
            // SAFETY : fenêtre d'ombre possédée par cet état, sur ce thread.
            unsafe { ShowWindow(shadow.window, SW_HIDE) };
            shadow.image = None;
            return;
        }
        if shadow.image != Some(key) {
            let (size, pixels) = generate_pixels(width, height, scale);
            if let Err(error) = upload_image(shadow.window, &destination, &size, &pixels) {
                eprintln!("karbon-ui: drawing the window shadow failed: {error}");
                return;
            }
            shadow.image = Some(key);
        }
        // Placer la couche juste DERRIÈRE la principale, sans l'activer.
        // L'image est réutilisée lors d'un simple déplacement.
        // SAFETY : deux fenêtres de ce thread ; aucun pointeur transmis.
        unsafe {
            SetWindowPos(
                shadow.window,
                main_window,
                destination.x,
                destination.y,
                width + 2 * margin,
                height + 2 * margin,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOOWNERZORDER,
            );
        }
    });
}

/// Crée l'ombre de la fenêtre courante sans l'afficher (les fenêtres de test sont
/// cachées, donc sans ombre) et renvoie sa fenêtre.
#[cfg(test)]
pub(crate) fn create_hidden() -> Hwnd {
    let shadow = create(&Rect {
        left: 0,
        top: 0,
        right: 1,
        bottom: 1,
    })
    .unwrap();
    let window = shadow.window;
    SHADOW.with(|state| *state.borrow_mut() = Some(shadow));
    window
}

pub(crate) fn destroy() {
    let shadow = SHADOW.with(|state| state.borrow_mut().take());
    if let Some(shadow) = shadow {
        // SAFETY : fenêtre d'ombre retirée de l'état, détruite une seule fois.
        unsafe { DestroyWindow(shadow.window) };
    }
}
