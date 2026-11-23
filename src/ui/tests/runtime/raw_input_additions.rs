//! Ajouts de 0.3.23 aux entrées : rang du clic (double-clic), molette avec Maj, Ctrl et
//! Alt (`on_pointer_wheel`), fermeture demandée à l'application, champ `autofocus`,
//! ligne d'une liste virtuelle rendue visible (`VirtualList::reveal`).
use crate::{
    platform::headless::HeadlessApp,
    ui::{
        runtime::raw_input::{PointerInput, RawPointerState},
        tests::support::*,
        virtual_list::VirtualList,
        widgets::TextInput,
    },
};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

fn input(phase: PointerPhase, button: Option<PointerButton>, at: PointF) -> PointerInput {
    PointerInput {
        phase,
        button,
        position: at,
        shift: false,
        control: false,
        alt: false,
        meta: false,
        motion: None,
    }
}

fn x(value: f32) -> PointF {
    PointF { x: value, y: 10. }
}

#[test]
fn presses_close_in_time_and_place_count_as_a_double_then_a_triple_click() {
    let mut state = RawPointerState::default();
    let left = Some(PointerButton::Primary);
    let t0 = Instant::now();
    let at = |ms| t0 + Duration::from_millis(ms);
    let mut count =
        |phase, button, place, ms| state.count_clicks(&input(phase, button, x(place)), at(ms));
    assert_eq!(count(PointerPhase::Down, left, 10., 0), 1);
    assert_eq!(count(PointerPhase::Up, left, 10., 50), 1);
    assert_eq!(count(PointerPhase::Move, None, 11., 60), 0);
    assert_eq!(count(PointerPhase::Down, left, 12., 200), 2);
    assert_eq!(count(PointerPhase::Up, left, 12., 250), 2);
    assert_eq!(count(PointerPhase::Down, left, 12., 400), 3);
    // Trop tard, trop loin, ou un autre bouton : la série recommence.
    assert_eq!(count(PointerPhase::Down, left, 12., 1000), 1);
    assert_eq!(count(PointerPhase::Down, left, 30., 1100), 1);
    let right = Some(PointerButton::Secondary);
    assert_eq!(count(PointerPhase::Down, right, 30., 1200), 1);
}

type Zooms = Rc<RefCell<Vec<(f32, bool, bool)>>>;

/// Une colonne qui défile, dont le bloc 2 prend Ctrl+molette.
struct Timeline {
    zooms: Zooms,
}
impl Application for Timeline {
    type Message = (f32, bool, bool);
    fn view(&self, _: &ViewContext) -> Element {
        let zoomable = box_node(2, Length::Px(100.), Length::Px(100.))
            .on_pointer_wheel(|e: PointerEvent| {
                (e.phase == PointerPhase::Wheel && e.control).then_some((e.wheel.1, e.shift, e.alt))
            })
            // `on_pointer` ne voit jamais la molette.
            .on_pointer(|e: PointerEvent| {
                assert_ne!(e.phase, PointerPhase::Wheel);
                None::<(f32, bool, bool)>
            });
        let filler = box_node(4, Length::Px(100.), Length::Px(900.));
        let content = Element::column(3, vec![zoomable, filler]);
        Element::scroll(1, Axis::Vertical, content).height(Length::Px(300.))
    }
    fn update(&mut self, message: Self::Message) -> bool {
        self.zooms.borrow_mut().push(message);
        false
    }
}

#[test]
fn the_wheel_with_ctrl_goes_to_on_pointer_wheel_and_does_not_scroll() {
    let zooms: Zooms = Rc::new(RefCell::new(Vec::new()));
    let mut ui = UiRuntime::new(
        Timeline {
            zooms: zooms.clone(),
        },
        &NoResources,
    )
    .unwrap();
    prepare(&mut ui, &mut Measure::default());
    let at = center(&ui, 2);
    let with_control = PointerInput {
        control: true,
        ..input(PointerPhase::Wheel, None, at)
    };
    assert!(ui.pointer_wheel(with_control, (0., 1.)).handled);
    assert_eq!(*zooms.borrow(), [(1., false, false)]);
    // Sans Ctrl, le gestionnaire laisse passer : rien n'est pris, et la zone défile comme
    // avant (par l'événement ordinaire).
    let plain = input(PointerPhase::Wheel, None, at);
    assert!(!ui.pointer_wheel(plain, (0., 1.)).handled);
    wheel(&mut ui, at, 0., 120.);
    prepare(&mut ui, &mut Measure::default());
    assert!(scroll_metrics(&ui, 1).offset > 0.);
    assert_eq!(zooms.borrow().len(), 1);
}

