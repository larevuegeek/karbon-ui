//! Chemins de présentation sur des fenêtres cachées : composition (WARP et GPU),
//! redimensionnement, perte du périphérique, test de contenu et repli layered.
use super::super::{
    ffi::client_area,
    present::{
        composition::{self, Path},
        runtime,
    },
    window::{state, take_error},
};
use super::*;
use crate::{ui::element::Length, window::PresentMode};
use windows_sys::Win32::{
    Foundation::POINT,
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
        DestroyWindow, GWL_EXSTYLE, GetWindowLongPtrW, HTTRANSPARENT, IsWindow, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOZORDER, SendMessageW, SetWindowPos, WM_NCHITTEST, WM_PAINT,
        WS_EX_LAYERED, WS_EX_NOREDIRECTIONBITMAP,
    },
};

struct Card;
impl Application for Card {
    type Message = ();
    fn update(&mut self, _: ()) -> bool {
        false
    }
    fn view(&self, _: &ViewContext) -> Element {
        // Contenu borné : le reste d'une fenêtre transparente reste vide.
        Element::column(
            1,
            vec![
                widgets::text(2, "Composition")
                    .width(Length::Px(120.))
                    .height(Length::Px(24.)),
            ],
        )
    }
}

fn open(
    session: &Session,
    id: u64,
    configure: impl FnOnce(&mut WindowOptions),
) -> crate::Result<Hwnd> {
    let mut options = WindowOptions {
        title: "Karbon UI - présentation cachée",
        width: 300,
        height: 220,
        min_width: 200,
        min_height: 160,
        activate: false,
        renderer: RenderMode::Software,
        ..WindowOptions::DEFAULT
    };
    configure(&mut options);
    session.open(WindowDescriptor::new(WindowId(id), options, Card), false)
}
fn extended_style(window: Hwnd) -> u32 {
    // SAFETY : fenêtre de test de ce thread ; simple lecture.
    unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 }
}
fn paint(window: Hwnd) {
    // SAFETY : fenêtre de test de ce thread ; aucun pointeur dans wparam/lparam.
    unsafe { SendMessageW(window, WM_PAINT, 0, 0) };
}
/// Résultat de WM_NCHITTEST pour un point client en pixels physiques.
fn hit(window: Hwnd, x: i32, y: i32) -> isize {
    let mut point = POINT { x, y };
    // SAFETY : fenêtre de test de ce thread ; `point` est valide en lecture et écriture.
    assert_ne!(unsafe { ClientToScreen(window, &mut point) }, 0);
    let packed = (point.x as u16 as u32 | ((point.y as u16 as u32) << 16)) as isize;
    // SAFETY : fenêtre de test de ce thread ; lparam ne contient que des coordonnées.
    unsafe { SendMessageW(window, WM_NCHITTEST, 0, packed) }
}
fn client_size(window: Hwnd) -> (u32, u32) {
    let area = client_area(window).unwrap();
    (area.right as u32, area.bottom as u32)
}

