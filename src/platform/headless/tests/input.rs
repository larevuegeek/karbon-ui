//! Entrées simulées : clics, saisie, presse-papiers, ordre du focus et défilement.
use super::{Msg, Notes};
use crate::{
    application::{Application, ViewContext},
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{
        element::{Axis, Element, ElementId, Role},
        prelude::{FILL, column, scroll, text},
        runtime::Key,
        text_input::EditCommand,
    },
};

#[test]
fn click_delivers_the_message_rebuilds_and_repaints_the_view() {
    let mut app = HeadlessApp::new(Notes::default()).unwrap();
    assert!(app.contains_text("Count: 0"));
    assert_eq!(app.runtime().stats().views, 1);
    app.click("add").unwrap();
    app.click("add").unwrap();
    assert_eq!(app.app().count, 2);
    assert!(app.contains_text("Count: 2"));
    assert!(!app.contains_text("Count: 0"));
    assert_eq!(
        app.accessible("count").map(|node| node.name),
        Some("Count: 2".to_owned())
    );
    assert_eq!(app.focused(), Some(ElementId::named("add")));
    app.click("reset").unwrap();
    assert!(app.contains_text("Count: 0"));
    // Messages directs et modification de l'état reconstruisent aussi la vue.
    assert!(app.send(Msg::Add).unwrap());
    assert!(app.contains_text("Count: 1"));
    app.update_app(|notes| notes.count = 41).unwrap();
    assert!(app.contains_text("Count: 41"));
    assert!(matches!(
        app.click("missing"),
        Err(crate::Error::InvalidInput { .. })
    ));
}

#[test]
fn typing_editing_and_clipboard_reach_the_application() {
    let mut app = HeadlessApp::new(Notes::default()).unwrap();
    app.click("name").unwrap();
    assert_eq!(app.focused(), Some(ElementId::named("name")));
    app.type_text("Hé ho").unwrap();
    assert_eq!(app.app().name, "Hé ho");
    assert_eq!(app.input_value("name").as_deref(), Some("Hé ho"));
    app.press(Key::Backspace).unwrap();
    assert_eq!(app.app().name, "Hé h");
    app.edit(EditCommand::SelectAll).unwrap();
    app.edit(EditCommand::Copy).unwrap();
    assert_eq!(app.clipboard(), Some("Hé h"));
    app.press(Key::End).unwrap();
    app.edit(EditCommand::Paste).unwrap();
    assert_eq!(app.app().name, "Hé hHé h");
    app.edit(EditCommand::SelectAll).unwrap();
    app.edit(EditCommand::Cut).unwrap();
    assert_eq!(app.app().name, "");
    app.set_clipboard("Ada");
    app.edit(EditCommand::Paste).unwrap();
    // Entrée dans le champ ne soumet rien ici : le bouton le fait.
    app.click("submit").unwrap();
    assert_eq!(app.app().submitted, ["Ada"]);
    assert_eq!(app.input_value("name").as_deref(), Some(""));
    let text = app.accessible("name").unwrap();
    assert_eq!(text.role, Role::TextInput);
}

#[test]
fn tab_and_shift_tab_follow_the_focus_order() {
    let mut app = HeadlessApp::new(Notes::default()).unwrap();
    assert_eq!(app.focused(), None);
    let order = ["add", "reset", "close", "name", "submit"];
    for id in order {
        app.press(Key::Tab).unwrap();
        assert_eq!(app.focused(), Some(ElementId::named(id)), "{id}");
    }
    app.press_with_shift(Key::Tab).unwrap();
    assert_eq!(app.focused(), Some(ElementId::named("name")));
    app.focus("reset").unwrap();
    assert_eq!(app.focused(), Some(ElementId::named("reset")));
    app.update_app(|notes| notes.count = 3).unwrap();
    app.press(Key::Space).unwrap();
    assert_eq!(app.app().count, 0);
    // Le focus reste sur le même élément après la reconstruction.
    assert_eq!(app.focused(), Some(ElementId::named("reset")));
    assert!(app.focus("count").is_err());
}

struct Feed;
impl Application for Feed {
    type Message = ();
    fn view(&self, _: &ViewContext) -> Element {
        scroll(
            "feed",
            Axis::Vertical,
            column(
                "rows",
                (0..50)
                    .map(|index| text(ElementId::named("row").item(index), format!("Row {index}"))),
            )
            .gap(4.),
        )
        .height(FILL)
    }
    fn update(&mut self, _: ()) -> bool {
        false
    }
}

