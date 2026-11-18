//! Texte en ligne et contrôles : classes et couleurs en variables des `<span>`, nom
//! accessible des boutons, largeur des listes déroulantes.
use super::{arrange, css};
use crate::support::{Msg, find};
use karbon_ui::ui::{
    element::PaintContext,
    layout,
    prelude::*,
    scene::{DrawCommand, Scene},
    theme::Theme,
};

fn paint(node: &Element) -> Scene {
    let mut scene = Scene::new(Color::hex(0));
    node.widget.as_ref().unwrap().paint(
        &PaintContext {
            rect: node.bounds(),
            intrinsic: Default::default(),
            theme: Theme::DARK,
            state: Default::default(),
        },
        &mut scene,
    );
    scene
}

#[test]
fn inline_spans_keep_their_classes_and_variable_colours() {
    let mut page = ui! {@styles(css);
        <div><p id="p">Rust<span class="font-mono text-xs text-red-500">Music</span> et <span class="text-(--panel)">Var<b>iable</b></span></p></div>
    };
    arrange(&mut page, 400., 300.);
    let scene = paint(find(&page, "p"));
    let paragraph = scene
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::RichText { paragraph, .. } => Some(paragraph.clone()),
            _ => None,
        })
        .expect("rich text");
    let style = |text: &str| {
        paragraph
            .spans
            .iter()
            .find(|s| &paragraph.text[s.range.clone()] == text)
            .unwrap_or_else(|| panic!("span {text}"))
            .style
    };
    let music = style("Music");
    assert_eq!(music.size, Some(12.));
    assert!(music.family.is_some_and(|f| f.contains("Consolas")));
    // `--panel` (#102030) déclaré par `:root` : évalué avec les variables du paragraphe,
    // et hérité par un élément en ligne imbriqué.
    assert_eq!(style("Var").color, Some(Color::hex(0x102030)));
    assert_eq!(style("iable").color, Some(Color::hex(0x102030)));
}

#[test]
fn aria_label_names_text_buttons_and_is_never_drawn_on_empty_ones() {
    let mut page = ui! {@styles(css);
        <div>
            <button id="close" class="w-[30px] h-[30px]" onclick={Msg::Click} aria-label="Fermer"></button>
            <button id="cross" onclick={Msg::Click} aria-label="Fermer la file">X</button>
        </div>
    };
    arrange(&mut page, 400., 300.);
    let close = find(&page, "close");
    let drawn = paint(close).commands.iter().any(|c| match c {
        DrawCommand::FormattedText { text, .. }
        | DrawCommand::FontText { text, .. }
        | DrawCommand::Text { text, .. } => !text.is_empty(),
        _ => false,
    });
    assert!(!drawn, "the accessible name is not drawn");
    assert_eq!(close.widget.as_ref().unwrap().semantics().name, "Fermer");
    let cross = find(&page, "cross").widget.as_ref().unwrap();
    assert_eq!(cross.semantics().name, "Fermer la file");
}

#[test]
fn a_select_is_as_wide_as_its_longest_option() {
    let mut page = ui! {@styles(css);
        <div class="flex items-start">
            <select id="short" value={1u64} onchange={|_: u64| Msg::Click}>
                <option value={1u64}>Un</option>
                <option value={2u64}>Deux</option>
            </select>
            <select id="long" value={1u64} onchange={|_: u64| Msg::Click}>
                <option value={1u64}>Un</option>
                <option value={2u64}>Une option nettement plus longue</option>
            </select>
        </div>
    };
    arrange(&mut page, 800., 300.);
    let short = layout::width(find(&page, "short").bounds());
    let long = layout::width(find(&page, "long").bounds());
    assert!(short < 120., "{short}");
    assert!(long > short + 150., "{long}");
}
