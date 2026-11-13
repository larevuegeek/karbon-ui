//! Outils communs : messages, mesureur fixe, runtime 800 × 600 (thème clair, métriques
//! headless), recherche d'éléments et de dessins.
use karbon_ui::{
    application::Resources,
    platform::headless::HeadlessTextMeasurer,
    ui::{
        element::{ElementId, SizeF, TextMeasurer},
        layout,
        prelude::*,
        runtime::{ChromeState, UiRuntime},
        scene::{DrawCommand, Image, RectF, Scene},
        styling::{self, StyleEnvironment},
    },
};
use std::{sync::Arc, time::Instant};

/// Messages des vues de test et de la démo Bootstrap (`examples/bootstrap_demo/page.ui`).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Msg {
    Noop,
    Click,
    Text(String),
    Pick(u64),
    Check(bool),
    Switch(bool),
    Range(f64),
    Window(karbon_ui::window::WindowAction),
}

/// Mesureur fixe : 0,5 em par caractère, lignes de 1,2 em.
pub struct Measure;
impl TextMeasurer for Measure {
    fn measure_text(&mut self, s: &str, size: f32, _: bool, max: f32) -> karbon_ui::Result<SizeF> {
        Ok(SizeF {
            width: (s.chars().count() as f32 * size * 0.5).min(max),
            height: size * 1.2,
        })
    }
}

pub struct Services;
impl Resources for Services {
    fn decode_image(&self, _: &[u8]) -> karbon_ui::Result<Arc<Image>> {
        unreachable!()
    }
}
/// Application dont la vue est une fonction sans état.
pub struct View(pub fn() -> Element);
impl Application for View {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        (self.0)()
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

/// Styles dans une fenêtre de w × h, puis mise en page avec le mesureur fixe.
pub fn arrange(root: &mut Element, w: f32, h: f32) {
    let env = StyleEnvironment {
        viewport: SizeF {
            width: w,
            height: h,
        },
        ..Default::default()
    };
    styling::resolve(root, env).unwrap();
    layout::validate(root).unwrap();
    let r = RectF::new(0., 0., w, h);
    layout::arrange(root, r, r, &mut Measure).unwrap();
}
pub fn find<'a>(root: &'a Element, id: &str) -> &'a Element {
    root.find(ElementId::named(id))
        .unwrap_or_else(|| panic!("element {id}"))
}
pub fn computed(e: &Element) -> styling::ComputedStyle {
    e.computed_style().expect("styles").clone()
}

/// Prépare `ui` à l'instant `now` (800 × 600, thème clair, sans barre de titre).
pub fn prepare_at(ui: &mut UiRuntime, now: Instant) {
    ui.prepare_at(
        ViewContext {
            width: 800.,
            height: 600.,
            theme: Theme::LIGHT,
            transparent: false,
            backend: "test",
            scale: 1.,
        },
        ChromeState {
            visible: false,
            ..Default::default()
        },
        &mut HeadlessTextMeasurer::new(),
        now,
    )
    .unwrap();
}
/// Runtime préparé (styles, mise en page, peinture) d'une vue de 800 × 600.
pub fn runtime(view: fn() -> Element) -> UiRuntime {
    let mut ui = UiRuntime::new(View(view), &Services).unwrap();
    prepare_at(&mut ui, Instant::now());
    ui
}
/// Rectangles des images SVG peintes par la scène.
pub fn svgs(scene: &Scene) -> Vec<RectF> {
    scene
        .commands
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Svg { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}
pub fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.05, "{a} != {b}");
}