/// Une application qui refuse la fermeture tant qu'elle a des changements.
struct Unsaved {
    unsaved: bool,
    asked: u32,
}
impl Application for Unsaved {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let label = if self.asked > 0 { "Enregistrer ?" } else { "" };
        widgets::text(1, label)
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
    fn close_requested(&mut self) -> bool {
        self.asked += 1;
        !self.unsaved
    }
}

#[test]
fn the_application_can_refuse_to_close_and_its_view_is_rebuilt() {
    let unsaved = Unsaved {
        unsaved: true,
        asked: 0,
    };
    let mut app = HeadlessApp::new(unsaved).unwrap();
    assert!(!app.request_close().unwrap());
    assert!(app.contains_text("Enregistrer ?"));
    app.update_app(|a| a.unsaved = false).unwrap();
    assert!(app.request_close().unwrap());
    assert_eq!(app.app().asked, 2);
    // Une application qui ne l'écrit pas ferme comme avant.
    let counter = Counter {
        value: Rc::new(Cell::new(0)),
    };
    assert!(HeadlessApp::new(counter).unwrap().request_close().unwrap());
}

/// Un champ qui n'apparaît qu'après un clic, `autofocus`, à côté d'un champ ordinaire.
struct Rename {
    editing: bool,
    name: String,
}
impl Application for Rename {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let mut children = vec![
            Element::custom(2, TextInput::new("autre", |t: String| Msg::Text(2, t))),
            widgets::button(3, "Renommer", Msg::Click(3)),
        ];
        if self.editing {
            let field = TextInput::new(self.name.as_str(), |t: String| Msg::Text(4, t)).autofocus();
            children.push(Element::custom(4, field));
        }
        Element::column(1, children)
    }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Click(3) => self.editing = true,
            Msg::Text(4, name) => self.name = name,
            _ => {}
        }
        true
    }
}

#[test]
fn an_autofocus_field_takes_the_keyboard_when_it_appears_with_its_text_selected() {
    let mut app = HeadlessApp::new(Rename {
        editing: false,
        name: "Caméra".into(),
    })
    .unwrap();
    app.click(3).unwrap();
    assert_eq!(app.focused(), Some(ElementId(4)));
    // Tout le texte est choisi : la frappe le remplace.
    app.type_text("Lampe").unwrap();
    assert_eq!(app.input_value(4).as_deref(), Some("Lampe"));
    // Déjà là, il ne reprend pas le clavier aux vues suivantes.
    app.click(2).unwrap();
    assert_eq!(app.focused(), Some(ElementId(2)));
    app.click(3).unwrap();
    assert_eq!(app.focused(), Some(ElementId(3)));
}

/// Une liste de 1 000 lignes qui montre la ligne `reveal` quand elle est donnée.
struct Revealed {
    reveal: Option<usize>,
}
impl Application for Revealed {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let mut list = VirtualList::new(1000, 20., |i| widgets::text(100 + i as u64, "ligne"));
        if let Some(index) = self.reveal {
            list = list.reveal(index);
        }
        Element::column(
            2,
            vec![Element::virtual_list(1, list).height(Length::Px(200.))],
        )
    }
    fn update(&mut self, _: Msg) -> bool {
        false
    }
}

#[test]
fn reveal_scrolls_just_enough_for_the_row() {
    let mut app = HeadlessApp::new(Revealed { reveal: Some(500) }).unwrap();
    let offset =
        |app: &HeadlessApp<Revealed>| app.find(1).unwrap().scroll_metrics().unwrap().offset;
    // La ligne 500 finit au bas de la vue : 501 × 20 − 200.
    assert_eq!(offset(&app), 501. * 20. - 200.);
    // Plus haut : la ligne se pose en haut.
    app.update_app(|a| a.reveal = Some(3)).unwrap();
    assert_eq!(offset(&app), 60.);
    // Déjà visible : rien ne bouge.
    app.update_app(|a| a.reveal = Some(5)).unwrap();
    assert_eq!(offset(&app), 60.);
    // Sans demande, l'offset reste celui de la vue d'avant.
    app.update_app(|a| a.reveal = None).unwrap();
    assert_eq!(offset(&app), 60.);
}
