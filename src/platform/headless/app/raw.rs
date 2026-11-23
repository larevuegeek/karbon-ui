//! Entrées brutes simulées, comme les donne une fenêtre : les boutons de la souris pour
//! `on_pointer` (et le panneau qui prend le clavier), le pointeur verrouillé, les touches
//! pour `on_key` et `Application::key`.
use super::HeadlessApp;
use crate::{
    application::Application,
    ui::{
        runtime::{KeyPress, PointerButton, PointerPhase, raw_input::PointerInput},
        scene::PointF,
    },
};

/// Modifier keys held during a raw pointer input of a [`HeadlessApp`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawModifiers {
    /// Shift is held.
    pub shift: bool,
    /// Ctrl is held.
    pub control: bool,
    /// Alt is held.
    pub alt: bool,
}

impl<A: Application> HeadlessApp<A> {
    /// Give `on_pointer` handlers a button of the mouse pressed (`Down`), the pointer
    /// moved (`Move`, `button` `None`) or a button released (`Up`) at `at`, as a window
    /// does, then prepare the next frame. A press also gives the keyboard to the panel
    /// under the pointer ([`Element::on_key`](crate::ui::element::Element::on_key)).
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn raw_pointer(
        &mut self,
        phase: PointerPhase,
        button: Option<PointerButton>,
        at: PointF,
        modifiers: RawModifiers,
    ) -> crate::Result<()> {
        self.raw(phase, button, at, modifiers, None)
    }

