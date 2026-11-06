//! Navigation : défilement sous la barre de titre, accordéon, case et interrupteur (et
//! leurs libellés).
use super::{Demo, center, demo, scroll_to, with_class};
use karbon_ui::platform::headless::HeadlessApp;

#[test]
fn the_page_scrolls_under_the_title_bar() {
    let mut app = demo();
    let title = app.root().unwrap().children[0].bounds();
    let accordion = center(with_class(app.root().unwrap(), "accordion")[0]);
    assert!(accordion.y > 900., "the accordion starts below the window");
    scroll_to(&mut app, "accordion-button", 1);
    // La barre de titre reste en place pendant le défilement.
    assert_eq!(app.root().unwrap().children[0].bounds(), title);
}

#[test]
fn accordion_buttons_toggle_their_panes() {
    let mut app = demo();
    let shown = |app: &HeadlessApp<Demo>| {
        with_class(app.root().unwrap(), "accordion-collapse")
            .iter()
            .map(|e| e.classes().contains(&"show"))
            .collect::<Vec<_>>()
    };
    assert_eq!(shown(&app), [true, false]);
    let second = scroll_to(&mut app, "accordion-button", 1);
    app.click_at(second).unwrap();
    assert_eq!(shown(&app), [false, true]);
    let first = scroll_to(&mut app, "accordion-button", 0);
    app.click_at(first).unwrap();
    assert_eq!(shown(&app), [true, false]);
}

#[test]
fn checkbox_and_switch_toggle() {
    let mut app = demo();
    let boxes = scroll_to(&mut app, "form-check-input", 0);
    app.click_at(boxes).unwrap();
    assert!(!app.app().checked);
    let switch = scroll_to(&mut app, "form-check-input", 1);
    app.click_at(switch).unwrap();
    assert!(app.app().switched);
    // Le libellé `for` bascule aussi, comme dans un navigateur.
    let label = scroll_to(&mut app, "form-check-label", 0);
    app.click_at(label).unwrap();
    assert!(app.app().checked, "clicking the label toggles the checkbox");
    let label = scroll_to(&mut app, "form-check-label", 1);
    app.click_at(label).unwrap();
    assert!(!app.app().switched, "clicking the label toggles the switch");
}
