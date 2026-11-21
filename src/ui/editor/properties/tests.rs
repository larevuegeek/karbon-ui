//! Grille de propriétés : sections qui se plient (souris et clavier), contrôles qui
//! envoient leurs messages, colonne des libellés alignée, bulle d'aide par ligne.
use super::*;
use crate::ui::{
    editor::{EditorPalette, FieldMsg, FieldState, NumberField, VectorField, testing::window},
    element::{Element, ElementId},
    prelude::{Application, ViewContext},
    runtime::Key,
    theme::Color,
};
use std::{collections::HashSet, time::Duration};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Fold(u64),
    Visible(bool),
    Name(String),
    Kind(u64),
    Pick,
    Field(FieldMsg),
}

/// Un inspecteur de lumière : deux sections, six sortes de contrôles.
struct Inspector {
    closed: HashSet<u64>,
    visible: bool,
    name: String,
    kind: u64,
    picks: u32,
    fields: FieldState,
    log: Vec<Msg>,
}

impl Default for Inspector {
    fn default() -> Self {
        Self {
            closed: HashSet::new(),
            visible: true,
            name: "Lampe".into(),
            kind: 0,
            picks: 0,
            fields: FieldState::default(),
            log: Vec::new(),
        }
    }
}

const GRID: &str = "inspector";

fn grid() -> ElementId {
    ElementId::named(GRID)
}

impl Application for Inspector {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element {
        let p = EditorPalette::DARK;
        let object = PropertySection::new(1, "Objet", !self.closed.contains(&1))
            .row(Property::new(
                "Nom",
                text_field("name", self.name.as_str(), Msg::Name, &p),
            ))
            .row(
                Property::new(
                    "Visible",
                    checkbox("visible", Some(self.visible), Msg::Visible, &p),
                )
                .tooltip("Montre l'objet dans la vue et dans le jeu."),
            )
            .row(Property::new("Identifiant", read_only("uid", "#42", &p)));
        let light = PropertySection::new(2, "Lumière", !self.closed.contains(&2))
            .row(Property::new(
                "Sorte",
                choice(
                    "kind",
                    self.kind,
                    &[(0, "Ponctuelle"), (1, "Spot")],
                    Msg::Kind,
                    &p,
                ),
            ))
            .row(Property::new(
                "Intensité",
                NumberField::new(1, 5., &self.fields, Msg::Field).element(),
            ))
            .row(Property::new(
                "Position",
                VectorField::new(2, [0.; 3], &self.fields, Msg::Field).element(),
            ))
            .row(Property::new(
                "Couleur",
                color_swatch("color", Color::hex(0xFFC080), Msg::Pick, &p),
            ));
        PropertyGrid::new(GRID, Msg::Fold)
            .section(object)
            .section(light)
            .element()
    }
    fn update(&mut self, message: Msg) -> bool {
        match &message {
            Msg::Fold(key) => {
                if !self.closed.remove(key) {
                    self.closed.insert(*key);
                }
            }
            Msg::Visible(on) => self.visible = *on,
            Msg::Name(name) => self.name = name.clone(),
            Msg::Kind(kind) => self.kind = *kind,
            Msg::Pick => self.picks += 1,
            Msg::Field(m) => {
                self.fields.update(m.clone());
            }
        }
        self.log.push(message);
        true
    }
}

fn header(section: u64) -> ElementId {
    grid().item(section).child("header")
}

#[test]
fn a_click_on_a_section_title_folds_and_unfolds_it() {
    let mut w = window(Inspector::default(), 420., 400.);
    assert!(w.find("kind").is_some());
    w.click(header(2)).unwrap();
    assert!(w.find("kind").is_none() && w.find("visible").is_some());
    w.click(header(2)).unwrap();
    assert!(w.find("kind").is_some());
    // Au clavier : Tab jusqu'au titre, Espace.
    w.focus(header(1)).unwrap();
    w.press(Key::Space).unwrap();
    assert!(w.find("visible").is_none());
    assert_eq!(w.app().log.last(), Some(&Msg::Fold(1)));
}

#[test]
fn the_controls_send_their_messages() {
    let mut w = window(Inspector::default(), 420., 400.);
    w.click("visible").unwrap();
    assert!(!w.app().visible);
    w.click("visible").unwrap();
    assert!(w.app().visible);
    w.click("color").unwrap();
    assert_eq!(w.app().picks, 1);
    w.click("name").unwrap();
    w.type_text("s").unwrap();
    assert_eq!(w.app().name, "Lampes");
}

#[test]
fn a_mixed_checkbox_becomes_checked() {
    #[derive(Clone)]
    struct Toggled(bool);
    struct Mixed(Option<bool>);
    impl Application for Mixed {
        type Message = Toggled;
        fn view(&self, _: &ViewContext) -> Element {
            checkbox("mixed", self.0, Toggled, &EditorPalette::DARK)
        }
        fn update(&mut self, message: Toggled) -> bool {
            self.0 = Some(message.0);
            true
        }
    }
    let mut w = window(Mixed(None), 200., 100.);
    w.click("mixed").unwrap();
    assert_eq!(w.app().0, Some(true));
}

#[test]
fn the_controls_start_on_the_same_column_and_rows_have_a_tooltip() {
    let mut w = window(Inspector::default(), 420., 400.);
    let lefts: Vec<f32> = ["name", "visible", "uid", "kind", "color"]
        .iter()
        .map(|id| w.bounds(*id).unwrap().left)
        .collect();
    assert!(
        lefts.iter().all(|x| (x - lefts[0]).abs() < 0.5),
        "{lefts:?}"
    );
    // Les lignes ont la hauteur d'une ligne d'éditeur.
    let visible_row = w.bounds(grid().item(1).item(1)).unwrap();
    assert!((visible_row.bottom - visible_row.top - 22.).abs() < 0.5);
    w.hover(grid().item(1).item(1)).unwrap();
    w.advance(Duration::from_millis(700)).unwrap();
    let tip = w.runtime().tooltip().expect("tooltip shown");
    assert_eq!(&*tip.text, "Montre l'objet dans la vue et dans le jeu.");
}
