//! Templates sous le runtime : actions des contrôles natifs (clic, saisie, case, curseur,
//! liste) par les commandes d'accessibilité, animation sans reconstruction, et arbre
//! d'accessibilité des boutons riches et de `aria-hidden`.
use crate::support::Measure;
use karbon_ui::{
    application::Resources,
    ui::{
        prelude::*,
        runtime::{AccessibilityAction, ChromeState, UiEvent, UiRuntime},
        scene::{DrawCommand, Image, PointF},
    },
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

struct Services;

impl Resources for Services {
    fn decode_image(&self, _: &[u8]) -> karbon_ui::Result<Arc<Image>> {
        unreachable!()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Click,
    Name(String),
    Checked(bool),
    Volume(f64),
    VolumeCommitted(f64),
    Choice(Option<u64>),
}

#[derive(Default)]
struct App {
    name: String,
    checked: bool,
    volume: f64,
    commits: Rc<RefCell<Vec<f64>>>,
    choice: Option<u64>,
    clicks: u32,
}

impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            <div style="gap:8px; padding:10px">
                <div id="action" style="height:50px" onclick={Msg::Click}
                        style:background={linear_gradient(0.,[Color::hex(0x20c060),Color::hex(0x063020)])}
                        style:animate={Animation::fade_in().duration(100).easing(Easing::Linear)}>
                    <span>Clics : {self.clicks}</span>
                </div>
                <input id="name" type="text" value={self.name.as_str()} oninput={Msg::Name} aria-label="Nom">
                <input id="check" type="checkbox" checked={self.checked} onchange={Msg::Checked}> Actif
                <input id="volume" type="range" value={self.volume} oninput={Msg::Volume}
                       onchange={Msg::VolumeCommitted} aria-label="Volume">
                <select id="choice" value={self.choice} onchange={|key| Msg::Choice(Some(key))}>
                    <option value="7">Sept</option>
                    <option value="9">Neuf</option>
                </select>
                {#if self.checked}<span id="message">{self.name.as_str()}</span>{/if}
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Click => self.clicks += 1,
            Msg::Name(name) => self.name = name,
            Msg::Checked(checked) => self.checked = checked,
            Msg::Volume(volume) => self.volume = volume,
            Msg::VolumeCommitted(volume) => {
                self.commits.borrow_mut().push(volume);
                return false;
            }
            Msg::Choice(choice) => self.choice = choice,
        }
        true
    }
}

fn prepare(ui: &mut UiRuntime, time: Instant) {
    ui.prepare_at(
        ViewContext {
            width: 600.,
            height: 500.,
            theme: Theme::DARK,
            transparent: false,
            backend: "test",
            scale: 1.,
        },
        ChromeState {
            visible: false,
            ..Default::default()
        },
        &mut Measure,
        time,
    )
    .unwrap();
}

