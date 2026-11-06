//! Texte : héritage de la typographie dans les textes, boutons, champs et cases ; casse,
//! espacement des lettres, coupure, ellipse, limite de lignes, chiffres tabulaires,
//! listes de familles et texte riche.
use crate::support::*;
use karbon_ui::{
    platform::headless::HeadlessTextMeasurer,
    ui::{
        element::TextMeasurer,
        prelude::*,
        scene::{DrawCommand, PointF, Scene},
        styling::{StyleEnvironment, TextTransform, WhiteSpace, text::TextFormat},
    },
};
css_file!(mod css, "text.css");

#[test]
fn typography_inherits_into_text_buttons_inputs_and_choice_controls() {
    let mut root = ui! {@styles(css);<div class="typography">
        <span>Texte</span><button onclick={Msg::Click(1)}>Bouton</button>
        <input type="text" value="Valeur" oninput={Msg::Text}>
        <input type="checkbox" checked={false} onchange={Msg::Checked}> Case
        <input type="checkbox" switch checked={true} onchange={Msg::Checked}> Switch
        <div aria-label="Groupe">
            <input type="radio" name="groupe" checked={true} onchange={Msg::Click(5)}> Radio
        </div>
        <span class="override">Local</span>
    </div>};
    arrange(&mut root, 600., 500.);
    for (index, node) in root.children[..6].iter().enumerate() {
        let scene = paint(if index == 5 { &node.children[0] } else { node });
        assert!(scene.commands.iter().any(|c|matches!(c,DrawCommand::FormattedText{format,color,..}if format.font.family=="Consolas"&&format.font.weight==600&&format.size==20.&&format.line_height==Some(30.)&&format.decoration.underline&&*color==Color::hex(0xe03040))),"widget {index}");
    }
    let scene = paint(&root.children[6]);
    assert!(scene.commands.iter().any(|c|matches!(c,DrawCommand::FormattedText{format,color,..}if format.size==12.&&format.line_height==Some(18.)&&!format.decoration.underline&&*color==Color::hex(0xffffff))));
}

#[test]
fn input_uses_inherited_font_and_line_metrics_in_the_live_editor() {
    let mut ui=karbon_ui::ui::runtime::UiRuntime::new(View(||ui!{@styles(css);<div class="typography"><input id="input" type="text" style="width:250px; height:60px" value="iiWWW" oninput={Msg::Text}></div>}),&Services).unwrap();
    prepare(&mut ui, std::time::Instant::now());
    assert!(ui.scene().commands.iter().any(|c|matches!(c,DrawCommand::FormattedText{format,text,..}if text.as_ref()=="iiWWW"&&format.font.family=="Consolas"&&format.size==20.&&format.line_height==Some(30.))));
}

/// Premier texte formaté d'une scène : (texte, format, retour à la ligne).
fn formatted(scene: &Scene) -> (String, TextFormat, bool) {
    scene
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::FormattedText {
                text, format, wrap, ..
            } => Some((text.to_string(), *format, *wrap)),
            _ => None,
        })
        .expect("formatted text")
}

