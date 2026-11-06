//! Acheminement des messages typés : chaque famille de widgets, arbres reconstruits,
//! lignes virtualisées construites à la demande et message d'un type inattendu.
use crate::ui::tests::support::*;
use crate::ui::{
    choices::{Checkbox, RadioButton, Switch, radio_group},
    drag_drop::{DragSource, DropEvent, DropPosition, DropTarget},
    modal::Modal,
    popover::{Anchor, ContextMenuRequest, MenuItem, Popover},
    select::{Select, SelectOption},
    slider::Slider,
    template::Node,
    text_input::TextInput,
    virtual_list::VirtualList,
};
use std::cell::RefCell;

fn tall(ui: &mut UiRuntime, m: &mut Measure) {
    ui.prepare(
        ViewContext {
            height: 900.,
            ..context()
        },
        ChromeState::default(),
        m,
    )
    .unwrap();
}
fn invoke(ui: &mut UiRuntime, action: AccessibilityAction) {
    ui.accessibility_action(action).unwrap();
}
fn click(ui: &mut UiRuntime, point: PointF) {
    ui.event(UiEvent::PointerDown(point));
    ui.event(UiEvent::PointerUp(point));
}

#[derive(Debug, Clone, PartialEq)]
enum Gallery {
    Save,
    Area,
    Name(String),
    Submit,
    Notifications(bool),
    Dark(bool),
    Size(u8),
    Volume(f64),
    VolumeCommitted(f64),
    Theme(u64),
    Reorder(DropEvent),
    Menu(ContextMenuRequest),
    Copy,
    CloseMenu,
    CloseDialog,
}
#[derive(Default)]
struct State {
    log: Vec<Gallery>,
    menu: bool,
    dialog: bool,
    volume: f64,
}
struct GalleryApp(Rc<RefCell<State>>);
impl Application for GalleryApp {
    type Message = Gallery;
    fn view(&self, _: &ViewContext) -> Element {
        let s = self.0.borrow();
        let page = Element::column(
            1,
            vec![
                widgets::button(2, "Enregistrer", Gallery::Save),
                Node::element(Element::column(3, vec![]).height(Length::Px(30.)))
                    .on_click(Gallery::Area)
                    .build(),
                Element::custom(
                    4,
                    TextInput::new("", Gallery::Name).on_submit(Gallery::Submit),
                ),
                Element::custom(5, Checkbox::new("Alertes", false, Gallery::Notifications)),
                Element::custom(6, Switch::new("Sombre", true, Gallery::Dark)),
                radio_group(
                    7,
                    "Taille",
                    vec![
                        Element::custom(8, RadioButton::new("Petite", true, Gallery::Size(1))),
                        Element::custom(9, RadioButton::new("Grande", false, Gallery::Size(2))),
                    ],
                ),
                Element::custom(
                    10,
                    Slider::new(s.volume, Gallery::Volume).on_commit(Gallery::VolumeCommitted),
                ),
                Element::custom(
                    11,
                    Select::new(
                        Some(7),
                        vec![SelectOption::new(7, "Sept"), SelectOption::new(9, "Neuf")],
                        Gallery::Theme,
                    ),
                ),
                widgets::button(12, "Source", Gallery::Save)
                    .height(Length::Px(40.))
                    .draggable(DragSource::new(1, 10)),
                widgets::text(13, "Cible")
                    .height(Length::Px(40.))
                    .drop_target(DropTarget::new(1, 20, Gallery::Reorder))
                    .context_menu(Gallery::Menu),
            ],
        )
        .gap(4.);
        let mut overlays = Vec::new();
        if s.menu {
            overlays.push(Element::popover(
                20,
                Popover::new("Menu", Anchor::Element(ElementId(2)), Gallery::CloseMenu).menu(),
                Element::column(
                    22,
                    vec![Element::custom(21, MenuItem::new("Copier", Gallery::Copy))],
                ),
            ));
        }
        if s.dialog {
            overlays.push(Element::modal(
                30,
                Modal::new("Dialogue").on_dismiss(Gallery::CloseDialog),
                widgets::button(31, "Fermer", Gallery::CloseDialog),
            ));
        }
        Element::layers(0, page, overlays)
    }
    fn update(&mut self, message: Gallery) -> bool {
        let mut s = self.0.borrow_mut();
        match &message {
            Gallery::Volume(v) => s.volume = *v,
            Gallery::CloseMenu => s.menu = false,
            Gallery::CloseDialog => s.dialog = false,
            _ => {}
        }
        s.log.push(message);
        true
    }
}

