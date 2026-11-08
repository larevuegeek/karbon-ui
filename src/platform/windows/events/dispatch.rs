//! Table de routage des messages Win32 vers leurs traitements (pointeur, clavier, texte,
//! zone non cliente, minuteries, peinture, fermeture).
use super::{
    super::{
        ffi::Hwnd,
        present::{self, paint, runtime},
        window::{change, shadow, timer},
    },
    accessibility, keyboard, nonclient, pointer, send_ui, tasks, text,
};
use crate::ui::runtime::UiEvent;
use windows_sys::Win32::UI::{
    Controls::WM_MOUSELEAVE,
    WindowsAndMessaging::{
        DefWindowProcW, WM_ACTIVATE, WM_CAPTURECHANGED, WM_CHAR, WM_CONTEXTMENU, WM_DESTROY,
        WM_DPICHANGED, WM_DWMCOMPOSITIONCHANGED, WM_ENTERSIZEMOVE, WM_ERASEBKGND, WM_EXITSIZEMOVE,
        WM_GETMINMAXINFO, WM_GETOBJECT, WM_IME_CHAR, WM_IME_COMPOSITION, WM_IME_ENDCOMPOSITION,
        WM_IME_SETCONTEXT, WM_IME_STARTCOMPOSITION, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
        WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCACTIVATE,
        WM_NCCALCSIZE, WM_NCHITTEST, WM_NCPAINT, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS,
        WM_SETTINGCHANGE, WM_SIZE, WM_SIZING, WM_TIMER, WM_UNICHAR, WM_WINDOWPOSCHANGED,
    },
};

/// # Safety
/// Arguments d'un message reçu par `window_proc` : `lparam` peut désigner une structure
/// fournie par Windows (WM_DPICHANGED, WM_GETMINMAXINFO).
pub(super) unsafe fn window_message(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if let Some(result) = accessibility::handle_message(window, message, wparam) {
        return result;
    }
    // Traitement Windows par défaut du message courant.
    // SAFETY : relais exact des arguments reçus de Windows pour cette même fenêtre,
    // pendant le traitement du message.
    let default_proc = || unsafe { DefWindowProcW(window, message, wparam, lparam) };
    match message {
        timer::FRAME_WAKE => {
            timer::frame_fired();
            // L'horloge est lue une fois la place de l'image prise dans la file de
            // présentation : intervalle constant entre lecture et affichage.
            runtime::wait_for_frame();
            send_ui(window, UiEvent::Timer(std::time::Instant::now()));
            0
        }
        tasks::TASK_WAKE => {
            tasks::deliver(window);
            0
        }
        WM_GETOBJECT => {
            accessibility::get_object(window, wparam, lparam).unwrap_or_else(default_proc)
        }
        WM_SETFOCUS => {
            send_ui(window, UiEvent::FocusChanged(true));
            0
        }
        WM_KILLFOCUS => {
            send_ui(window, UiEvent::Cancel);
            send_ui(window, UiEvent::FocusChanged(false));
            text::reset();
            0
        }
        // Minuteries : caret, synchronisation UI Automation, puis réveil d'animation.
        WM_TIMER => {
            if text::is_timer(wparam) {
                send_ui(window, UiEvent::CaretTick);
                0
            } else if accessibility::timer_fired(window, wparam) {
                0
            } else if timer::fired(window, wparam) {
                send_ui(window, UiEvent::Timer(std::time::Instant::now()));
                0
            } else {
                default_proc()
            }
        }
        WM_CHAR => text::character_message(window, wparam, default_proc),
        WM_UNICHAR => text::unicode_character(window, wparam),
        // SAFETY : arguments du message courant, relayés par `set_context`.
        WM_IME_SETCONTEXT => unsafe { text::set_context(window, message, wparam, lparam) },
        WM_IME_STARTCOMPOSITION => text::start_composition(window, default_proc),
        WM_IME_COMPOSITION => text::composition(window, lparam, default_proc),
        WM_IME_ENDCOMPOSITION => text::end_composition(window, default_proc),
        WM_IME_CHAR => text::ime_character(default_proc),
        WM_SETCURSOR => pointer::set_cursor(lparam, default_proc),
        WM_CONTEXTMENU => pointer::context_menu(window, lparam, default_proc),
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            pointer::wheel(window, message, wparam, lparam, default_proc)
        }
        WM_MOUSEMOVE => pointer::mouse_move(window, lparam),
        WM_MOUSELEAVE => pointer::mouse_leave(window),
        WM_LBUTTONDOWN => pointer::button_down(window, lparam),
        WM_LBUTTONUP => pointer::button_up(window, lparam),
        WM_CAPTURECHANGED => pointer::capture_changed(window),
        WM_KEYDOWN => keyboard::key_down(window, wparam, lparam, default_proc),
        WM_KEYUP => keyboard::key_up(window, wparam, default_proc),
        WM_ERASEBKGND => 1, // Aucun effacement GDI opaque.
        WM_ENTERSIZEMOVE => nonclient::enter_size_move(window),
        WM_EXITSIZEMOVE => nonclient::exit_size_move(window),
        // SAFETY : pour WM_DPICHANGED, lparam pointe vers un RECT fourni par Windows.
        WM_DPICHANGED => unsafe { nonclient::dpi_changed(window, lparam) },
        WM_ACTIVATE => nonclient::activate(window, default_proc),
        WM_WINDOWPOSCHANGED => nonclient::position_changed(window, default_proc),
        WM_DWMCOMPOSITIONCHANGED => nonclient::composition_changed(window),
        // Réglages système (dont « Afficher les animations dans Windows »).
        WM_SETTINGCHANGE => {
            present::settings_changed(window);
            default_proc()
        }
        WM_SIZE => nonclient::size(window),
        // Toute la fenêtre reste une zone de contenu : aucun cadre à afficher,
        // même avec WS_THICKFRAME. Windows conserve le mécanisme de resize.
        WM_NCCALCSIZE => 0,
        // Notre bande et nos boutons remplacent le dessin du cadre système.
        WM_NCPAINT => 0,
        WM_NCACTIVATE => 1, // Autoriser l'activation sans repeindre ce cadre.
        // SAFETY : pour WM_SIZING, lparam pointe vers le RECT modifiable du geste.
        WM_SIZING if unsafe { change::sizing(window, wparam, lparam) } => 1,
        // SAFETY : pour WM_GETMINMAXINFO, lparam pointe vers un MINMAXINFO modifiable.
        WM_GETMINMAXINFO => unsafe { nonclient::min_max_info(window, lparam) },
        WM_NCHITTEST => nonclient::hit_test(window, lparam, default_proc),
        WM_PAINT => {
            paint::paint_window(window);
            0
        }
        WM_DESTROY => {
            runtime::finish_profile();
            tasks::close();
            timer::shutdown(window);
            text::shutdown(window);
            accessibility::shutdown();
            shadow::destroy();
            0
        }
        // Windows gère les autres messages, dont la fermeture et le fond.
        _ => default_proc(),
    }
}