#[test]
fn scrolling_moves_content_without_rebuilding_the_view() {
    let mut app = HeadlessApp::with_options(
        Feed,
        HeadlessOptions {
            width: 300.,
            height: 200.,
            ..Default::default()
        },
    )
    .unwrap();
    let row = ElementId::named("row").item(10);
    let top = app.bounds(row).unwrap().top;
    let views = app.runtime().stats().views;
    app.scroll("feed", 0., 60.).unwrap();
    assert_eq!(app.bounds(row).unwrap().top, top - 60.);
    assert_eq!(app.runtime().stats().views, views);
    app.scroll("feed", 0., -1000.).unwrap();
    assert_eq!(app.bounds(row).unwrap().top, top);
}

/// Champ multiligne dans une page défilante : police de 10 px (6 px par caractère,
/// lignes de 12,5 px), sans padding, 62 px de large (10 caractères) et 50 px de haut.
#[derive(Default)]
struct Journal {
    notes: String,
}
impl Application for Journal {
    type Message = String;
    fn view(&self, _: &ViewContext) -> Element {
        let notes = crate::ui::text_input::TextInput::new(self.notes.as_str(), |s| s)
            .multiline()
            .label("Notes")
            .style(crate::ui::text_input::InputStyle {
                font_size: 10.,
                padding: 0.,
                ..Default::default()
            });
        scroll(
            "page",
            Axis::Vertical,
            column(
                "content",
                [
                    crate::ui::view::widget("notes", notes).width(62).height(50),
                    column("spacer", []).height(800),
                ],
            ),
        )
        .height(FILL)
    }
    fn update(&mut self, notes: String) -> bool {
        self.notes = notes;
        true
    }
}

#[test]
fn multiline_input_types_lines_moves_vertically_and_scrolls_before_the_page() {
    let mut app = HeadlessApp::with_options(
        Journal::default(),
        HeadlessOptions {
            width: 300.,
            height: 200.,
            ..Default::default()
        },
    )
    .unwrap();
    app.click("notes").unwrap();
    app.type_text("une deux trois").unwrap();
    app.press(Key::Enter).unwrap();
    app.type_text("quatre").unwrap();
    assert_eq!(app.app().notes, "une deux trois\nquatre");
    let lines = |app: &HeadlessApp<Journal>| {
        app.accessible("notes")
            .and_then(|n| n.text)
            .map(|t| t.lines)
            .unwrap()
    };
    let ranges: Vec<_> = lines(&app).into_iter().map(|l| l.range).collect();
    assert_eq!(ranges, [0..9, 9..14, 15..21]);
    let selection = |app: &HeadlessApp<Journal>| {
        app.runtime()
            .input_snapshot(ElementId::named("notes"))
            .unwrap()
            .selection
    };
    app.press(Key::Up).unwrap();
    assert_eq!(selection(&app), 14..14);
    app.press(Key::Up).unwrap();
    assert_eq!(selection(&app), 6..6);
    app.press_with_shift(Key::Down).unwrap();
    assert_eq!(selection(&app), 6..14);
    app.edit(EditCommand::Copy).unwrap();
    assert_eq!(app.clipboard(), Some("ux trois"));
    // Sept lignes de 12,5 px dans 50 px : le caret, en dernière ligne, reste visible.
    app.press(Key::Down).unwrap();
    app.press(Key::Down).unwrap();
    for _ in 0..4 {
        app.press(Key::Enter).unwrap();
    }
    app.type_text("fin").unwrap();
    let field = app.bounds("notes").unwrap();
    let last = lines(&app).last().unwrap().rect;
    assert!(
        (last.bottom - field.bottom).abs() < 0.01,
        "{last:?} {field:?}"
    );
    // La molette fait d'abord défiler le champ, puis la page quand il est au bord.
    app.scroll("notes", 0., -1000.).unwrap();
    assert!((lines(&app)[0].rect.top - field.top).abs() < 0.01);
    assert_eq!(app.bounds("notes").unwrap().top, field.top);
    app.scroll("notes", 0., 50.).unwrap();
    let last = lines(&app).last().unwrap().rect;
    let moved = app.bounds("notes").unwrap();
    assert!(moved.top < field.top, "la page défile avec le reste");
    assert!((last.bottom - moved.bottom).abs() < 0.01);
    assert!(app.scene().validate().is_ok());
}
