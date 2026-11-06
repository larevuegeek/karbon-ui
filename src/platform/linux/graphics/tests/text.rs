//! Texte Pango : polices embarquées WOFF2 (prioritaires sur les polices installées du même
//! nom), mesure cohérente avec le dessin, lignes de saisie, ellipses, limite de lignes et
//! paragraphes enrichis.
use super::*;
use crate::ui::{
    element::TextMeasurer,
    font::{Font, FontAsset},
    scene::RectF,
    styling::{
        TextAlign,
        text::{RichParagraph, SpanStyle, TextFormat, TextSpan},
    },
};
use std::sync::Arc;

fn fonts() -> Renderer {
    let mut renderer = renderer();
    renderer
        .load_fonts(&[
            FontAsset {
                family: "Figtree Light",
                data: include_bytes!("../../../../../tests/fixtures/fonts/figtree.woff2"),
            },
            FontAsset {
                family: "Inter",
                data: include_bytes!("../../../../../tests/fixtures/fonts/inter.woff2"),
            },
        ])
        .unwrap();
    renderer
}
fn format(family: &'static str, size: f32) -> TextFormat {
    TextFormat::new(Default::default(), Font::new(family, 400), size)
}
#[test]
fn embedded_woff2_fonts_are_measured_by_family_and_weight_and_drawn() {
    let mut r = fonts();
    let light = r
        .measure_font("Musique HD", 20., Font::new("Inter", 300), 1000.)
        .unwrap();
    let bold = r
        .measure_font("Musique HD", 20., Font::new("Inter", 800), 1000.)
        .unwrap();
    let figtree = r
        .measure_font("Musique HD", 20., Font::new("Figtree Light", 400), 1000.)
        .unwrap();
    assert!(light.width > 30. && light.height > 15.);
    assert!(bold.width > light.width + 1., "la graisse suit l'axe wght");
    assert!((figtree.width - light.width).abs() > 0.5);
    // Liste CSS : la première famille disponible est retenue.
    let listed = r
        .measure_font("Musique HD", 20., Font::new("Absente, Inter", 300), 1000.)
        .unwrap();
    assert_eq!(listed, light);
    let mut scene = clear();
    scene.font_text(
        RectF::new(5., 5., 290., 40.),
        "Musique HD",
        20.,
        Font::new("Inter", 700),
        Color::rgb(255, 255, 255),
    );
    let px = r.render(&scene, 300, 50, 1.).unwrap();
    assert!(px.as_chunks::<4>().0.iter().any(|p| p[3] > 200));
}
#[test]
fn wrapping_line_height_clamp_and_system_aliases() {
    let mut r = fonts();
    let text = "Une phrase suffisamment longue pour occuper plusieurs lignes.";
    let one = r
        .measure_font("M", 16., Font::new("Inter", 400), 1000.)
        .unwrap();
    let wrapped = r
        .measure_font(text, 16., Font::new("Inter", 400), 120.)
        .unwrap();
    assert!(
        wrapped.height > one.height * 2.5 && wrapped.width <= 121.,
        "{wrapped:?}"
    );
    let mut uniform = format("Inter", 16.);
    uniform.line_height = Some(30.);
    let lines = (wrapped.height / one.height).round();
    let tall = r.measure_formatted(text, uniform, 120.).unwrap();
    assert!((tall.height - lines * 30.).abs() < 0.01, "{tall:?}");
    uniform.max_lines = 2;
    assert!((r.measure_formatted(text, uniform, 120.).unwrap().height - 60.).abs() < 0.01);
    // « Segoe UI » (police système du moteur) désigne la police d'interface (`system-ui`).
    let system = r.measure_text("Bonjour", 14., false, 1000.).unwrap();
    let bold = r.measure_text("Bonjour", 14., true, 1000.).unwrap();
    assert!(system.width > 20. && bold.width > system.width);
}
#[test]
fn editing_lines_have_ordered_carets_and_grapheme_clusters() {
    let mut r = fonts();
    let value = "e\u{301}t\u{e9} 👍🏽!";
    let line = r
        .layout_formatted_line(value, format("Inter", 16.))
        .unwrap();
    assert_eq!(line.carets.first().map(|c| c.byte), Some(0));
    assert_eq!(line.carets.last().map(|c| c.byte), Some(value.len()));
    assert!(
        line.carets
            .windows(2)
            .all(|w| w[0].byte <= w[1].byte && w[0].x <= w[1].x + 0.01)
    );
    // « e » + accent combinant et l'émoji avec son modificateur sont des clusters uniques.
    assert_eq!(line.clusters[0].start..line.clusters[0].end, 0..3);
    assert!(line.clusters.iter().any(|c| &value[c.start..c.end] == "👍🏽"));
    assert!(line.width > 30. && line.height > 15.);
    // Un champ vide garde la hauteur d'une ligne de texte (l'émoji, d'une autre police,
    // peut agrandir la ligne ci-dessus).
    let text = r
        .layout_formatted_line("abc", format("Inter", 16.))
        .unwrap();
    let empty = r.layout_formatted_line("", format("Inter", 16.)).unwrap();
    assert_eq!((empty.width, empty.carets.len()), (0., 1));
    assert!((empty.height - text.height).abs() < 0.01);
}
#[test]
fn ellipses_keep_text_within_the_box() {
    let mut r = fonts();
    let path = "D:\\Music\\FLAC\\Led Zeppelin\\Stairway To Heaven.flac";
    let mut end = format("Inter", 14.);
    end.ellipsis = true;
    end.white_space = crate::ui::styling::WhiteSpace::NoWrap;
    let natural = r
        .measure_formatted(
            path,
            {
                let mut f = end;
                f.ellipsis = false;
                f
            },
            100.,
        )
        .unwrap();
    let cut = r.measure_formatted(path, end, 100.).unwrap();
    assert!(
        natural.width > 200. && cut.width <= 100.5,
        "{natural:?} {cut:?}"
    );
    let mut start = end;
    start.ellipsis_start = true;
    let paragraph = RichParagraph {
        text: Arc::from(path),
        format: start,
        spans: Vec::new(),
    };
    let cut = r.measure_rich(&paragraph, 100.).unwrap();
    assert!(cut.width <= 100.5 && cut.width > 50., "{cut:?}");
}
#[test]
fn rich_paragraphs_measure_their_spans_and_report_link_rects_per_line() {
    let mut r = fonts();
    let mut base = format("Inter", 16.);
    base.align = TextAlign::Start;
    let text = "Voir la file d'attente complète maintenant";
    let plain = RichParagraph {
        text: Arc::from(text),
        format: base,
        spans: Vec::new(),
    };
    let rich = RichParagraph {
        spans: vec![TextSpan {
            range: 0..4,
            style: SpanStyle::new().bold().size(24.),
        }],
        ..plain.clone()
    };
    let (a, b) = (
        r.measure_rich(&plain, 1000.).unwrap(),
        r.measure_rich(&rich, 1000.).unwrap(),
    );
    assert!(b.width > a.width && b.height > a.height);
    let link = text.find("attente").unwrap();
    let rects = r.rich_text_rects(&plain, 90., link..text.len()).unwrap();
    assert!(rects.len() >= 2, "{} rectangles", rects.len());
    assert!(rects.windows(2).all(|w| w[1].top > w[0].top));
    assert!(r.rich_text_rects(&plain, 90., 0..1).unwrap()[0].left.abs() < 1.);
}
#[test]
fn wrapped_editing_lines_follow_pango_breaks() {
    let mut r = fonts();
    let value = "Bonjour le monde entier\n\nfin\n";
    let width = 90.;
    let lines = r
        .layout_wrapped_text(value, format("Inter", 16.), width)
        .unwrap();
    assert!(lines.len() >= 5, "{}", lines.len());
    assert_eq!(lines[0].range.start, 0);
    assert_eq!(lines.last().unwrap().range, value.len()..value.len());
    for pair in lines.windows(2) {
        let (a, b) = (&pair[0].range, &pair[1].range);
        // Coupure automatique contiguë, ou retour explicite d'un octet.
        assert!(b.start == a.end || (b.start == a.end + 1 && &value[a.end..b.start] == "\n"));
    }
    assert!(lines.iter().filter(|l| !l.range.is_empty()).count() >= 3);
    for line in &lines {
        let carets = &line.line.carets;
        assert_eq!(carets.first().unwrap().byte, line.range.start);
        assert_eq!(carets.last().unwrap().byte, line.range.end);
        let content = value[line.range.clone()].trim_end();
        let end = line.range.start + content.len();
        let x = carets.iter().find(|c| c.byte == end).unwrap().x;
        assert!(x <= width + 0.5, "{content:?} {x}");
        assert!(line.line.height > 15.);
    }
    // Même coupure que la mesure du paragraphe.
    let size = r
        .measure_formatted(value, format("Inter", 16.), width)
        .unwrap();
    let height: f32 = lines.iter().map(|l| l.line.height).sum();
    assert!(
        (size.height - height).abs() < 0.5,
        "{} {height}",
        size.height
    );
}
#[test]
fn embedded_fonts_win_over_installed_families_and_aliases_map_to_generics() {
    let mut r = fonts();
    // Une police installée du même nom (Inter sur certains bureaux) ne remplace pas la
    // police embarquée, enregistrée sous une famille privée.
    let desc = r.fonts.font("Inter", 400, false, 16.).unwrap();
    assert_eq!(
        r.fonts.map().loaded_family(&desc).as_deref(),
        Some("karbon-embedded-inter")
    );
    // « Segoe UI » n'est pas cherché par approximation (fontconfig proposerait une famille
    // au nom voisin) : c'est la police d'interface du système.
    let segoe = r.fonts.font("Segoe UI", 400, false, 16.).unwrap();
    let system = r.fonts.font("system-ui", 400, false, 16.).unwrap();
    assert_eq!(
        r.fonts.map().loaded_family(&segoe),
        r.fonts.map().loaded_family(&system)
    );
    // Sans police embarquée, la même famille vient du système.
    let mut plain = renderer();
    let installed = plain.fonts.font("Inter", 400, false, 16.).unwrap();
    assert_ne!(
        plain.fonts.map().loaded_family(&installed).as_deref(),
        Some("karbon-embedded-inter")
    );
}
#[test]
fn windows_follow_the_desktop_fonts_and_exports_keep_fontconfig_generics() {
    let mut export = renderer();
    let generic = export.fonts.font("system-ui", 400, false, 16.).unwrap();
    let generic = export.fonts.map().loaded_family(&generic);
    let mut window = renderer();
    window.follow_desktop();
    // Les boucles chargent les polices de l'application après `follow_desktop` : une
    // application sans police embarquée garde celles du bureau.
    window.load_fonts(&[]).unwrap();
    let desktop = window.fonts.font("system-ui", 400, false, 16.).unwrap();
    let desktop = window.fonts.map().loaded_family(&desktop);
    match &super::super::super::desktop::fonts().interface {
        // Police du bureau installée (GNOME : « Inter 10 ») : elle sert les fenêtres.
        Some(family) if window.fonts.font(family, 400, false, 16.).is_ok() => {
            let installed = window.fonts.font(family, 400, false, 16.).unwrap();
            assert_eq!(desktop, window.fonts.map().loaded_family(&installed));
        }
        // Bureau sans réglage lisible : fenêtres et exports emploient le même générique.
        _ => assert_eq!(desktop, generic),
    }
}
