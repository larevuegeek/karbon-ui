//! Surface DIB persistante pour présenter les pixels BGRA prémultipliés.
use super::super::ffi::{Hwnd, last_error};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{POINT, SIZE},
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
        CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
        HGDIOBJ, RGBQUAD, SelectObject,
    },
    UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow},
};

pub(crate) struct LayeredSurface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    width: i32,
    height: i32,
}
impl Drop for LayeredSurface {
    fn drop(&mut self) {
        // SAFETY : objets créés par `new` et possédés par cette surface ; le bitmap est
        // retiré du DC avant sa libération, chaque handle n'est libéré qu'une fois.
        unsafe {
            if !self.previous.is_null() {
                SelectObject(self.dc, self.previous);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
        }
    }
}
impl LayeredSurface {
    pub(crate) fn new(width: i32, height: i32) -> crate::Result<Self> {
        if width <= 0 || height <= 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid layered surface size"));
        }
        let mut s = Self {
            // SAFETY : un DC nul demande un DC mémoire compatible avec l'écran.
            dc: unsafe { CreateCompatibleDC(null_mut()) },
            bitmap: null_mut(),
            previous: null_mut(),
            pixels: null_mut(),
            width,
            height,
        };
        if s.dc.is_null() {
            return Err(last_error("CreateCompatibleDC"));
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
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
        let mut pixels = null_mut();
        // SAFETY : `info` décrit un DIB 32 bits complet ; Windows écrit l'adresse des pixels.
        s.bitmap =
            unsafe { CreateDIBSection(s.dc, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0) };
        if s.bitmap.is_null() || pixels.is_null() {
            return Err(last_error("CreateDIBSection"));
        }
        s.pixels = pixels.cast();
        // SAFETY : DC et bitmap valides, créés ci-dessus.
        s.previous = unsafe { SelectObject(s.dc, s.bitmap) };
        // HGDI_ERROR vaut -1 : aucune sélection à restaurer.
        if s.previous.is_null() || s.previous as isize == -1 {
            s.previous = null_mut();
            return Err(last_error("SelectObject"));
        }
        Ok(s)
    }
    pub(super) fn matches(&self, width: i32, height: i32) -> bool {
        self.width == width && self.height == height
    }
    pub(crate) fn pixels_mut(&mut self) -> &mut [u8] {
        // SAFETY : le DIB possède cette allocation, top-down et contiguë à 32 bits par pixel.
        // L'emprunt exclusif empêche present ou une destruction pendant l'écriture.
        unsafe {
            std::slice::from_raw_parts_mut(
                self.pixels,
                self.width as usize * self.height as usize * 4,
            )
        }
    }
    pub(super) fn present(&self, window: Hwnd) -> crate::Result<()> {
        let size = SIZE {
            cx: self.width,
            cy: self.height,
        };
        let origin = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        // Position nulle : conserver celle de la fenêtre, y compris pendant un déplacement natif.
        // SAFETY : structures locales valides pendant l'appel ; le DC contient le DIB.
        if unsafe {
            UpdateLayeredWindow(
                window,
                null_mut(),
                std::ptr::null(),
                &size,
                self.dc,
                &origin,
                0,
                &blend,
                ULW_ALPHA,
            )
        } == 0
        {
            return Err(last_error("UpdateLayeredWindow"));
        }
        Ok(())
    }
}
