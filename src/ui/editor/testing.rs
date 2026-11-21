//! Outils des tests des composants d'éditeur : une fenêtre simulée (`HeadlessApp`) et
//! les entrées brutes que la fenêtre Windows donnerait (`on_pointer`, touches).
use crate::{
    application::Application,
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        runtime::{KeyCode, KeyPress, PointerButton, PointerPhase, raw_input::PointerInput},
        scene::PointF,
    },
};

/// Une fenêtre simulée de `width` × `height`, préparée.
pub(super) fn window<A: Application>(app: A, width: f32, height: f32) -> HeadlessApp<A> {
    HeadlessApp::with_options(
        app,
        HeadlessOptions {
            width,
            height,
            ..Default::default()
        },
    )
    .unwrap()
}

/// Modificateurs tenus pendant une entrée.
#[derive(Clone, Copy, Default)]
pub(super) struct Mods {
    pub(super) shift: bool,
    pub(super) control: bool,
}

pub(super) const NONE: Mods = Mods {
    shift: false,
    control: false,
};
pub(super) const SHIFT: Mods = Mods {
    shift: true,
    control: false,
};
pub(super) const CONTROL: Mods = Mods {
    shift: false,
    control: true,
};

/// Une entrée `on_pointer` à `at`, puis la préparation de l'image suivante.
pub(super) fn pointer<A: Application>(
    window: &mut HeadlessApp<A>,
    phase: PointerPhase,
    button: Option<PointerButton>,
    at: PointF,
    mods: Mods,
) {
    window.prepare().unwrap();
    window.runtime_mut().pointer_input(PointerInput {
        phase,
        button,
        position: at,
        shift: mods.shift,
        control: mods.control,
        alt: false,
    });
    window.prepare().unwrap();
}

/// Appui puis relâchement du bouton gauche à `at`.
pub(super) fn click<A: Application>(window: &mut HeadlessApp<A>, at: PointF, mods: Mods) {
    let left = Some(PointerButton::Primary);
    pointer(window, PointerPhase::Down, left, at, mods);
    pointer(window, PointerPhase::Up, left, at, mods);
}

/// Une touche enfoncée, donnée à `Application::key`, puis l'image suivante.
pub(super) fn key<A: Application>(window: &mut HeadlessApp<A>, key: KeyCode, mods: Mods) {
    window.prepare().unwrap();
    let press = KeyPress {
        shift: mods.shift,
        control: mods.control,
        ..KeyPress::new(key, true)
    };
    window.runtime_mut().key_input(press);
    window.prepare().unwrap();
}

/// Point à `dx`, `dy` du coin haut gauche du rectangle de l'élément `id`.
pub(super) fn at<A: Application>(
    window: &HeadlessApp<A>,
    id: impl Into<crate::ui::element::ElementId>,
    dx: f32,
    dy: f32,
) -> PointF {
    let r = window.bounds(id).expect("element on screen");
    PointF {
        x: r.left + dx,
        y: r.top + dy,
    }
}
