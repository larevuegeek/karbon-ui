//! Outils communs : mesureurs de texte, mise en page directe, runtime préparé, peinture.
//!
//! Deux mesureurs coexistent : `Measure` (0,5 em par caractère, lignes de 1,2 em) et
//! `HeadlessTextMeasurer`, les métriques du backend headless (0,6 em, lignes de 1,25 em).
//! Les fonctions `*_headless` utilisent le second.
use karbon_ui::{
    application::Resources,
    platform::headless::{HeadlessOptions, HeadlessTextMeasurer},
    ui::{
        element::{PaintContext, SizeF, TextMeasurer},
        layout,
        prelude::*,
        runtime::{ChromeState, UiRuntime},
        scene::{DrawCommand, Image, RectF, Scene},
        styling::{self, StyleEnvironment},
    },
};
use std::{sync::Arc, time::Instant};

/// Messages inertes : ces tests vérifient le style, le layout et le hit-test.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Msg {
    Click(u8),
    Text(String),
    Value(f64),
    Checked(bool),
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
pub fn runtime(view: fn() -> Element) -> UiRuntime {
    UiRuntime::new(View(view), &Services).unwrap()
}
fn prepare_with(ui: &mut UiRuntime, now: Instant, text: &mut dyn TextMeasurer) {
    ui.prepare_at(
        ViewContext {
            width: 600.,
            height: 400.,
            theme: Theme::DARK,
            transparent: true,
            backend: "test",
            scale: 1.,
        },
        ChromeState {
            visible: false,
            ..Default::default()
        },
        text,
        now,
    )
    .unwrap();
}
/// Prépare une vue de 600 × 400 (thème sombre, sans barre de titre), mesureur fixe.
pub fn prepare(ui: &mut UiRuntime, now: Instant) {
    prepare_with(ui, now, &mut Measure);
}
/// Comme `prepare`, avec les métriques headless.
pub fn prepare_headless(ui: &mut UiRuntime, now: Instant) {
    prepare_with(ui, now, &mut HeadlessTextMeasurer::new());
}
/// Scène complète d'une vue, préparée par le runtime (fonds, bordures, textes).
pub fn scene(view: fn() -> Element) -> Scene {
    let mut ui = runtime(view);
    prepare_headless(&mut ui, Instant::now());
    ui.scene().clone()
}
/// Opacité du premier effet de la scène (1 sans effet).
pub fn opacity(ui: &UiRuntime) -> f32 {
    ui.scene()
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::PushEffect { opacity, .. } => Some(*opacity),
            _ => None,
        })
        .unwrap_or(1.)
}

/// Styles (environnement par défaut, sans fenêtre), puis mise en page w × h avec le
/// mesureur fixe.
pub fn arrange(root: &mut Element, w: f32, h: f32) {
    styling::resolve(root, StyleEnvironment::default()).unwrap();
    layout::validate(root).unwrap();
    let r = RectF::new(0., 0., w, h);
    layout::arrange(root, r, r, &mut Measure).unwrap();
}
/// Styles dans `env`, puis mise en page w × h avec les métriques headless.
pub fn arrange_headless_in(root: &mut Element, w: f32, h: f32, env: StyleEnvironment) {
    styling::resolve(root, env).unwrap();
    layout::validate(root).unwrap();
    let r = RectF::new(0., 0., w, h);
    layout::arrange(root, r, r, &mut HeadlessTextMeasurer::new()).unwrap();
}
/// Fenêtre de w × h (requêtes média, unités vw/vh), métriques headless.
pub fn arrange_headless(root: &mut Element, w: f32, h: f32) {
    arrange_headless_in(
        root,
        w,
        h,
        StyleEnvironment {
            viewport: SizeF {
                width: w,
                height: h,
            },
            ..Default::default()
        },
    );
}

fn paint_context(node: &Element) -> PaintContext {
    PaintContext {
        rect: node.bounds(),
        intrinsic: SizeF::default(),
        theme: Theme::DARK,
        state: Default::default(),
    }
}
/// Dessin du widget de `node` seul.
pub fn paint(node: &Element) -> Scene {
    let mut s = Scene::new(Color::hex(0));
    node.widget
        .as_ref()
        .unwrap()
        .paint(&paint_context(node), &mut s);
    s
}
/// Dessin des widgets du sous-arbre, en ordre du document.
pub fn paint_tree(node: &Element) -> Scene {
    fn walk(node: &Element, s: &mut Scene) {
        if let Some(widget) = &node.widget {
            widget.paint(&paint_context(node), s);
        }
        for child in &node.children {
            walk(child, s);
        }
    }
    let mut s = Scene::new(Color::hex(0));
    walk(node, &mut s);
    s
}
/// Couleur de texte du premier texte peint dans le sous-arbre.
pub fn text_color(node: &Element) -> Color {
    paint_tree(node)
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::FormattedText { color, .. } | DrawCommand::FontText { color, .. } => {
                Some(*color)
            }
            DrawCommand::RichText { color, .. } => Some(*color),
            _ => None,
        })
        .unwrap()
}

pub fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.02, "{a} != {b}");
}
pub fn height(r: RectF) -> f32 {
    r.bottom - r.top
}
/// Fenêtre headless de `width` × `height`.
pub fn options(width: f32, height: f32) -> HeadlessOptions {
    HeadlessOptions {
        width,
        height,
        ..Default::default()
    }
}
/// Pixel BGRA (x, y) d'une image rendue.
pub fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * image.width + x) * 4) as usize;
    image.pixels[offset..offset + 4].try_into().unwrap()
}
