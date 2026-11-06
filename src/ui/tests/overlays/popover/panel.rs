//! Panneaux ancrés : focus, fermeture (clic extérieur, Tab, molette, perte d'ancre),
//! placement, empilement sur une modale et validation.
use super::*;

#[test]
fn anchored_panel_accepts_controls_and_restores_focus_without_a_modal_veil() {
    let mut f = Fixture::new();
    f.open(false);
    assert_eq!(f.ui.focused(), Some(ElementId(102)));
    f.event(UiEvent::Text("Nom".into()));
    assert_eq!(f.m.borrow().text, "Nom");
    f.focus(104);
    f.key(Key::Right, false);
    f.event(UiEvent::KeyUp(Key::Right));
    assert_eq!(f.m.borrow().value, 51.);
    let nodes = f.ui.accessibility();
    assert_eq!(nodes[0].role, Role::Popover);
    assert!(!f.ui.scene().commands.iter().any(
        |c| matches!(c, crate::ui::scene::DrawCommand::Fill {color, ..} if color.alpha == 150)
    ));
    f.key(Key::Escape, false);
    assert!(f.m.borrow().popup.is_none());
    assert_eq!(f.ui.focused(), Some(ElementId(2)));
}

#[test]
fn outside_click_closes_the_popup_and_reaches_the_control_underneath() {
    let mut f = Fixture::new();
    f.open(true);
    let p = center(&f.ui, 3);
    // Comme une page web : le popover se ferme à l'appui, le bouton dessous reçoit le clic.
    assert!(f.event(UiEvent::PointerDown(p)).capture);
    assert!(f.m.borrow().popup.is_none());
    assert!(f.event(UiEvent::PointerUp(p)).release);
    assert!(f.m.borrow().actions.contains(&9));
    assert!(!f.ui.has_capture());
    // Sur son ancre, l'appui referme seulement le popover, sans le rouvrir.
    f.open(true);
    let anchor = center(&f.ui, 5);
    let opened = f.m.borrow().actions.iter().filter(|a| **a == 2).count();
    assert!(f.event(UiEvent::PointerDown(anchor)).capture);
    assert!(f.m.borrow().popup.is_some());
    assert!(f.event(UiEvent::PointerUp(anchor)).release);
    assert!(f.m.borrow().popup.is_none());
    assert_eq!(
        f.m.borrow().actions.iter().filter(|a| **a == 2).count(),
        opened
    );
}

#[test]
fn tab_closes_popup_and_advances_from_the_restored_owner() {
    let mut f = Fixture::new();
    f.open(false);
    assert!(f.key(Key::Tab, false).handled);
    assert!(f.m.borrow().popup.is_none());
    assert_eq!(f.ui.focused(), Some(ElementId(3)));
    f.open(true);
    f.key(Key::Tab, true);
    assert!(f.m.borrow().popup.is_none());
    assert_eq!(f.ui.focused(), Some(ElementId(3)));
}

#[test]
fn popup_point_placement_fits_edges_and_chooses_above_near_bottom() {
    let mut f = Fixture::new();
    f.m.borrow_mut().popup = Some((Anchor::Point(PointF { x: 599., y: 438. }), true));
    f.rebuild();
    let popup = f.ui.root().unwrap().find(ElementId(100)).unwrap().bounds();
    assert!(popup.left >= 8. && popup.right <= 592. && popup.top >= 48. && popup.bottom <= 432.);
    assert!(popup.top < 438.);
    f.ui.scene().validate().unwrap();
}

#[test]
fn losing_anchor_removes_popup_and_releases_capture_with_live_focus() {
    for mode in 0..3 {
        let mut f = Fixture::new();
        f.open(false);
        let p = center(&f.ui, 105);
        assert!(f.event(UiEvent::PointerDown(p)).capture);
        match mode {
            0 => f.m.borrow_mut().missing = true,
            1 => f.m.borrow_mut().hidden = true,
            _ => f.m.borrow_mut().disabled = true,
        }
        f.rebuild();
        assert!(f.m.borrow().popup.is_none());
        assert!(f.ui.root().unwrap().find(ElementId(100)).is_none());
        assert!(f.event(UiEvent::PointerUp(p)).release);
        assert!(!f.ui.has_capture());
        assert!(f.ui.focused().is_some());
    }
}

#[test]
fn popup_on_modal_restores_dialog_focus_and_background_cannot_open_a_menu() {
    let mut f = Fixture::new();
    f.m.borrow_mut().modal = true;
    f.rebuild();
    assert_eq!(f.ui.focused(), Some(ElementId(22)));
    let outside = center(&f.ui, 2);
    assert!(!f.event(UiEvent::ContextMenu(Some(outside))).handled);
    assert!(f.m.borrow().requests.is_empty());
    f.invoke(22);
    assert_eq!(f.ui.focused(), Some(ElementId(102)));
    f.key(Key::Escape, false);
    assert!(f.m.borrow().modal);
    assert_eq!(f.ui.focused(), Some(ElementId(22)));
    f.key(Key::Escape, false);
    assert!(!f.m.borrow().modal);
}

#[test]
fn wheel_outside_and_focus_loss_dismiss_without_scrolling_the_page() {
    let mut f = Fixture::new();
    f.open(false);
    let p = center(&f.ui, 3);
    assert!(
        f.event(UiEvent::Scroll {
            position: p,
            delta_x: 0.,
            delta_y: 200.
        })
        .handled
    );
    assert!(f.m.borrow().popup.is_none());
    near(scroll_metrics(&f.ui, 6).offset, 0.);
    f.open(true);
    f.event(UiEvent::FocusChanged(false));
    assert!(f.m.borrow().popup.is_none());
}

#[test]
fn popup_validation_rejects_bad_order_self_anchor_nonfinite_point_and_editable_menu() {
    let make = |anchor| {
        Element::popover(
            10,
            Popover::new("Test", anchor, Msg::Click(4)),
            widgets::text(11, "Texte"),
        )
    };
    assert!(crate::error::rejected(layout::validate(&make(
        Anchor::Point(PointF { x: 0., y: 0. })
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        widgets::text(2, "Page"),
        vec![make(Anchor::Element(ElementId(11)))]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        widgets::text(2, "Page"),
        vec![make(Anchor::Point(PointF { x: f32::NAN, y: 0. }))]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        widgets::text(2, "Page"),
        vec![
            make(Anchor::Element(ElementId(2))),
            Element::modal(20, Modal::new("M"), widgets::text(21, "T"))
        ]
    ))));
    assert!(crate::error::rejected(layout::validate(&Element::layers(
        1,
        widgets::text(2, "Page"),
        vec![Element::popover(
            10,
            Popover::new("Menu", Anchor::Element(ElementId(2)), Msg::Click(4)).menu(),
            Element::custom(11, TextInput::new("", |text| Msg::Text(8, text)))
        )]
    ))));
}
