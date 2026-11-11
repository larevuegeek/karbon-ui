//! Pont AT-SPI sans bus : chemins des objets, états, rôles, actions et frontières du
//! texte ; avec le bus de la session s'il existe, l'inscription auprès du registre. Le
//! dialogue avec un vrai client (libatspi) se vérifie à la main, voir `docs/tests.md`.
use super::{
    text,
    tree::{self, Object, state},
};
use crate::{
    ui::{
        element::{CheckState, ChoiceState, ElementId, Role},
        runtime::{AccessibilityAction, AccessibleElement},
        scene::RectF,
        text_input::{AccessibleLine, AccessibleText},
    },
    window::WindowId,
};
use std::sync::Arc;

fn element(id: u64, role: Role) -> AccessibleElement {
    AccessibleElement {
        id: ElementId(id),
        parent: None,
        role,
        name: format!("élément {id}"),
        enabled: true,
        focused: false,
        bounds: RectF::new(10., 20., 30., 40.),
        clipped_bounds: RectF::new(10., 20., 30., 40.),
        focusable: true,
        offscreen: false,
        invokable: true,
        value: None,
        choice: None,
        selection_container: None,
        text: None,
        range: None,
        select: None,
        read_only: false,
        help: String::new(),
        expanded: None,
        current: None,
    }
}
#[test]
fn object_paths_round_trip_and_reject_foreign_paths() {
    for object in [
        Object::Root,
        Object::Window(WindowId(3)),
        Object::Element(WindowId(3), ElementId(u64::MAX)),
    ] {
        assert_eq!(Object::parse(&object.path()), Some(object));
    }
    assert_eq!(Object::parse(tree::NULL), None);
    assert_eq!(Object::parse("/org/a11y/atspi/accessible/x1"), None);
}
#[test]
fn states_roles_and_actions_follow_the_element() {
    let mut check = element(1, Role::Checkbox);
    check.choice = Some(ChoiceState::Toggle(CheckState::On));
    check.focused = true;
    let [low, high] = tree::element_states(&check);
    let has = |bit: u32| {
        if bit < 32 {
            low & 1 << bit != 0
        } else {
            high & 1 << (bit - 32) != 0
        }
    };
    assert!(has(state::CHECKED) && has(state::FOCUSED) && has(state::CHECKABLE));
    assert!(has(state::ENABLED) && has(state::SHOWING) && !has(state::EDITABLE));
    assert_eq!(tree::role(&check), (7, "check box"));
    assert!(matches!(
        tree::press(&check),
        Some(("toggle", AccessibilityAction::Toggle(_)))
    ));
    check.enabled = false;
    assert!(
        tree::press(&check).is_none(),
        "un contrôle désactivé n'a pas d'action"
    );
    let mut main = element(2, Role::Main);
    main.offscreen = true;
    let [low, _] = tree::element_states(&main);
    assert_eq!(low & 1 << state::SHOWING, 0, "élément défilé hors de vue");
    assert!(tree::attributes(&main).contains(&("xml-roles", "main")));
}
#[test]
fn aria_states_give_toggle_buttons_tabs_and_current_items() {
    let has = |e: &AccessibleElement, bit: u32| {
        let set = tree::element_states(e);
        set[(bit / 32) as usize] & 1 << (bit % 32) != 0
    };
    // `aria-pressed` : bouton bascule enfoncé.
    let mut pin = element(1, Role::Button);
    pin.invokable = false;
    pin.choice = Some(ChoiceState::Toggle(CheckState::On));
    assert_eq!(tree::role(&pin), (62, "toggle button"));
    assert!(has(&pin, state::PRESSED) && has(&pin, state::CHECKED));
    assert!(matches!(
        tree::press(&pin),
        Some(("toggle", AccessibilityAction::Toggle(_)))
    ));
    // `role="tab"` et `aria-selected` : onglet sélectionné, pas coché.
    let mut tab = element(2, Role::Tab);
    tab.invokable = false;
    tab.choice = Some(ChoiceState::Radio(true));
    assert_eq!(tree::role(&tab), (37, "page tab"));
    assert_eq!(
        tree::role(&element(3, Role::TabList)),
        (38, "page tab list")
    );
    assert!(has(&tab, state::SELECTABLE) && has(&tab, state::SELECTED));
    assert!(!has(&tab, state::CHECKED));
    assert!(matches!(
        tree::press(&tab),
        Some(("click", AccessibilityAction::SelectChoice(_)))
    ));
    // `aria-current="page"` : état actif et attribut `current`.
    let mut link = element(4, Role::Button);
    link.current = Some(crate::ui::element::CurrentItem::Page);
    assert!(has(&link, state::ACTIVE));
    assert!(tree::attributes(&link).contains(&("current", "page")));
}
#[test]
fn text_offsets_are_characters_and_boundaries_follow_words_and_lines() {
    let value: Arc<str> = Arc::from("été\nsuivant mot");
    let mut input = element(3, Role::TextInput);
    input.text = Some(AccessibleText {
        value: Arc::clone(&value),
        selection: 3..3,
        carets: Vec::new(),
        clusters: Vec::new(),
        clip: RectF::new(0., 0., 100., 40.),
        multiline: true,
        lines: vec![
            AccessibleLine {
                range: 0..5,
                rect: RectF::new(0., 0., 100., 20.),
            },
            AccessibleLine {
                range: 6..17,
                rect: RectF::new(0., 20., 100., 20.),
            },
        ],
    });
    // « é » vaut deux octets : le caret à l'octet 3 est au caractère 2.
    match text::property(&input, "CaretOffset") {
        Some(super::properties::Prop::Int(offset)) => assert_eq!(offset, 2),
        _ => panic!("CaretOffset"),
    }
    match text::property(&input, "CharacterCount") {
        Some(super::properties::Prop::Int(count)) => {
            assert_eq!(count, value.chars().count() as i32)
        }
        _ => panic!("CharacterCount"),
    }
    let t = input.text.as_ref().unwrap();
    let at = |byte: usize, unit: u32, granularity: bool| {
        let (a, b) = text::piece(t, byte, unit, granularity);
        &value[a..b]
    };
    // Mot (et blancs qui le suivent), ligne visuelle avec son retour, caractère.
    assert_eq!(at(9, 1, false), "suivant ");
    assert_eq!(at(2, 5, false), "été\n");
    assert_eq!(at(14, 5, false), "suivant mot");
    assert_eq!(at(0, 0, false), "é");
    assert_eq!(tree::role(&input), (61, "text"), "champ multiligne");
    assert!(tree::interfaces(&input).contains(&"org.a11y.atspi.EditableText"));
}

