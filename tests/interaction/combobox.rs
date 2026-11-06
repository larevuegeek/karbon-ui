//! Champ à suggestions (`k-popover combobox`) : le focus reste dans le champ, la frappe
//! continue, `onkeydown` déplace la suggestion en surbrillance.
use crate::css;
use karbon_ui::{
    platform::headless::{HeadlessApp, HeadlessOptions},
    ui::{element::Role, prelude::*, runtime::Key},
};

/// Suggestions sous un champ (`k-popover combobox`) : le focus reste dans le champ, la
/// frappe continue, `onkeydown` déplace la suggestion en surbrillance.
#[derive(Clone, Debug, PartialEq)]
enum Find {
    Query(String),
    Key(Key),
    Close,
    Pick(usize),
    Page,
}

#[derive(Default)]
struct Search {
    query: String,
    open: bool,
    active: Option<usize>,
    picked: Vec<usize>,
    page: u32,
}

const SUGGESTIONS: [&str; 3] = ["Kings and Queens", "Que du love", "Queen"];

impl Application for Search {
    type Message = Find;
    fn view(&self, _: &ViewContext) -> Element {
        let cx = TemplateContext::default();
        ui! { @context(cx); @styles(css);
            <div class="page">
                <div class="bar">
                    <input id="field" class="search" type="search" value={self.query.as_str()}
                           oninput={Find::Query} onkeydown={Find::Key} aria-label="Recherche">
                    <button id="elsewhere" onclick={Find::Page}>Page</button>
                </div>
                {#if self.open}
                    <k-popover id="suggestions" anchor="field" combobox match-width aria-label="Suggestions"
                               onclose={Find::Close} class="menu">
                        {#each SUGGESTIONS as (i, s) (i)}
                            <div id={*s} class="item" class:active={self.active == Some(i)} role="option"
                                 onclick={Find::Pick(i)}>{s}</div>
                        {/each}
                    </k-popover>
                {/if}
            </div>
        }
    }
    fn update(&mut self, message: Find) -> bool {
        match message {
            Find::Query(q) => {
                self.open = q.len() >= 2;
                self.query = q;
                self.active = None;
            }
            Find::Key(Key::Down) => {
                self.active = Some(
                    self.active
                        .map_or(0, |i| (i + 1).min(SUGGESTIONS.len() - 1)),
                )
            }
            Find::Key(Key::Up) => self.active = self.active.and_then(|i| i.checked_sub(1)),
            Find::Key(Key::Enter) => {
                if let Some(i) = self.active.take() {
                    self.picked.push(i);
                    self.open = false;
                }
            }
            Find::Key(_) => {}
            Find::Close => self.open = false,
            Find::Pick(i) => self.picked.push(i),
            Find::Page => self.page += 1,
        }
        true
    }
}

#[test]
fn a_combobox_keeps_the_focus_in_its_field_and_follows_its_keys() -> karbon_ui::Result<()> {
    let mut app = HeadlessApp::with_options(
        Search::default(),
        HeadlessOptions {
            width: 800.,
            height: 500.,
            ..Default::default()
        },
    )?;
    app.focus("field")?;
    app.type_text("qu")?;
    assert!(app.app().open);
    // La liste s'ouvre sous le champ sans lui prendre le focus : la frappe continue.
    assert_eq!(app.focused(), Some(ElementId::named("field")));
    let field = app.bounds("field").unwrap();
    let list = app.bounds("suggestions").unwrap();
    assert!(list.top >= field.bottom && (list.left - field.left).abs() < 0.01);
    assert_eq!(app.accessible("suggestions").unwrap().role, Role::List);
    app.type_text("e")?;
    assert_eq!(app.input_value("field").as_deref(), Some("que"));
    assert_eq!(app.focused(), Some(ElementId::named("field")));
    // Haut/Bas vont à l'application (onkeydown), pas au focus.
    app.press(Key::Down)?;
    app.press(Key::Down)?;
    assert_eq!(app.app().active, Some(1));
    assert_eq!(app.focused(), Some(ElementId::named("field")));
    app.press(Key::Up)?;
    assert_eq!(app.app().active, Some(0));
    app.press(Key::Enter)?;
    assert_eq!(app.app().picked, [0]);
    assert!(!app.app().open);
    // Un clic sur une suggestion la choisit et ferme la liste.
    app.type_text("e")?;
    assert!(app.app().open);
    app.click("Queen")?;
    assert_eq!(app.app().picked, [0, 2]);
    assert!(!app.app().open);
    // Échap ferme ; un clic ailleurs ferme et agit quand même sur la page.
    app.focus("field")?;
    app.type_text("n")?;
    app.press(Key::Escape)?;
    assert!(!app.app().open);
    assert_eq!(app.focused(), Some(ElementId::named("field")));
    app.type_text("s")?;
    assert!(app.app().open);
    app.click("elsewhere")?;
    assert!(!app.app().open);
    assert_eq!(app.app().page, 1);
    Ok(())
}