#[test]
fn every_widget_kind_routes_its_typed_message_to_update() {
    let state = Rc::new(RefCell::new(State {
        volume: 50.,
        ..Default::default()
    }));
    let mut ui = UiRuntime::new(GalleryApp(state.clone()), &NoResources).unwrap();
    let mut m = Measure::default();
    tall(&mut ui, &mut m);
    let take = |state: &Rc<RefCell<State>>| std::mem::take(&mut state.borrow_mut().log);

    invoke(&mut ui, AccessibilityAction::Invoke(ElementId(2)));
    let area = center(&ui, 3);
    click(&mut ui, area);
    assert_eq!(take(&state), [Gallery::Save, Gallery::Area]);
    tall(&mut ui, &mut m);

    invoke(&mut ui, AccessibilityAction::Focus(ElementId(4)));
    ui.event(UiEvent::Text("Ada".into()));
    key(&mut ui, Key::Enter);
    assert_eq!(take(&state), [Gallery::Name("Ada".into()), Gallery::Submit]);
    invoke(
        &mut ui,
        AccessibilityAction::SetValue {
            id: ElementId(4),
            value: "Grace".into(),
        },
    );
    assert_eq!(take(&state), [Gallery::Name("Grace".into())]);
    tall(&mut ui, &mut m);

    // Case et interrupteur reçoivent leur nouvelle valeur ; le radio choisi n'émet rien.
    invoke(&mut ui, AccessibilityAction::Toggle(ElementId(5)));
    invoke(&mut ui, AccessibilityAction::Toggle(ElementId(6)));
    invoke(&mut ui, AccessibilityAction::SelectChoice(ElementId(8)));
    invoke(&mut ui, AccessibilityAction::SelectChoice(ElementId(9)));
    assert_eq!(
        take(&state),
        [
            Gallery::Notifications(true),
            Gallery::Dark(false),
            Gallery::Size(2)
        ]
    );
    tall(&mut ui, &mut m);

    invoke(&mut ui, AccessibilityAction::Focus(ElementId(10)));
    key(&mut ui, Key::Right);
    tall(&mut ui, &mut m);
    ui.event(UiEvent::KeyUp(Key::Right));
    assert_eq!(
        take(&state),
        [Gallery::Volume(51.), Gallery::VolumeCommitted(51.)]
    );
    tall(&mut ui, &mut m);

    invoke(
        &mut ui,
        AccessibilityAction::ExpandSelect {
            id: ElementId(11),
            expanded: true,
        },
    );
    invoke(
        &mut ui,
        AccessibilityAction::ChooseOption {
            id: ElementId(11),
            option: 9,
        },
    );
    assert_eq!(take(&state), [Gallery::Theme(9)]);
    tall(&mut ui, &mut m);

    // Un dépôt valide n'active pas le bouton source.
    let source = center(&ui, 12);
    let target = center(&ui, 13);
    ui.event(UiEvent::PointerDown(source));
    ui.event(UiEvent::PointerMove(PointF {
        x: source.x,
        y: source.y + 10.,
    }));
    ui.event(UiEvent::PointerMove(target));
    ui.event(UiEvent::PointerUp(target));
    let drop = DropEvent {
        group: 1,
        source: 10,
        target: 20,
        position: DropPosition::After,
    };
    assert_eq!(take(&state), [Gallery::Reorder(drop)]);
    tall(&mut ui, &mut m);

    assert!(ui.event(UiEvent::ContextMenu(Some(target))).handled);
    assert_eq!(
        take(&state),
        [Gallery::Menu(ContextMenuRequest {
            target: ElementId(13),
            position: target,
            keyboard: false,
        })]
    );

    state.borrow_mut().menu = true;
    ui.invalidate();
    tall(&mut ui, &mut m);
    invoke(&mut ui, AccessibilityAction::Invoke(ElementId(21)));
    assert_eq!(take(&state), [Gallery::Copy, Gallery::CloseMenu]);
    tall(&mut ui, &mut m);

    state.borrow_mut().dialog = true;
    ui.invalidate();
    tall(&mut ui, &mut m);
    key(&mut ui, Key::Escape);
    assert_eq!(take(&state), [Gallery::CloseDialog]);
}