#[test]
fn text_transform_and_letter_spacing_inherit_and_follow_states() {
    let mut root = ui! {@styles(css);<div class="caps" style="width: 300px">
        <div>héllo ßtraße</div>
        <div class="capitalize">deux mots-composés</div>
    </div>};
    arrange_headless(&mut root, 300., 200.);
    let (text, format, _) = formatted(&paint_tree(&root.children[0]));
    assert_eq!(text, "HÉLLO SSTRASSE");
    // 0.1em sur une police de 20 px, calculé sur l'élément déclarant puis hérité.
    assert_eq!(format.letter_spacing, 2.);
    assert_eq!(format.transform, TextTransform::Uppercase);
    // Espacement : 14 caractères × (0,6 × 20 + 2) = 196 px.
    let size = HeadlessTextMeasurer::new()
        .measure_formatted(&text, format, 1000.)
        .unwrap();
    assert_eq!(size.width, 196.);
    let (text, format, _) = formatted(&paint_tree(&root.children[1]));
    assert_eq!(text, "Deux Mots-Composés");
    assert_eq!(format.letter_spacing, 2.);
    // Survol du conteneur : la règle d'état retire la casse et l'espacement.
    arrange_headless_in(
        &mut root,
        300.,
        200.,
        StyleEnvironment {
            pointer: Some(PointF { x: 5., y: 5. }),
            ..Default::default()
        },
    );
    let (text, format, _) = formatted(&paint_tree(&root.children[1]));
    assert_eq!(text, "Deux Mots-Composés");
    assert_eq!(format.letter_spacing, 2.);
    let scene = paint_tree(&root.children[0]);
    let text = scene
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::FontText { text, .. } | DrawCommand::FormattedText { text, .. } => {
                Some(text.to_string())
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(text, "héllo ßtraße");
}

#[test]
fn nowrap_ellipsis_and_line_clamp_bound_measure_and_paint() {
    let long = "alpha beta gamma delta epsilon zeta eta theta";
    let mut root = ui! {@styles(css);<div style="width: 60px">
        <p class="truncate">{long}</p>
        <p class="clamp">{long}</p>
        <p class="pre">{"a  b\nc"}</p>
    </div>};
    arrange_headless(&mut root, 60., 400.);
    // Une ligne de 10 px × 1,25, largeur bornée par la boîte.
    let truncate = root.children[0].bounds();
    assert_eq!(truncate.bottom - truncate.top, 12.5);
    let (_, format, wrap) = formatted(&paint_tree(&root.children[0]));
    assert!(format.ellipsis && !wrap && format.white_space == WhiteSpace::NoWrap);
    // Deux lignes gardées sur sept.
    let clamp = root.children[1].bounds();
    assert_eq!(clamp.bottom - clamp.top, 25.);
    let (_, format, wrap) = formatted(&paint_tree(&root.children[1]));
    assert!(wrap && format.max_lines == 2);
    let (text, format, wrap) = formatted(&paint_tree(&root.children[2]));
    assert_eq!(text, "a  b\nc");
    assert!(!wrap && format.white_space == WhiteSpace::Pre);

    // Métriques du backend headless : `…` remplace les derniers caractères.
    let mut measurer = HeadlessTextMeasurer::new();
    let mut f = TextFormat::new(
        Default::default(),
        karbon_ui::ui::font::Font::system(false),
        10.,
    );
    f.white_space = WhiteSpace::NoWrap;
    f.ellipsis = true;
    let size = measurer.measure_formatted(long, f, 60.).unwrap();
    assert_eq!((size.width, size.height), (60., 12.5));
    f.white_space = WhiteSpace::Normal;
    f.ellipsis = false;
    f.max_lines = 2;
    let size = measurer.measure_formatted(long, f, 60.).unwrap();
    assert_eq!(size.height, 25.);
    f.max_lines = 0;
    f.letter_spacing = 4.;
    let size = measurer.measure_formatted("abc", f, 1000.).unwrap();
    assert_eq!(size.width, 30.);
}

#[test]
fn italic_tabular_digits_and_family_lists_reach_the_text_format() {
    let mut root = ui! {@styles(css);<div class="numbers"><div>0123</div>
    <p>Paragraphe <strong>riche</strong></p></div>};
    arrange_headless(&mut root, 300., 100.);
    let (_, format, _) = formatted(&paint_tree(&root.children[0]));
    assert!(format.italic && format.tabular_numbers);
    assert_eq!(format.font.family, "Missing Family,Inter,Segoe UI");
    let scene = paint_tree(&root.children[1]);
    let paragraph = scene
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::RichText { paragraph, .. } => Some(paragraph.clone()),
            _ => None,
        })
        .unwrap();
    assert!(paragraph.format.italic && paragraph.format.tabular_numbers);
}

#[test]
fn rich_text_runs_keep_their_ranges_after_case_transform() {
    let mut root = ui! {@styles(css);<div class="caps" style="width: 400px">
    <p>a <strong>straße</strong> b</p></div>};
    arrange_headless(&mut root, 400., 100.);
    let scene = paint_tree(&root.children[0]);
    let paragraph = scene
        .commands
        .iter()
        .find_map(|c| match c {
            DrawCommand::RichText { paragraph, .. } => Some(paragraph.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(&*paragraph.text, "A STRASSE B");
    let span = &paragraph.spans[0];
    assert_eq!(&paragraph.text[span.range.clone()], "STRASSE");
}
