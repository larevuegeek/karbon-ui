//! Minuteries et cadence des images : réveil d'animation, présentation de la dernière
//! image, aucun minuteur armé pour une fenêtre cachée.
use super::super::{
    present::{paint, runtime},
    window::{state, timer},
};
use super::*;
use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;

#[test]
fn timer_requests_presentation_even_when_prepare_already_sampled_the_animation() {
    struct Animated(u32);
    impl Application for Animated {
        type Message = ();
        fn update(&mut self, _: ()) -> bool {
            false
        }
        fn view(&self, _: &ViewContext) -> Element {
            widgets::text(1, "Animation")
                .animate(crate::ui::animation::Animation::fade_in().duration(self.0))
        }
    }
    let (_apartment, session) = session();
    for duration in [10_000, 1] {
        let mut descriptor = descriptor(duration as u64, app());
        descriptor.application = Box::new(Animated(duration));
        let window = session.open(descriptor, false).unwrap();
        let _scope = state::enter(window);
        if duration == 1 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Le timestamp du message précède le prepare exécuté par runtime::input.
        let reply = runtime::input(
            window,
            crate::ui::runtime::UiEvent::Timer(std::time::Instant::now()),
        );
        assert!(
            reply.repaint,
            "l'image intermédiaire et la dernière image doivent être présentées"
        );
        assert_eq!(runtime::next_wake().is_none(), duration == 1);
        if duration == 1 {
            // Le réveil est différé : l'image est préparée et présentée par WM_PAINT.
            paint::paint_window(window);
            assert!(
                !runtime::input(
                    window,
                    crate::ui::runtime::UiEvent::Timer(std::time::Instant::now()),
                )
                .repaint
            );
        }
        // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
        assert_ne!(unsafe { DestroyWindow(window) }, 0);
    }
}

#[test]
fn system_reduced_motion_is_read_once_and_again_after_a_setting_change() {
    use super::super::present::{SYSTEM_REDUCED_MOTION, system_reduced_motion};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_SETTINGCHANGE};
    let (_apartment, session) = session();
    let window = session.open(descriptor(1, app()), false).unwrap();
    let actual = system_reduced_motion();
    // Valeur mémorisée : pas de nouvelle lecture du système.
    SYSTEM_REDUCED_MOTION.with(|cached| cached.set(Some(!actual)));
    assert_eq!(system_reduced_motion(), !actual);
    // SAFETY : fenêtre de test créée sur ce thread ; message sans structure associée.
    unsafe { SendMessageW(window, WM_SETTINGCHANGE, 0, 0) };
    assert_eq!(SYSTEM_REDUCED_MOTION.with(std::cell::Cell::get), None);
    assert_eq!(system_reduced_motion(), actual);
    // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
    assert_ne!(unsafe { DestroyWindow(window) }, 0);
}

#[test]
fn reduced_motion_option_finishes_native_entry_and_hidden_windows_do_not_arm_timers() {
    struct Animated;
    impl Application for Animated {
        type Message = ();
        fn update(&mut self, _: ()) -> bool {
            false
        }
        fn view(&self, _: &ViewContext) -> Element {
            widgets::text(1, "Animation")
                .animate(crate::ui::animation::Animation::fade_in().duration(10_000))
        }
    }
    let (_apartment, session) = session();
    for reduced in [false, true] {
        let mut descriptor = descriptor(if reduced { 2 } else { 1 }, app());
        descriptor.options.reduced_motion = reduced;
        descriptor.application = Box::new(Animated);
        let window = session.open(descriptor, false).unwrap();
        let _scope = state::enter(window);
        assert_eq!(runtime::next_wake().is_none(), reduced);
        timer::sync(window);
        state::WindowLocal::new(|state| &state.deadline)
            .with(|deadline| assert!(deadline.get().is_none()));
        // SAFETY : fenêtre de test créée sur ce thread ; Windows valide le handle.
        assert_ne!(unsafe { DestroyWindow(window) }, 0);
    }
}