    /// Move the mouse by `motion` while the pointer is locked
    /// ([`Element::pointer_lock`](crate::ui::element::Element::pointer_lock)): the
    /// position stays at `at`, the handler reads `motion` in its `delta`.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn raw_locked_move(&mut self, at: PointF, motion: (f32, f32)) -> crate::Result<()> {
        let modifiers = RawModifiers::default();
        self.raw(PointerPhase::Move, None, at, modifiers, Some(motion))
    }

    /// Whether the gesture in progress asks for the pointer to be locked: a window would
    /// hide and hold the pointer now.
    pub fn pointer_locked(&self) -> bool {
        self.ui.pointer_lock_wanted()
    }

    /// Give `press` to the keys of the application, as a window does: to the
    /// [`on_key`](crate::ui::element::Element::on_key) handlers of the panel that has the
    /// focus, then to [`Application::key`]. Then prepare the next frame.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn raw_key(&mut self, press: KeyPress) -> crate::Result<()> {
        self.prepare()?;
        self.ui.key_input(press);
        self.prepare()
    }

    /// Drag files from another program over the window, out of it, or drop them at `at`
    /// ([`Element::on_file_drop`](crate::ui::element::Element::on_file_drop)), then
    /// prepare the next frame. Returns whether an element accepted them.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn drag_files(
        &mut self,
        phase: crate::ui::runtime::FileDropPhase,
        paths: &[std::path::PathBuf],
        at: PointF,
    ) -> crate::Result<bool> {
        self.prepare()?;
        let paths: std::sync::Arc<[std::path::PathBuf]> = paths.into();
        let accepted = self.ui.file_drag(phase, &paths, at);
        self.prepare()?;
        Ok(accepted)
    }

    fn raw(
        &mut self,
        phase: PointerPhase,
        button: Option<PointerButton>,
        at: PointF,
        modifiers: RawModifiers,
        motion: Option<(f32, f32)>,
    ) -> crate::Result<()> {
        self.prepare()?;
        self.ui.pointer_input(PointerInput {
            phase,
            button,
            position: at,
            shift: modifiers.shift,
            control: modifiers.control,
            alt: modifiers.alt,
            meta: false,
            motion,
        });
        self.prepare()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{prelude::*, runtime::KeyCode};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Turned(f32),
        Deleted,
    }

    #[derive(Default)]
    struct View {
        log: Vec<Msg>,
    }
    impl Application for View {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            column("view", [])
                .width(Length::Px(200.))
                .height(Length::Px(200.))
                .on_pointer(|e: crate::ui::runtime::PointerEvent| {
                    (e.button == Some(PointerButton::Secondary) || e.phase == PointerPhase::Move)
                        .then_some(Msg::Turned(e.delta.0))
                })
                .on_key(|press: KeyPress| (press.key == KeyCode::Delete).then_some(Msg::Deleted))
                .pointer_lock(true)
        }
        fn update(&mut self, message: Msg) -> bool {
            self.log.push(message);
            false
        }
    }

    #[test]
    fn a_window_simulates_a_locked_turn_and_a_key_given_to_the_view() {
        let mut app = HeadlessApp::new(View::default()).unwrap();
        let at = PointF { x: 50., y: 50. };
        let right = Some(PointerButton::Secondary);
        let none = RawModifiers::default();
        app.raw_pointer(PointerPhase::Down, right, at, none)
            .unwrap();
        assert!(app.pointer_locked());
        app.raw_locked_move(at, (12., 0.)).unwrap();
        app.raw_pointer(PointerPhase::Up, right, at, none).unwrap();
        assert!(!app.pointer_locked());
        app.raw_key(KeyPress::new(KeyCode::Delete, true)).unwrap();
        assert_eq!(
            app.app().log,
            [
                Msg::Turned(0.),
                Msg::Turned(12.),
                Msg::Turned(0.),
                Msg::Deleted
            ]
        );
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;
    use crate::ui::{
        prelude::*,
        runtime::{FileDropEvent, FileDropPhase},
    };
    use std::path::PathBuf;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Hover(u8, bool),
        Import(u8, Vec<PathBuf>),
    }

    /// Deux zones : la vue (1) prend les modèles `.glb`, le navigateur (2) tout fichier.
    #[derive(Default)]
    struct Drops {
        log: Vec<Msg>,
    }
    impl Application for Drops {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            let zone = |name: &'static str, k: u8, models_only: bool| {
                column(name, [])
                    .width(Length::Px(100.))
                    .height(Length::Px(100.))
                    .on_file_drop(move |drop: &FileDropEvent| {
                        let ok = !models_only
                            || drop
                                .paths
                                .iter()
                                .all(|p| p.extension().is_some_and(|e| e == "glb"));
                        match drop.phase {
                            FileDropPhase::Leave => Some(Msg::Hover(k, false)),
                            FileDropPhase::Drop if ok => Some(Msg::Import(k, drop.paths.to_vec())),
                            _ if ok => Some(Msg::Hover(k, true)),
                            _ => None,
                        }
                    })
            };
            row("page", [zone("view", 1, true), zone("browser", 2, false)])
        }
        fn update(&mut self, message: Msg) -> bool {
            self.log.push(message);
            false
        }
    }

    #[test]
    fn dropped_files_reach_the_element_that_accepts_them_with_their_full_paths() {
        let mut app = HeadlessApp::new(Drops::default()).unwrap();
        let view = PointF { x: 50., y: 50. };
        let browser = PointF { x: 150., y: 50. };
        let files: Vec<PathBuf> = ["C:/a/caisse.glb", "C:/a/tonneau.glb", "C:/a/pierre.glb"]
            .into_iter()
            .map(PathBuf::from)
            .collect();
        assert!(app.drag_files(FileDropPhase::Over, &files, view).unwrap());
        // Vers le navigateur : la vue est quittée.
        assert!(
            app.drag_files(FileDropPhase::Over, &files, browser)
                .unwrap()
        );
        assert!(
            app.drag_files(FileDropPhase::Drop, &files, browser)
                .unwrap()
        );
        assert_eq!(
            app.app().log,
            [
                Msg::Hover(1, true),
                Msg::Hover(2, true),
                Msg::Hover(1, false),
                Msg::Import(2, files.clone()),
            ]
        );
        // Témoin : une image sur la vue est refusée (curseur interdit), rien n'arrive.
        let image = [PathBuf::from("C:/a/herbe.png")];
        let before = app.app().log.len();
        assert!(!app.drag_files(FileDropPhase::Over, &image, view).unwrap());
        assert!(!app.drag_files(FileDropPhase::Drop, &image, view).unwrap());
        assert_eq!(app.app().log.len(), before);
        // Un glisser annulé : la zone survolée reçoit la sortie.
        app.drag_files(FileDropPhase::Over, &image, browser)
            .unwrap();
        app.drag_files(FileDropPhase::Leave, &image, browser)
            .unwrap();
        assert_eq!(app.app().log.last(), Some(&Msg::Hover(2, false)));
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::*;
    use crate::ui::prelude::*;
    use std::cell::Cell;

    /// Une application qui note la largeur et l'échelle que lui donne sa vue.
    struct Zoomed {
        zoom: f32,
        seen: Cell<(f32, f32)>,
    }
    impl Application for Zoomed {
        type Message = f32;
        fn view(&self, context: &ViewContext) -> Element {
            self.seen.set((context.width, context.scale));
            column("page", [])
        }
        fn update(&mut self, zoom: f32) -> bool {
            self.zoom = zoom;
            true
        }
        fn interface_zoom(&self) -> f32 {
            self.zoom
        }
    }

    #[test]
    fn the_zoom_of_the_application_multiplies_the_display_scale() {
        let options = crate::platform::headless::HeadlessOptions {
            width: 800.,
            height: 600.,
            scale: 1.5,
            ..Default::default()
        };
        let app = Zoomed {
            zoom: 1.,
            seen: Cell::new((0., 0.)),
        };
        let mut window = HeadlessApp::with_options(app, options).unwrap();
        assert_eq!(window.app().seen.get(), (800., 1.5));
        window.send(1.25).unwrap();
        window.prepare().unwrap();
        assert_eq!(window.app().seen.get(), (640., 1.875));
        // Hors des bornes : tenu entre 0,5 et 3 ; un nombre impossible vaut 1.
        window.send(10.).unwrap();
        window.prepare().unwrap();
        assert_eq!(window.app().seen.get().1, 4.5);
        window.send(f32::NAN).unwrap();
        window.prepare().unwrap();
        assert_eq!(window.app().seen.get(), (800., 1.5));
    }
}