#[derive(Debug, Clone, PartialEq)]
enum Versioned {
    Click(u32),
    Value(u32, f64),
    Commit(u32, f64),
}
#[derive(Default)]
struct Versions {
    generation: u32,
    value: f64,
    swapped: bool,
    log: Vec<Versioned>,
}
struct VersionApp(Rc<RefCell<Versions>>);
impl Application for VersionApp {
    type Message = Versioned;
    fn view(&self, _: &ViewContext) -> Element {
        let s = self.0.borrow();
        let generation = s.generation;
        // Chaque arbre capture sa génération : un message révèle l'arbre qui l'a produit.
        let button = if s.swapped {
            widgets::button(3, "Autre", Versioned::Click(99))
        } else {
            widgets::button(2, "Version", Versioned::Click(generation))
        };
        Element::column(
            1,
            vec![
                button.height(Length::Px(40.)),
                Element::custom(
                    10,
                    Slider::new(s.value, move |v| Versioned::Value(generation, v))
                        .on_commit(move |v| Versioned::Commit(generation, v)),
                )
                .width(Length::Px(200.)),
            ],
        )
    }
    fn update(&mut self, message: Versioned) -> bool {
        let mut s = self.0.borrow_mut();
        if let Versioned::Value(_, v) = message {
            s.value = v;
        }
        s.log.push(message);
        s.generation += 1;
        true
    }
}

#[test]
fn rebuilt_trees_route_each_event_to_the_handlers_of_the_tree_it_targets() {
    let state = Rc::new(RefCell::new(Versions {
        value: 50.,
        ..Default::default()
    }));
    let mut ui = UiRuntime::new(VersionApp(state.clone()), &NoResources).unwrap();
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let take = |state: &Rc<RefCell<Versions>>| std::mem::take(&mut state.borrow_mut().log);

    // Tant que la vue n'est pas reconstruite, l'arbre affiché (génération 0) répond.
    invoke(&mut ui, AccessibilityAction::Invoke(ElementId(2)));
    invoke(&mut ui, AccessibilityAction::Invoke(ElementId(2)));
    assert_eq!(take(&state), [Versioned::Click(0), Versioned::Click(0)]);
    prepare(&mut ui, &mut m);
    invoke(&mut ui, AccessibilityAction::Invoke(ElementId(2)));
    assert_eq!(take(&state), [Versioned::Click(2)]);
    prepare(&mut ui, &mut m);

    // Un glissement survit aux reconstructions ; chaque valeur vient de l'arbre courant.
    let slider = ui.root().unwrap().find(ElementId(10)).unwrap().bounds();
    let y = (slider.top + slider.bottom) / 2.;
    let at = |ratio: f32| PointF {
        x: slider.left + (slider.right - slider.left) * ratio,
        y,
    };
    assert!(ui.event(UiEvent::PointerDown(at(0.2))).capture);
    let first = take(&state);
    assert!(
        matches!(first.as_slice(), [Versioned::Value(3, _)]),
        "{first:?}"
    );
    prepare(&mut ui, &mut m);
    ui.event(UiEvent::PointerMove(at(0.8)));
    let second = take(&state);
    assert!(
        matches!(second.as_slice(), [Versioned::Value(4, _)]),
        "{second:?}"
    );
    prepare(&mut ui, &mut m);
    assert!(ui.has_capture());
    assert!(ui.event(UiEvent::PointerUp(at(0.8))).release);
    let Versioned::Value(_, last) = second[0] else {
        unreachable!()
    };
    assert_eq!(take(&state), [Versioned::Commit(5, last)]);
    prepare(&mut ui, &mut m);

    // Un élément retiré pendant l'appui ne transmet son relâchement à aucun autre.
    let point = center(&ui, 2);
    ui.event(UiEvent::PointerDown(point));
    state.borrow_mut().swapped = true;
    ui.invalidate();
    prepare(&mut ui, &mut m);
    assert!(ui.root().unwrap().find(ElementId(3)).is_some());
    ui.event(UiEvent::PointerUp(point));
    assert!(take(&state).is_empty());
}

