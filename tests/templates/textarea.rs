//! `<textarea>` : champ multiligne compilé, stylé par la feuille comme `<input>`
//! (fond, bordure, padding, police, placeholder), piloté au clavier par le harnais
//! headless (6 px par caractère et lignes de 12,5 px à 10 px).
use karbon_ui::{
    platform::headless::HeadlessApp,
    ui::prelude::*,
    ui::{element::Role, runtime::Key, scene::DrawCommand},
};

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Notes(String),
}

css_file!(mod css, "textarea.css");

#[derive(Default)]
struct Form {
    notes: String,
}

impl Application for Form {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        ui! {
            @styles(css);
            <div>
                <textarea id="notes" class="notes" value={self.notes.as_str()} oninput={Msg::Notes}
                          placeholder="Votre message" aria-label="Message" maxlength="200"></textarea>
                <textarea id="tall" rows="4" readonly value={"Lecture\nseule"}></textarea>
            </div>
        }
    }
    fn update(&mut self, message: Msg) -> bool {
        let Msg::Notes(notes) = message;
        self.notes = notes;
        true
    }
}

#[test]
fn textarea_reuses_input_styles_and_edits_several_lines() {
    let mut app = HeadlessApp::new(Form::default()).unwrap();
    let fills = |app: &HeadlessApp<Form>, color: Color| {
        app.scene()
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Fill { color: c, .. } if *c == color))
    };
    assert!(fills(&app, Color::hex(0x102030)));
    assert!(app.contains_text("Votre message"));
    let notes = app.bounds("notes").unwrap();
    assert_eq!(
        (notes.right - notes.left, notes.bottom - notes.top),
        (200., 60.)
    );
    app.click("notes").unwrap();
    app.type_text("Bonjour").unwrap();
    app.press(Key::Enter).unwrap();
    app.type_text("tout le monde").unwrap();
    assert_eq!(app.app().notes, "Bonjour\ntout le monde");
    // Chaque ligne est dessinée séparément.
    assert!(app.contains_text("Bonjour"));
    assert!(app.contains_text("tout le monde"));
    let node = app.accessible("notes").unwrap();
    assert_eq!(node.role, Role::TextInput);
    assert_eq!(node.name, "Message");
    let text = node.text.unwrap();
    assert!(text.multiline);
    assert_eq!(
        text.lines.into_iter().map(|l| l.range).collect::<Vec<_>>(),
        [0..7, 8..21]
    );
    // Haut garde la colonne ; Tab quitte le champ sans rien insérer.
    app.press(Key::Up).unwrap();
    let selection = app
        .runtime()
        .input_snapshot(ElementId::named("notes"))
        .unwrap()
        .selection;
    assert_eq!(selection, 7..7);
    app.press(Key::Tab).unwrap();
    assert_eq!(app.focused(), Some(ElementId::named("tall")));
    assert_eq!(app.app().notes, "Bonjour\ntout le monde");
    // Lecture seule : Entrée n'insère rien. `rows` donne la hauteur naturelle.
    app.press(Key::Enter).unwrap();
    assert_eq!(app.input_value("tall").as_deref(), Some("Lecture\nseule"));
    let tall = app.bounds("tall").unwrap();
    assert!((tall.bottom - tall.top - (4. * 18.75 + 24.)).abs() < 0.01);
}
