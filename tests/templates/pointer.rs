//! Molette (`onwheel`) et parties exclues d'une zone de déplacement de fenêtre
//! (`data-no-drag`) écrites dans un gabarit, comme le micro-lecteur de RustMusic.
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::prelude::*,
    ui::{runtime::WheelEvent, scene::PointF},
};

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Volume(i32),
}

#[derive(Default)]
struct Micro {
    volume: i32,
    locked: bool,
}

impl Application for Micro {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let locked = self.locked;
        let step = |e: WheelEvent| if e.delta_y < 0. { 5 } else { -5 };
        ui! {
            <div id="cover" style="height:200px" data-drag-region
                 onwheel={move |e: WheelEvent| (!locked).then(|| Msg::Volume(step(e)))}>
                <div id="top" style="height:100px"></div>
                <div id="bottom" style="height:40px; flex-direction:row" data-no-drag>
                    <div id="timeline" style="width:100px; height:40px"></div>
                    <div id="grip" style="width:40px; height:40px" data-drag-region></div>
                </div>
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        let Msg::Volume(step) = message;
        self.volume += step;
        true
    }
}

fn center(app: &HeadlessApp<Micro>, id: &str) -> PointF {
    let r = app.bounds(id).unwrap();
    PointF {
        x: (r.left + r.right) / 2.,
        y: (r.top + r.bottom) / 2.,
    }
}

#[test]
fn data_no_drag_excludes_a_row_from_the_drag_region() {
    let mut app = HeadlessApp::new(Micro::default()).unwrap();
    app.prepare().unwrap();
    let runtime = app.runtime();
    assert!(runtime.drag_region_at(center(&app, "top")));
    assert!(!runtime.drag_region_at(center(&app, "timeline")));
    assert!(runtime.drag_region_at(center(&app, "grip")));
}

#[test]
fn onwheel_turns_the_wheel_into_messages_unless_the_handler_declines() {
    let mut app = HeadlessApp::new(Micro::default()).unwrap();
    app.scroll("timeline", 0., -100.).unwrap();
    app.scroll("top", 0., 100.).unwrap();
    app.scroll("top", 0., -100.).unwrap();
    assert_eq!(app.app().volume, 5);
    let mut app = HeadlessApp::new(Micro {
        locked: true,
        ..Micro::default()
    })
    .unwrap();
    app.scroll("top", 0., -100.).unwrap();
    assert_eq!(app.app().volume, 0, "gestionnaire qui rend None : rien");
}
