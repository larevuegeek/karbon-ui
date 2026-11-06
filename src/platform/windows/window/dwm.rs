//! Effets du compositeur DWM : cadre étendu, coins arrondis, bordure système masquée.
use super::super::ffi::Hwnd;
use windows_sys::Win32::{
    Graphics::Dwm::{
        DWMNCRP_ENABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmExtendFrameIntoClientArea,
        DwmSetWindowAttribute,
    },
    UI::{Controls::MARGINS, WindowsAndMessaging::IsZoomed},
};

pub(crate) fn update_frame(window: Hwnd) -> bool {
    // Un cadre étendu très fin permet de conserver les effets du compositeur
    // autour de notre surface personnalisée. L'ombre visible est calculée
    // séparément dans window/shadow.rs : cette marge ne définit pas son flou.
    // Aucun débordement n'est nécessaire lorsque la fenêtre est agrandie.
    // SAFETY : simple lecture de l'état de la fenêtre.
    let thickness = if unsafe { IsZoomed(window) } != 0 {
        0
    } else {
        1
    };
    let margins = MARGINS {
        cxLeftWidth: thickness,
        cxRightWidth: thickness,
        cyTopHeight: thickness,
        cyBottomHeight: thickness,
    };
    // SAFETY : `margins` est une structure locale valide pendant l'appel.
    let result = unsafe { DwmExtendFrameIntoClientArea(window, &margins) };
    if result < 0 {
        eprintln!(
            "karbon-ui: DwmExtendFrameIntoClientArea failed (HRESULT 0x{:08X})",
            result as u32
        );
        return false;
    }
    true
}

pub(crate) fn apply_style(window: Hwnd) {
    let mut available = true;
    for (attribute, value, effect) in [
        (
            DWMWA_NCRENDERING_POLICY,
            DWMNCRP_ENABLED as u32,
            "frame composition",
        ),
        (
            DWMWA_WINDOW_CORNER_PREFERENCE,
            DWMWCP_ROUND as u32,
            "rounded corners",
        ),
        (DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, "hidden system border"),
    ] {
        // SAFETY : DWM lit la valeur de 32 bits pendant cet appel ; la taille correspond.
        let result = unsafe {
            DwmSetWindowAttribute(
                window,
                attribute as u32,
                (&value as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            )
        };
        if result < 0 {
            // Ces effets restent facultatifs : la fenêtre fonctionne aussi sans eux.
            eprintln!(
                "karbon-ui: DWM effect `{effect}` unavailable (HRESULT 0x{:08X})",
                result as u32
            );
            available = false;
        }
    }
    if update_frame(window) && available {
        println!("karbon-ui: DWM style applied: rounded corners");
    }
}