fn verify_composition(renderer: RenderMode) {
    let (_apartment, session) = session();
    let window = open(&session, 1, |options| {
        options.renderer = renderer;
        options.profile = true;
    })
    .unwrap();
    let _scope = state::enter(window);
    let style = extended_style(window);
    assert_ne!(style & WS_EX_NOREDIRECTIONBITMAP, 0);
    assert_eq!(style & WS_EX_LAYERED, 0);
    let (width, height) = client_size(window);
    // Fenêtre opaque non agrandie : coins arrondis transparents, alpha prémultiplié.
    assert_eq!(
        runtime::presentation_state(),
        Some((Path::Composition, Some((width, height, true))))
    );
    let report = runtime::profile_report().unwrap();
    assert!(report.contains("\"presentation\":\"composition\","));
    assert!(report.contains("\"frames\":1,"));
    assert!(report.contains("\"surface_creations\":1,"));

    // Les coins arrondis du fond laissent passer le clic ; le fond, non.
    assert_eq!(hit(window, 1, 1), HTTRANSPARENT as isize);
    assert_ne!(
        hit(window, width as i32 / 2, height as i32 / 2),
        HTTRANSPARENT as isize
    );

    // Redimensionnement : ResizeBuffers sur la même chaîne.
    assert_ne!(
        // SAFETY : fenêtre de test de ce thread ; aucun pointeur transmis.
        unsafe {
            SetWindowPos(
                window,
                null_mut(),
                0,
                0,
                340,
                260,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        },
        0
    );
    paint(window);
    let (width, height) = client_size(window);
    assert_eq!(
        runtime::presentation_state(),
        Some((Path::Composition, Some((width, height, true))))
    );
    let report = runtime::profile_report().unwrap();
    assert!(report.contains("\"surface_creations\":2,"));
    assert!(report.contains("\"size_changes\":1,"));

    // Perte du périphérique pendant Present : renderer, chaîne et rattachement recréés.
    runtime::simulate_device_loss();
    paint(window);
    // SAFETY : simple vérification du handle de test.
    assert_ne!(unsafe { IsWindow(window) }, 0);
    let report = runtime::profile_report().unwrap();
    assert!(report.contains("\"renderer_recoveries\":1,"), "{report}");
    assert!(report.contains("\"failed_paints\":0,"));
    assert_eq!(
        runtime::presentation_state(),
        Some((Path::Composition, Some((width, height, true))))
    );
    paint(window);
    assert!(take_error().is_none());
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}

#[test]
fn software_windows_present_through_composition_resize_and_recover_device_loss() {
    verify_composition(RenderMode::Software);
}

#[test]
#[ignore = "nécessite un adaptateur GPU matériel ; lancer explicitement avec --include-ignored"]
fn gpu_windows_present_through_composition_resize_and_recover_device_loss() {
    verify_composition(RenderMode::Gpu);
}

#[test]
fn layered_path_is_used_for_click_through_explicit_choice_and_failed_composition() {
    let (_apartment, session) = session();
    let layered = |window: Hwnd| {
        let _scope = state::enter(window);
        let style = extended_style(window);
        assert_ne!(style & WS_EX_LAYERED, 0);
        assert_eq!(style & WS_EX_NOREDIRECTIONBITMAP, 0);
        assert_eq!(runtime::presentation_state(), Some((Path::Layered, None)));
        // Windows teste lui-même l'alpha : aucun HTTRANSPARENT ajouté par le moteur.
        assert_ne!(hit(window, 1, 1), HTTRANSPARENT as isize);
        paint(window);
        // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
        assert_ne!(unsafe { DestroyWindow(window) }, 0);
    };
    layered(
        open(&session, 1, |options| {
            options.transparent = true;
            options.click_through = true;
        })
        .unwrap(),
    );
    layered(
        open(&session, 2, |options| {
            options.presentation = PresentMode::Layered
        })
        .unwrap(),
    );
    let invalid = open(&session, 3, |options| {
        options.transparent = true;
        options.click_through = true;
        options.presentation = PresentMode::Composition;
    });
    assert!(matches!(
        invalid,
        Err(crate::Error::InvalidWindowOptions { .. })
    ));

    composition::tests::FAIL.with(|fail| fail.set(true));
    layered(open(&session, 4, |_| {}).unwrap());
    let required = open(&session, 5, |options| {
        options.presentation = PresentMode::Composition;
    });
    composition::tests::FAIL.with(|fail| fail.set(false));
    assert!(required.is_err());
    assert!(state::windows().is_empty());
    assert!(take_error().is_none());

    // Fenêtre transparente : seule la zone peinte reçoit les clics.
    let window = open(&session, 6, |options| options.transparent = true).unwrap();
    {
        let _scope = state::enter(window);
        let (width, height) = client_size(window);
        assert_eq!(
            runtime::presentation_state(),
            Some((Path::Composition, Some((width, height, true))))
        );
        assert_eq!(
            hit(window, width as i32 - 2, height as i32 - 2),
            HTTRANSPARENT as isize
        );
    }
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}
