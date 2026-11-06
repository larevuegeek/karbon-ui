//! Outils partagés des tests de contrat : messages, application de vue, mesureur
//! déterministe, applications d'exemple et aides d'événements.
pub(super) use crate::application::*;
pub(super) use crate::ui::{
    element::*,
    layout,
    runtime::*,
    scene::{Image, PointF, RectF, Scene},
    theme::Theme,
    widgets::{self, Button, Text},
};
pub(super) use std::{cell::Cell, rc::Rc, sync::Arc};

/// Messages des tests : identifiant du contrôle et valeur transportée.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Msg {
    Click(u64),
    Text(u64, String),
    Value(u64, f64),
    Commit(u64, f64),
    Select(u64, u64),
    Toggle(u64, bool),
    Drop(u64, crate::ui::drag_drop::DropEvent),
    Menu(u64, crate::ui::popover::ContextMenuRequest),
}

struct ViewApp(Box<dyn Fn() -> Element>);
impl Application for ViewApp {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        (self.0)()
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}
pub(super) fn view_runtime(view: impl Fn() -> Element + 'static) -> UiRuntime {
    UiRuntime::new(ViewApp(Box::new(view)), &NoResources).unwrap()
}
pub(super) fn scroll_metrics(ui: &UiRuntime, id: u64) -> crate::ui::scroll::ScrollMetrics {
    ui.root()
        .unwrap()
        .find(ElementId(id))
        .unwrap()
        .scroll_metrics()
        .unwrap()
}
pub(super) fn wheel(ui: &mut UiRuntime, position: PointF, x: f32, y: f32) -> EventResult {
    ui.event(UiEvent::Scroll {
        position,
        delta_x: x,
        delta_y: y,
    })
}

#[derive(Default)]
pub(super) struct Measure {
    pub(super) calls: usize,
}
impl TextMeasurer for Measure {
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        _: bool,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.calls += 1;
        let natural = text.chars().count() as f32 * size * 0.5;
        Ok(SizeF {
            width: natural.min(max_width),
            height: (natural / max_width.max(1.)).ceil().max(1.) * size * 1.4,
        })
    }
}
pub(super) fn box_node(id: u64, width: Length, height: Length) -> Element {
    Element::column(id, vec![]).width(width).height(height)
}
pub(super) fn arrange(root: &mut Element, width: f32, height: f32) {
    layout::validate(root).unwrap();
    let bounds = RectF::new(0., 0., width, height);
    layout::arrange(root, bounds, bounds, &mut Measure::default()).unwrap();
}
pub(super) fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
}

pub(super) struct NoResources;
impl Resources for NoResources {
    fn decode_image(&self, _: &[u8]) -> crate::Result<Arc<Image>> {
        Err(crate::Error::Unsupported {
            reason: "no image decoder in this test",
        })
    }
}
pub(super) struct Counter {
    pub(super) value: Rc<Cell<u32>>,
}
impl Application for Counter {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        Element::column(
            1,
            vec![
                widgets::text(2, format!("Count: {}", self.value.get())),
                widgets::button(10, "Ajouter", Msg::Click(1)),
                Element::custom(11, Button::new("Inactif", Msg::Click(2)).disabled()),
                widgets::button(12, "Autre", Msg::Click(1)),
            ],
        )
        .gap(8.)
        .padding(10.)
    }
    fn update(&mut self, _: Msg) -> bool {
        self.value.set(self.value.get() + 1);
        true
    }
}
pub(super) fn context() -> ViewContext {
    ViewContext {
        width: 600.,
        height: 400.,
        theme: Theme::DARK,
        transparent: false,
        backend: "test",
        scale: 1.,
    }
}
pub(super) fn prepare(runtime: &mut UiRuntime, measure: &mut Measure) {
    runtime
        .prepare(context(), ChromeState::default(), measure)
        .unwrap();
}
pub(super) fn center(runtime: &UiRuntime, id: u64) -> PointF {
    let r = runtime
        .root()
        .unwrap()
        .find(ElementId(id))
        .unwrap()
        .bounds();
    PointF {
        x: (r.left + r.right) / 2.,
        y: (r.top + r.bottom) / 2.,
    }
}
pub(super) fn key(runtime: &mut UiRuntime, key: Key) {
    runtime.event(UiEvent::KeyDown {
        key,
        shift: false,
        repeat: false,
    });
}