#[test]
fn lazily_built_virtual_rows_carry_their_own_handlers() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let builds = Rc::new(Cell::new(0));
    struct Rows {
        log: Rc<RefCell<Vec<Msg>>>,
        builds: Rc<Cell<usize>>,
    }
    impl Application for Rows {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            let builds = self.builds.clone();
            Element::virtual_list(
                1,
                VirtualList::new(10_000, 32., move |index| {
                    builds.set(builds.get() + 1);
                    let index = index as u64;
                    widgets::button(100 + index, format!("Ligne {index}"), Msg::Click(index))
                        .context_menu(move |request| Msg::Menu(index, request))
                }),
            )
            .height(Length::Px(200.))
        }
        fn update(&mut self, message: Msg) -> bool {
            self.log.borrow_mut().push(message);
            false
        }
    }
    let mut ui = UiRuntime::new(
        Rows {
            log: log.clone(),
            builds: builds.clone(),
        },
        &NoResources,
    )
    .unwrap();
    let mut m = Measure::default();
    prepare(&mut ui, &mut m);
    let initial = builds.get();
    let list = ui.root().unwrap().bounds();
    let inside = PointF {
        x: list.left + 20.,
        y: list.top + 20.,
    };
    wheel(&mut ui, inside, 0., 32. * 5_000.);
    prepare(&mut ui, &mut m);
    assert!(builds.get() > initial, "rows must be built after scrolling");
    let row = ui
        .root()
        .unwrap()
        .children
        .iter()
        .filter(|row| row.bounds().top >= list.top && row.bounds().bottom <= list.bottom)
        .map(|row| row.id.0 - 100)
        .find(|&index| index >= 4_000)
        .expect("a lazily built row is fully visible");
    let point = center(&ui, 100 + row);
    click(&mut ui, point);
    ui.event(UiEvent::ContextMenu(Some(point)));
    assert_eq!(
        *log.borrow(),
        [
            Msg::Click(row),
            Msg::Menu(
                row,
                ContextMenuRequest {
                    target: ElementId(100 + row),
                    position: point,
                    keyboard: false,
                }
            )
        ]
    );
}

#[test]
#[should_panic(expected = "an element emitted a message of type `u8`")]
fn a_message_of_another_type_is_a_clear_programming_error() {
    struct Mixed;
    impl Application for Mixed {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            // Erreur volontaire : ce bouton n'émet pas `Msg`.
            widgets::button(1, "Incompatible", 7_u8)
        }
        fn update(&mut self, _: Msg) -> bool {
            false
        }
    }
    let mut ui = UiRuntime::new(Mixed, &NoResources).unwrap();
    prepare(&mut ui, &mut Measure::default());
    let _ = ui.accessibility_action(AccessibilityAction::Invoke(ElementId(1)));
}
