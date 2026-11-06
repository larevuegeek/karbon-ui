//! Texte enrichi : coupure, liens cliquables et focalisables, nœud d'accessibilité
//! unique et rendu des portions.
use crate::{
    application::{Application, ViewContext},
    platform::headless::HeadlessApp,
    ui::{
        element::{Element, ElementId, Role},
        prelude::{column, text},
        runtime::Key,
        scene::{DrawCommand, PointF},
    },
};

/// Texte enrichi : un seul nœud de texte, des liens cliquables, focalisables et invocables.
#[derive(Default)]
struct Article {
    opened: Vec<u8>,
}
impl Application for Article {
    type Message = u8;
    fn view(&self, _: &ViewContext) -> Element {
        use crate::ui::{components::RichText, styling::text::SpanStyle};
        column(
            "page",
            [
                RichText::new()
                    .size(10.)
                    .text("Hello ")
                    .styled(
                        "you",
                        SpanStyle::new()
                            .bold()
                            .color(crate::ui::theme::Color::hex(0xff0000)),
                    )
                    .text(" see ")
                    .link("first", 1u8)
                    .text(" and a long tail that wraps then ")
                    .link("second link", 2u8)
                    .element("para")
                    .width(120.),
                text("count", format!("{}", self.opened.len())),
            ],
        )
        .padding(10.)
    }
    fn update(&mut self, message: u8) -> bool {
        self.opened.push(message);
        true
    }
}

#[test]
fn rich_text_wraps_hit_tests_links_and_exposes_one_text_node() {
    let mut app = HeadlessApp::new(Article::default()).unwrap();
    // 120 px à 6 px par caractère : 20 caractères par ligne.
    let para = app.bounds("para").unwrap();
    assert_eq!(para.bottom - para.top, 4. * 12.5);
    let nodes = app.accessibility();
    let text = nodes
        .iter()
        .find(|n| n.id == ElementId::named("para"))
        .unwrap();
    assert_eq!(text.role, Role::Text);
    assert!(text.name.starts_with("Hello you see first and"));
    let links = nodes
        .iter()
        .filter(|n| n.role == Role::Link)
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].name, "first");
    assert!(links.iter().all(|l| l.invokable && l.focusable));
    // « second link » est coupé sur deux lignes : sa boîte les couvre.
    assert_eq!(links[1].bounds.bottom - links[1].bounds.top, 2. * 12.5);
    // « first » : octets 14..19 de la première ligne.
    let first = links[0].bounds;
    assert_eq!((first.left, first.right), (10. + 14. * 6., 10. + 19. * 6.));
    app.click_at(PointF {
        x: first.left + 3.,
        y: first.top + 3.,
    })
    .unwrap();
    assert_eq!(app.app().opened, [1]);
    // Le texte ordinaire n'est pas une cible.
    app.click_at(PointF {
        x: para.left + 3.,
        y: para.top + 3.,
    })
    .unwrap();
    assert_eq!(app.app().opened, [1]);
    // Clavier : Tab atteint les liens dans l'ordre, Entrée active.
    app.press(Key::Tab).unwrap();
    app.press(Key::Tab).unwrap();
    app.press(Key::Enter).unwrap();
    assert_eq!(app.app().opened, [1, 2]);
    // Rendu : glyphes rouges pour « you », soulignement des liens.
    assert!(app.scene().commands.iter().any(|c| matches!(
        c,
        DrawCommand::RichText { paragraph, .. } if paragraph.spans.len() == 3
    )));
    let image = app.render(1.).unwrap();
    let at = |x: u32, y: u32| {
        let offset = ((y * image.width + x) * 4) as usize;
        image.pixels[offset..offset + 4].to_vec()
    };
    // « y » de « you » : colonne 6, centre de la boîte de glyphe (BGRA rouge).
    assert_eq!(at(10 + 6 * 6 + 3, 10 + 6), [0, 0, 255, 255]);
}
