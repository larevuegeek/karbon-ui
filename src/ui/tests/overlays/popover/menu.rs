//! Menus contextuels : requêtes souris et clavier, navigation par flèches, activation,
//! longs menus et lignes virtualisées.
use super::*;

#[test]
fn mouse_and_keyboard_context_requests_choose_closest_binding_and_local_coordinates() {
    let mut f = Fixture::new();
    let p = center(&f.ui, 2);
    assert!(f.event(UiEvent::ContextMenu(Some(p))).handled);
    let request = f.m.borrow().requests[0];
    assert_eq!(request.target, ElementId(2));
    assert!(!request.keyboard);
    near(request.position.x, p.x);
    near(request.position.y, p.y);
    assert_eq!(f.m.borrow().actions[0], 3);
    f.key(Key::Escape, false);
    f.focus(5);
    f.event(UiEvent::ContextMenu(None));
    let request = f.m.borrow().requests[1];
    assert_eq!(request.target, ElementId(5));
    assert!(request.keyboard);
    assert!(request.position.y > p.y);
    f.key(Key::Escape, false);
    let p = center(&f.ui, 3);
    f.event(UiEvent::ContextMenu(Some(p)));
    assert_eq!(f.m.borrow().requests[2].target, ElementId(1));
}

#[test]
fn menu_arrows_wrap_skip_disabled_and_activation_closes_once() {
    let mut f = Fixture::new();
    f.open(true);
    assert_eq!(f.ui.focused(), Some(ElementId(111)));
    f.key(Key::Down, false);
    assert_eq!(f.ui.focused(), Some(ElementId(114)));
    f.key(Key::Down, false);
    assert_eq!(f.ui.focused(), Some(ElementId(111)));
    f.key(Key::Up, false);
    assert_eq!(f.ui.focused(), Some(ElementId(114)));
    f.key(Key::Home, false);
    f.key(Key::Enter, false);
    assert!(f.m.borrow().popup.is_none());
    assert_eq!(f.ui.focused(), Some(ElementId(5)));
    assert_eq!(f.m.borrow().actions, [2, 11, 4]);
}

#[test]
fn context_menu_actions_can_open_a_dialog_without_leaving_a_popup_above_it() {
    let mut f = Fixture::new();
    f.open(true);
    f.key(Key::End, false);
    f.key(Key::Enter, false);
    assert!(f.m.borrow().modal);
    assert!(f.m.borrow().popup.is_none());
    assert_eq!(f.ui.focused(), Some(ElementId(22)));
    assert_eq!(f.ui.accessibility()[0].role, Role::Dialog);
}

#[test]
fn stationary_hover_keeps_keyboard_choice_and_does_not_rebuild_layout() {
    let mut f = Fixture::new();
    f.open(true);
    let p = center(&f.ui, 111);
    f.event(UiEvent::PointerMove(p));
    f.key(Key::End, false);
    let stats = f.ui.stats();
    f.event(UiEvent::PointerMove(p));
    assert_eq!(f.ui.focused(), Some(ElementId(114)));
    assert_eq!(f.ui.stats().views, stats.views);
    assert_eq!(f.ui.stats().layouts, stats.layouts);
    f.event(UiEvent::PointerMove(PointF { x: p.x + 1., ..p }));
    assert_eq!(f.ui.focused(), Some(ElementId(111)));
}

#[test]
fn long_menu_reveals_last_action_and_exposes_invoke_on_disabled_items() {
    let mut f = Fixture::new();
    f.m.borrow_mut().long = true;
    f.open(true);
    f.key(Key::End, false);
    assert_eq!(f.ui.focused(), Some(ElementId(229)));
    assert!(scroll_metrics(&f.ui, 120).offset > 0.);
    assert!(
        !f.ui
            .accessibility()
            .iter()
            .find(|e| e.id == ElementId(229))
            .unwrap()
            .offscreen
    );
    f.key(Key::Escape, false);
    f.m.borrow_mut().long = false;
    f.open(true);
    let nodes = f.ui.accessibility();
    assert_eq!(nodes[0].role, Role::Menu);
    let disabled = nodes.iter().find(|e| e.id == ElementId(112)).unwrap();
    assert!(disabled.invokable && !disabled.enabled);
    assert_eq!(
        f.ui.accessibility_action(AccessibilityAction::Invoke(ElementId(112))),
        Err(AccessibilityError::Disabled)
    );
    assert_eq!(
        f.ui.accessibility_action(AccessibilityAction::Invoke(ElementId(3))),
        Err(AccessibilityError::Unavailable)
    );
}

#[test]
fn keyboard_context_menu_targets_active_virtual_row_instead_of_list_viewport() {
    struct VirtualApp(Rc<RefCell<Vec<ContextMenuRequest>>>);
    impl Application for VirtualApp {
        type Message = Msg;
        fn view(&self, _: &ViewContext) -> Element {
            Element::virtual_list(
                1,
                crate::ui::virtual_list::VirtualList::new(50, 40., |i| {
                    widgets::button(100 + i as u64, format!("Titre {i}"), Msg::Click(1))
                        .context_menu(|request| Msg::Menu(2, request))
                }),
            )
        }
        fn update(&mut self, message: Msg) -> bool {
            if let Msg::Menu(2, r) = message {
                self.0.borrow_mut().push(r);
            }
            false
        }
    }
    let requests = Rc::new(RefCell::new(vec![]));
    let mut ui = UiRuntime::new(VirtualApp(requests.clone()), &NoResources).unwrap();
    let mut measure = Measure::default();
    prepare(&mut ui, &mut measure);
    ui.accessibility_action(AccessibilityAction::Focus(ElementId(1)))
        .unwrap();
    ui.event(UiEvent::KeyDown {
        key: Key::End,
        shift: false,
        repeat: false,
    });
    prepare(&mut ui, &mut measure);
    assert!(ui.event(UiEvent::ContextMenu(None)).handled);
    let r = requests.borrow()[0];
    assert_eq!(r.target, ElementId(149));
    assert!(r.keyboard);
    let row = ui.root().unwrap().find(ElementId(149)).unwrap();
    near(r.position.y, row.clip.bottom);
}