#[test]
fn templates_route_native_control_actions_and_animate_without_rebuilding() {
    let commits = Rc::new(RefCell::new(Vec::new()));
    let app = App {
        commits: Rc::clone(&commits),
        ..App::default()
    };
    let mut ui = UiRuntime::new(app, &Services).unwrap();
    let start = Instant::now();
    prepare(&mut ui, start);
    assert!(
        ui.scene()
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Gradient { .. }))
    );
    let stats = ui.stats();
    prepare(&mut ui, start + Duration::from_millis(50));
    assert_eq!(ui.stats().views, stats.views);
    assert_eq!(ui.stats().layouts, stats.layouts);
    assert!(
        ui.scene()
            .commands
            .iter()
            .any(|c| matches!(c,DrawCommand::PushEffect{opacity,..} if (*opacity-0.5).abs()<0.01))
    );
    prepare(&mut ui, start + Duration::from_millis(100));
    let rect = ui
        .root()
        .unwrap()
        .find(ElementId::named("action"))
        .unwrap()
        .bounds();
    let point = PointF {
        x: rect.left + 5.,
        y: rect.top + 5.,
    };
    assert!(ui.event(UiEvent::PointerDown(point)).handled);
    assert!(ui.event(UiEvent::PointerUp(point)).handled);
    prepare(&mut ui, start + Duration::from_millis(110));
    assert!(ui.accessibility().iter().any(|n| n.name == "Clics : 1"));
    ui.accessibility_action(AccessibilityAction::SetValue {
        id: ElementId::named("name"),
        value: "David".into(),
    })
    .unwrap();
    prepare(&mut ui, start + Duration::from_millis(120));
    ui.accessibility_action(AccessibilityAction::Toggle(ElementId::named("check")))
        .unwrap();
    prepare(&mut ui, start + Duration::from_millis(130));
    assert!(
        ui.accessibility()
            .iter()
            .any(|n| n.id == ElementId::named("message") && n.name == "David")
    );
    ui.accessibility_action(AccessibilityAction::SetNumber {
        id: ElementId::named("volume"),
        value: 42.,
    })
    .unwrap();
    prepare(&mut ui, start + Duration::from_millis(140));
    let volume = ui.root().unwrap().find(ElementId::named("volume")).unwrap();
    assert_eq!(volume.widget.as_ref().unwrap().slider().unwrap().value, 42.);
    // Une commande accessible est une interaction complète : valeur puis validation.
    assert_eq!(*commits.borrow(), [42.]);
    ui.accessibility_action(AccessibilityAction::ExpandSelect {
        id: ElementId::named("choice"),
        expanded: true,
    })
    .unwrap();
    ui.accessibility_action(AccessibilityAction::ChooseOption {
        id: ElementId::named("choice"),
        option: 9,
    })
    .unwrap();
    prepare(&mut ui, start + Duration::from_millis(150));
    let choice = ui.root().unwrap().find(ElementId::named("choice")).unwrap();
    assert_eq!(
        choice.widget.as_ref().unwrap().select().unwrap().selected,
        Some(9)
    );
}

#[test]
fn rich_buttons_and_aria_hidden_shape_the_accessibility_tree() {
    use karbon_ui::ui::{element::Role, runtime::UiEvent};
    let mut ui = static_view(|| {
        ui! {<div>
            <button onclick={Msg::Click} aria-label="Lire"><div aria-hidden="true">*</div><p>Lecture</p></button>
            <p aria-hidden="true">Décor</p>
            <p aria-hidden={false}>Visible</p>
        </div>}
    });
    let now = std::time::Instant::now();
    prepare_headless(&mut ui, now);
    let tree = ui.accessibility();
    assert!(
        tree.iter()
            .any(|a| a.role == Role::Button && a.name == "Lire")
    );
    assert!(tree.iter().any(|a| a.name == "Lecture"));
    assert!(tree.iter().any(|a| a.name == "Visible"));
    assert!(!tree.iter().any(|a| a.name == "Décor" || a.name == "*"));
    // Le contenu masqué reste dessiné.
    assert!(ui.scene().commands.iter().any(|c| matches!(c,
        DrawCommand::FontText { text, .. } | DrawCommand::FormattedText { text, .. } if &**text == "Décor")));
    // Le bouton riche est focalisable et activable au clavier.
    ui.event(UiEvent::KeyDown {
        key: karbon_ui::ui::runtime::Key::Tab,
        shift: false,
        repeat: false,
    });
    prepare_headless(&mut ui, now);
    assert!(
        ui.accessibility()
            .iter()
            .any(|a| a.role == Role::Button && a.focused)
    );
}

/// Application dont la vue est une fonction sans état.
struct Static(fn() -> Element);
impl Application for Static {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        (self.0)()
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}
fn static_view(view: fn() -> Element) -> UiRuntime {
    UiRuntime::new(Static(view), &Services).unwrap()
}
/// 600 × 400 avec les métriques du backend headless.
fn prepare_headless(ui: &mut UiRuntime, time: Instant) {
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
        &mut karbon_ui::platform::headless::HeadlessTextMeasurer::new(),
        time,
    )
    .unwrap();
}
