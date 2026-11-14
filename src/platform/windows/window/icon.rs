//! Icônes natives (`HICON`) tirées d'une image encodée : icône de la fenêtre (barre des
//! tâches, Alt+Tab, menu de la fenêtre) et de la zone de notification.
use super::super::{
    ffi::{Hwnd, last_error},
    imaging::DrawingEngine,
};
use crate::ui::scene::Image;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
        DeleteObject,
    },
    UI::{
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        WindowsAndMessaging::{
            CreateIconIndirect, DestroyIcon, HICON, ICON_BIG, ICON_SMALL, ICONINFO, SM_CXICON,
            SM_CXSMICON, SendMessageW, WM_SETICON,
        },
    },
};

/// Icône possédée, détruite avec la valeur (après la fenêtre ou l'entrée de la zone de
/// notification qui l'affiche).
pub(crate) struct Icon(HICON);
impl Icon {
    pub(crate) fn handle(&self) -> HICON {
        self.0
    }
}
impl Drop for Icon {
    fn drop(&mut self) {
        // SAFETY : icône créée par CreateIconIndirect, détruite une seule fois.
        unsafe { DestroyIcon(self.0) };
    }
}

/// Icône carrée de `edge` pixels tirée de `bytes` : l'image est réduite à `edge` pixels sur
/// son grand côté (jamais agrandie) puis centrée.
pub(crate) fn from_image(decoder: &DrawingEngine, bytes: &[u8], edge: u32) -> crate::Result<Icon> {
    let image = decoder.scaled(bytes, edge)?;
    let pixels = square_straight_alpha(&image, edge);
    let header = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: edge as i32,
        // Hauteur négative : lignes de haut en bas, comme les pixels décodés.
        biHeight: -(edge as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..Default::default()
    };
    let info = BITMAPINFO {
        bmiHeader: header,
        ..Default::default()
    };
    let mut bits = null_mut();
    // SAFETY : en-tête local valide ; Windows alloue la section et écrit son adresse.
    let color =
        unsafe { CreateDIBSection(null_mut(), &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
    if color.is_null() || bits.is_null() {
        return Err(last_error("CreateDIBSection (icon)"));
    }
    // SAFETY : la section fait `edge * edge * 4` octets, la taille de `pixels`.
    unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len()) };
    // Masque monochrome nul : la couche alpha de l'image couleur décide seule.
    // SAFETY : bitmap monochrome alloué par Windows, sans pixels fournis.
    let mask = unsafe { CreateBitmap(edge as i32, edge as i32, 1, 1, null()) };
    let icon = if mask.is_null() {
        Err(last_error("CreateBitmap (icon mask)"))
    } else {
        let parts = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        // SAFETY : les deux bitmaps sont valides ; Windows les copie dans l'icône.
        let icon = unsafe { CreateIconIndirect(&parts) };
        if icon.is_null() {
            Err(last_error("CreateIconIndirect"))
        } else {
            Ok(Icon(icon))
        }
    };
    // SAFETY : bitmaps créés ci-dessus, copiés par CreateIconIndirect, rendus une fois.
    unsafe {
        DeleteObject(color);
        if !mask.is_null() {
            DeleteObject(mask);
        }
    }
    icon
}

/// Pose l'icône de la fenêtre aux deux tailles du système, au DPI de la fenêtre : grande
/// (barre des tâches, Alt+Tab) et petite (menu de la fenêtre). Les icônes rendues doivent
/// vivre autant que la fenêtre.
pub(crate) fn set_window_icon(
    window: Hwnd,
    decoder: &DrawingEngine,
    bytes: &[u8],
) -> crate::Result<Vec<Icon>> {
    // SAFETY : fenêtre valide créée sur ce thread.
    let dpi = unsafe { GetDpiForWindow(window) };
    let mut icons = Vec::with_capacity(2);
    for (metric, kind) in [(SM_CXICON, ICON_BIG), (SM_CXSMICON, ICON_SMALL)] {
        // SAFETY : simple lecture d'une métrique système.
        let edge = unsafe { GetSystemMetricsForDpi(metric, dpi) }.max(16) as u32;
        let icon = from_image(decoder, bytes, edge)?;
        // SAFETY : fenêtre valide ; l'icône vit autant que la fenêtre (gardée par l'appelant).
        unsafe { SendMessageW(window, WM_SETICON, kind as usize, icon.handle() as isize) };
        icons.push(icon);
    }
    Ok(icons)
}

/// Pixels BGRA à alpha droit, ce qu'attend une icône 32 bits, l'image (BGRA prémultiplié)
/// centrée dans un carré transparent de `edge` pixels.
pub(crate) fn square_straight_alpha(image: &Image, edge: u32) -> Vec<u8> {
    let edge = edge as usize;
    let (width, height) = (image.width as usize, image.height as usize);
    let (left, top) = (
        edge.saturating_sub(width) / 2,
        edge.saturating_sub(height) / 2,
    );
    let mut out = vec![0_u8; edge * edge * 4];
    for y in 0..height.min(edge) {
        for x in 0..width.min(edge) {
            let source = (y * width + x) * 4;
            let target = ((top + y) * edge + left + x) * 4;
            let pixel = &image.pixels[source..source + 4];
            let alpha = pixel[3];
            for channel in 0..3 {
                out[target + channel] = if alpha == 0 {
                    0
                } else {
                    ((u32::from(pixel[channel]) * 255 + u32::from(alpha) / 2) / u32::from(alpha))
                        .min(255) as u8
                };
            }
            out[target + 3] = alpha;
        }
    }
    out
}