/// Fenêtre jamais créée : le pont est pompé sans fenêtre.
struct NoWindow;
impl super::bridge::Host for NoWindow {
    fn id(&self) -> crate::window::WindowId {
        unreachable!()
    }
    fn title(&self) -> &str {
        unreachable!()
    }
    fn size(&self) -> (i32, i32) {
        unreachable!()
    }
    fn runtime(&self) -> &crate::ui::runtime::UiRuntime {
        unreachable!()
    }
    fn active(&self) -> bool {
        unreachable!()
    }
    fn act(&mut self, _: crate::ui::runtime::AccessibilityAction) -> crate::Result<bool> {
        unreachable!()
    }
}
/// La connexion au registre (sauté sans bus d'accessibilité) n'attend pas la liste des
/// écoutes déjà déclarées : elle arrive ensuite, par `pump`.
#[test]
fn the_registered_listeners_arrive_after_connecting() {
    let Some(mut bridge) = super::Bridge::connect() else {
        eprintln!("karbon-ui: no accessibility bus, test skipped");
        return;
    };
    assert!(
        bridge.registered.is_some(),
        "listeners requested without waiting"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while bridge.registered.is_some() && std::time::Instant::now() < deadline {
        bridge.bus.wait(50);
        bridge.pump::<NoWindow>(&mut []);
    }
    assert!(bridge.registered.is_none(), "registry answered");
}
