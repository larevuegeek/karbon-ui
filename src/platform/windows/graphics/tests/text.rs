//! Texte DirectWrite : polices embarquées, mesure cohérente avec le dessin, lignes de
//! saisie et paragraphes enrichis.
use super::super::*;
use crate::ui::{
    element::TextMeasurer,
    styling::{TextAlign, TextDecoration, text::TextFormat},
};

#[test]
fn embedded_woff2_fonts_are_private_measured_and_drawn_by_family_and_weight() {
    use crate::ui::element::Widget;
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
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
    let a = renderer
        .measure_font("Musique HD", 20., Font::new("Figtree Light", 700), 300.)
        .unwrap();
    let b = renderer
        .measure_font("Musique HD", 20., Font::new("Inter", 400), 300.)
        .unwrap();
    assert!(a.width > 30. && a.height > 10.);
    assert!((a.width - b.width).abs() > 0.1);
    let paragraph = crate::ui::widgets::Text::new(
        "Une phrase suffisamment longue pour occuper plusieurs lignes avec la police embarquée.",
    )
    .font("Inter")
    .weight(600)
    .size(18.);
    let measured = paragraph
        .measure(
            crate::ui::element::SizeF {
                width: 110.,
                height: 300.,
            },
            &mut renderer,
        )
        .unwrap();
    assert!(measured.height > 40.);
    let mut wrapped = Scene::new(Color::rgba(0, 0, 0, 0));
    paragraph.paint(
        &crate::ui::element::PaintContext {
            rect: RectF::new(0., 0., 110., 300.),
            intrinsic: measured,
            theme: crate::ui::theme::Theme::DARK,
            state: Default::default(),
        },
        &mut wrapped,
    );
    let wrapped = renderer.render(&wrapped, 110, 300, 1.).unwrap();
    assert!(
        wrapped
            .as_chunks::<4>()
            .0
            .iter()
            .skip(40 * 110)
            .any(|p| p[3] > 0),
        "Le paragraphe doit être dessiné après la première ligne"
    );
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.font_text(
        RectF::new(5., 5., 290., 40.),
        "Musique HD",
        20.,
        Font::new("Figtree Light", 700),
        Color::rgb(255, 255, 255),
    );
    assert!(
        renderer
            .render(&scene, 300, 60, 1.)
            .unwrap()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 0)
    );
    assert!(
        renderer
            .load_fonts(&[FontAsset {
                family: "Broken",
                data: b"bad font"
            }])
            .is_err()
    );
    assert!(renderer.render(&scene, 300, 60, 1.).is_ok());
}
/// Un texte disposé à sa largeur mesurée ne repasse pas à la ligne avec DirectWrite
/// (« Lectur/e », « 602 » → « 60 » dans une ligne flex ajustée au contenu).
#[test]
fn directwrite_text_fits_its_measured_width() {
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    crate::ui::tests::text_fit::verify_text_fits(&mut renderer);
}
#[test]
fn input_line_clusters_keep_utf8_boundaries_and_native_geometry() {
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    for value in ["", "Café 😃", "e\u{301}", "abc אבג مرحبا", "fi"] {
        let line = renderer.layout_line(value, 16.).unwrap();
        assert!(line.height > 0. && line.height.is_finite());
        assert!(
            line.carets
                .iter()
                .all(|c| value.is_char_boundary(c.byte) && c.x.is_finite())
        );
        assert!(line.carets.iter().any(|c| c.byte == 0));
        assert!(line.carets.iter().any(|c| c.byte == value.len()));
        assert!(line.clusters.iter().all(|c| c.start <= c.end
            && value.is_char_boundary(c.start)
            && value.is_char_boundary(c.end)));
        if value == "e\u{301}" {
            assert!(!line.carets.iter().any(|c| c.byte == 1));
        }
    }
}
#[test]
fn text_measurement_wraps_and_its_cache_is_bounded() {
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    let text = "Un texte qui occupe plusieurs lignes dans une colonne étroite.";
    let wide = renderer.measure_text(text, 16., false, 800.).unwrap();
    let narrow = renderer.measure_text(text, 16., false, 100.).unwrap();
    assert!(narrow.height > wide.height);
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.text_single_line(
        RectF::new(0., 0., 80., 30.),
        text,
        16.,
        false,
        Color::rgb(255, 255, 255),
    );
    renderer.render(&scene, 100, 100, 1.).unwrap();
    let narrow_after = renderer.measure_text(text, 16., false, 99.).unwrap();
    assert!(narrow_after.height > wide.height);
    for i in 0..300 {
        renderer
            .measure_text(&format!("Label {i}"), 14., true, 300.)
            .unwrap();
    }
    assert!(renderer.measurements.len() <= 256);
    for size in 10..30 {
        let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
        scene.text(
            RectF::new(0., 0., 100., 50.),
            "Cache",
            size as f32,
            false,
            Color::rgb(255, 255, 255),
        );
        renderer.render(&scene, 100, 100, 1.).unwrap();
        assert_eq!(renderer.formats.len(), 1);
    }
}
#[test]
fn rich_paragraphs_wrap_hit_test_ranges_and_draw_span_colors() {
    use crate::ui::styling::text::{RichParagraph, SpanStyle, TextSpan};
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    let text: Arc<str> = "Bonjour toi, lis le guide détaillé jusqu'au bout".into();
    let link = text.find("guide").unwrap();
    let red = Color::rgb(255, 0, 0);
    let paragraph = RichParagraph {
        text: Arc::clone(&text),
        format: TextFormat::new(Default::default(), Font::system(false), 16.),
        spans: vec![
            TextSpan {
                range: 0..7,
                style: SpanStyle::new().bold().italic(),
            },
            TextSpan {
                range: link..link + "guide détaillé".len(),
                style: SpanStyle::new()
                    .underline()
                    .color(red)
                    .size(20.)
                    .family("Consolas"),
            },
        ],
    };
    let wide = renderer.measure_rich(&paragraph, 1000.).unwrap();
    let narrow = renderer.measure_rich(&paragraph, 120.).unwrap();
    assert!(narrow.height > wide.height * 1.5, "{wide:?} {narrow:?}");
    let one = renderer
        .rich_text_rects(&paragraph, 1000., link..link + 5)
        .unwrap();
    assert_eq!(one.len(), 1);
    assert!(one[0].left > 0. && one[0].right > one[0].left);
    let wrapped = renderer
        .rich_text_rects(&paragraph, 120., link..link + "guide détaillé".len())
        .unwrap();
    assert!(!wrapped.is_empty());
    assert!(
        renderer
            .rich_text_rects(&paragraph, 120., 1..text.len() + 1)
            .is_err()
    );
    let mut scene = Scene::new(Color::rgb(0, 0, 0));
    scene.commands.push(DrawCommand::RichText {
        rect: RectF::new(0., 0., 1000., 60.),
        paragraph: Arc::new(paragraph),
        color: Color::rgb(255, 255, 255),
        wrap: true,
    });
    let pixels = renderer.render(&scene, 600, 60, 1.).unwrap();
    // Des pixels rouges (BGRA) sous le lien, blancs ailleurs.
    let reds = pixels
        .chunks(4)
        .filter(|p| p[2] > 200 && p[1] < 60 && p[0] < 60)
        .count();
    let whites = pixels
        .chunks(4)
        .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
        .count();
    assert!(reds > 20 && whites > 20, "{reds} {whites}");
}
pub(super) fn verify_typography(mode: RenderMode) {
    let mut renderer = Renderer::new(mode).unwrap();
    let format = TextFormat {
        clip: true,
        font: Font::system(false),
        size: 20.,
        align: TextAlign::Center,
        line_height: Some(30.),
        decoration: TextDecoration {
            underline: true,
            line_through: true,
            overline: true,
        },
        ..TextFormat::new(Default::default(), Font::system(false), 20.)
    };
    let measured = renderer
        .measure_formatted("First\nSecond", format, 200.)
        .unwrap();
    assert!((measured.height - 60.).abs() < 0.1, "{}", measured.height);
    let line = renderer.layout_formatted_line("iiWWW", format).unwrap();
    assert!(line.width > 20.);
    assert_eq!(line.carets.first().unwrap().byte, 0);
    assert_eq!(line.carets.last().unwrap().byte, 5);
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::FormattedText {
        rect: RectF::new(16., 16., 200., 70.),
        text: Arc::from("First\nSecond"),
        format,
        color: Color::rgb(255, 255, 255),
        wrap: true,
    });
    scene.fill(RectF::new(250., 10., 12., 12.), 0., Color::rgb(0, 255, 0));
    for scale in [1., 1.5, 2.] {
        let width = (300. * scale) as u32;
        let height = (140. * scale) as u32;
        let pixels = renderer.render(&scene, width, height, scale).unwrap();
        let mut ink = 0;
        for y in 0..height {
            for x in 0..(240. * scale) as u32 {
                let i = (y * width + x) as usize * 4;
                if pixels[i + 3] > 0 {
                    ink += 1;
                    assert!((x as f32) >= 16. * scale && (x as f32) < 216. * scale);
                    assert!((y as f32) >= 16. * scale && (y as f32) < 86. * scale);
                }
            }
        }
        assert!(ink > 100);
        let i = (((12. * scale) as u32 * width + (252. * scale) as u32) * 4) as usize;
        assert_eq!(&pixels[i..i + 4], [0, 255, 0, 255]);
    }
}
#[test]
fn software_formatted_text_metrics_decorations_clip_and_dpi() {
    verify_typography(RenderMode::Software);
}
#[test]
fn software_spacing_italic_tabular_ellipsis_clamp_and_family_lists() {
    use crate::ui::styling::WhiteSpace;
    let mut renderer = Renderer::new(RenderMode::Software).unwrap();
    let base = TextFormat::new(Default::default(), Font::system(false), 16.);
    let natural = renderer
        .measure_formatted("Espacement", base, 1000.)
        .unwrap();
    let spaced = TextFormat {
        letter_spacing: 3.,
        italic: true,
        tabular_numbers: true,
        ..base
    };
    let wide = renderer
        .measure_formatted(
            "Espacement",
            TextFormat {
                letter_spacing: 3.,
                ..base
            },
            1000.,
        )
        .unwrap();
    renderer.measure_formatted("0123", spaced, 1000.).unwrap();
    // Dix caractères, 3 px après chacun.
    assert!(
        (wide.width - natural.width - 30.).abs() < 1.,
        "{wide:?} {natural:?}"
    );
    let long = "Une phrase assez longue pour déborder de sa boîte sur plusieurs lignes";
    let ellipsis = TextFormat {
        white_space: WhiteSpace::NoWrap,
        ellipsis: true,
        ..base
    };
    let one = renderer.measure_formatted(long, ellipsis, 120.).unwrap();
    assert!(one.width <= 120.5 && one.height < 25., "{one:?}");
    let clamp = TextFormat {
        max_lines: 2,
        ..base
    };
    let all = renderer.measure_formatted(long, base, 120.).unwrap();
    let two = renderer.measure_formatted(long, clamp, 120.).unwrap();
    assert!(all.height > two.height * 1.4, "{all:?} {two:?}");
    // Liste de familles : la première absente est sautée, la mesure suit « Segoe UI ».
    let listed = TextFormat {
        font: Font::new("Absente Partout,Segoe UI", 400),
        ..base
    };
    let a = renderer
        .measure_formatted("Famille", listed, 1000.)
        .unwrap();
    let b = renderer
        .measure_formatted(
            "Famille",
            TextFormat {
                line_height: Some(1000.),
                ..base
            },
            1000.,
        )
        .unwrap();
    assert!((a.width - b.width).abs() < 0.5, "{a:?} {b:?}");
    // Au dessin, rien n'apparaît sous les deux lignes gardées.
    let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
    scene.commands.push(DrawCommand::FormattedText {
        rect: RectF::new(0., 0., 120., 200.),
        text: Arc::from(long),
        format: clamp,
        color: Color::rgb(255, 255, 255),
        wrap: true,
    });
    scene.commands.push(DrawCommand::FormattedText {
        rect: RectF::new(0., 0., 120., 200.),
        text: Arc::from(long),
        format: TextFormat {
            ellipsis: true,
            white_space: WhiteSpace::NoWrap,
            ..spaced
        },
        color: Color::rgb(255, 255, 255),
        wrap: false,
    });
    let pixels = renderer.render(&scene, 160, 200, 1.).unwrap();
    for y in (two.height.ceil() as u32 + 2)..200 {
        for x in 0..160 {
            assert_eq!(
                pixels[((y * 160 + x) * 4 + 3) as usize],
                0,
                "ink at {x},{y}"
            );
        }
    }
}
