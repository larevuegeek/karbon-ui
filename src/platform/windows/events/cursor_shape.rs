//! Le curseur choisi par l'application pour un élément (`Element::cursor_shape`) : un
//! curseur nommé, aucun curseur, ou une image faite curseur par `CreateIconIndirect`.
//! L'image faite est gardée tant que l'application donne la même : `WM_SETCURSOR` arrive
//! à chaque mouvement de la souris.
use crate::ui::{
    runtime::{CursorImage, CursorShape},
    scene::Image,
};
use std::{cell::RefCell, ptr::null_mut};
use windows_sys::Win32::{
    Graphics::Gdi::{CreateBitmap, DeleteObject},
    UI::WindowsAndMessaging::{CreateIconIndirect, DestroyIcon, HCURSOR, ICONINFO, SetCursor},
};

/// Le curseur fait d'une image, et ce qui l'a fait (l'image et son point de visée).
struct Made {
    from: CursorImage,
    cursor: HCURSOR,
}

impl Drop for Made {
    fn drop(&mut self) {
        // SAFETY : curseur fait par `CreateIconIndirect` sur ce thread, détruit une fois.
        unsafe { DestroyIcon(self.cursor) };
    }
}

thread_local! {
    static MADE: RefCell<Option<Made>> = const { RefCell::new(None) };
}

/// Montre `shape` ; `false` quand il faut le curseur ordinaire (une image refusée par
/// Windows).
pub(super) fn show(shape: &CursorShape, named: impl FnOnce(crate::ui::runtime::Cursor)) -> bool {
    match shape {
        CursorShape::Named(cursor) => {
            named(*cursor);
            true
        }
        CursorShape::Hidden => {
            // SAFETY : un curseur nul cache le curseur tant que la souris est sur la zone.
            unsafe { SetCursor(null_mut()) };
            true
        }
        CursorShape::Image(image) => match cursor_of(image) {
            Some(cursor) => {
                // SAFETY : `cursor` est gardé dans `MADE` tant qu'il est montré.
                unsafe { SetCursor(cursor) };
                true
            }
            None => false,
        },
    }
}

/// Le curseur fait de `image`, gardé d'un appel à l'autre.
fn cursor_of(image: &CursorImage) -> Option<HCURSOR> {
    MADE.with(|made| {
        let mut made = made.borrow_mut();
        let same = made.as_ref().is_some_and(|m| m.from == *image);
        if !same {
            *made = make(&image.image, image.hotspot).map(|cursor| Made {
                from: image.clone(),
                cursor,
            });
        }
        made.as_ref().map(|m| m.cursor)
    })
}

/// Un curseur de Windows fait des pixels de `image` (BGRA, alpha prémultiplié) visant
/// `hotspot`.
fn make(image: &Image, hotspot: (u32, u32)) -> Option<HCURSOR> {
    let (width, height) = (image.width as i32, image.height as i32);
    // Le masque d'un curseur en couleurs à alpha ne sert pas, mais il en faut un : tout à
    // zéro, en lignes alignées sur 16 bits.
    let mask_row = image.width.div_ceil(16) as usize * 2;
    let mask = vec![0u8; mask_row * image.height as usize];
    // SAFETY : `mask` et `image.pixels` couvrent exactement les lignes demandées (1 bit,
    // puis 32 bits par pixel) et vivent pendant les appels, qui en font des copies.
    let (mask, color) = unsafe {
        (
            CreateBitmap(width, height, 1, 1, mask.as_ptr().cast()),
            CreateBitmap(width, height, 1, 32, image.pixels.as_ptr().cast()),
        )
    };
    let info = ICONINFO {
        fIcon: 0,
        xHotspot: hotspot.0,
        yHotspot: hotspot.1,
        hbmMask: mask,
        hbmColor: color,
    };
    let made = (!mask.is_null() && !color.is_null())
        // SAFETY : `info` décrit deux images valides ; Windows les recopie.
        .then(|| unsafe { CreateIconIndirect(&info) })
        .filter(|cursor| !cursor.is_null());
    // SAFETY : les deux images ont été recopiées dans le curseur (ou n'ont pas servi).
    unsafe {
        DeleteObject(mask);
        DeleteObject(color);
    }
    made
}
