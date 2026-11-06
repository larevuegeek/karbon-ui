//! Tests des métriques fixes : coupures, ellipses et hauteur de ligne explicite.
use super::{
    HeadlessTextMeasurer,
    lines::{formatted_lines, wrap},
    rich::rich_lines,
};
use crate::ui::{
    element::TextMeasurer,
    font::Font,
    styling::text::{RichParagraph, TextFormat},
};

#[test]
fn rtl_direction_moves_the_ellipsis_to_the_start() {
    use crate::ui::styling::{TextOverflow, Typography, WhiteSpace};
    let text = Typography {
        overflow: Some(TextOverflow::Ellipsis),
        white_space: Some(WhiteSpace::NoWrap),
        rtl: Some(true),
        ..Default::default()
    };
    let format = TextFormat::new(text, Font::system(false), 10.);
    assert!(format.ellipsis_start);
    // 6 px par caractère, 60 px : `…` + les 9 derniers caractères.
    let path = r"C:\Music\FLAC\a.flac";
    let lines = formatted_lines(path, &format, 60., true);
    assert_eq!(lines, [r"…AC\a.flac"]);
    let paragraph = RichParagraph {
        text: path.into(),
        format,
        spans: Vec::new(),
    };
    let lines = rich_lines(&paragraph, 60., true);
    let glyphs = &lines[0].glyphs;
    assert_eq!(glyphs.len(), 10);
    // Le signe `…` est à gauche, le dernier caractère gardé finit dans la boîte.
    assert_eq!(glyphs[0].x, 0.);
    assert_eq!(glyphs[0].start, glyphs[0].end);
    assert_eq!(&paragraph.text[glyphs[1].start..], r"AC\a.flac");
    assert!(glyphs.last().map(|g| g.x + g.advance).unwrap() <= 60.);
    // Sans rtl : ellipse finale habituelle.
    let end = TextFormat {
        ellipsis_start: false,
        ..format
    };
    assert_eq!(
        formatted_lines("abcdefghijkl", &end, 60., true),
        ["abcdefghi…"]
    );
    // Paragraphe coupé : portions décalées derrière le signe.
    let styled = RichParagraph {
        text: "abcdef".into(),
        format,
        spans: vec![crate::ui::styling::text::TextSpan {
            range: 1..4,
            style: crate::ui::styling::text::SpanStyle::new().bold(),
        }],
    };
    let cut = styled.cut_start(2);
    assert_eq!(&*cut.text, "…cdef");
    assert_eq!(cut.spans[1].range, 3..5);
    assert!(cut.validate().is_ok());
    let at =
        crate::ui::styling::text::cut_start("abcdefgh", |c| Ok(c.chars().count() <= 4)).unwrap();
    assert_eq!(at, 5);
}
#[test]
fn fixed_metrics_wrap_at_spaces_and_break_long_words() {
    let mut text = HeadlessTextMeasurer::new();
    let one = text.measure_text("Hello", 20., false, 1000.).unwrap();
    assert_eq!((one.width, one.height), (60., 25.));
    // 80 px = 6 caractères de 12 px : coupure à l'espace, sans le compter.
    assert_eq!(wrap("Hello world", 6), ["Hello", "world"]);
    let two = text.measure_text("Hello world", 20., true, 80.).unwrap();
    assert_eq!((two.width, two.height), (60., 50.));
    assert_eq!(wrap("abcdefgh", 3), ["abc", "def", "gh"]);
    assert_eq!(wrap("a\n\nb\r\n", 10), ["a", "", "b", ""]);
    assert_eq!(wrap("aa   bb", 3), ["aa", "bb"]);
    assert_eq!(wrap("   abcd", 3), ["   ", "abc", "d"]);
    let empty = text.measure_text("", 10., false, 100.).unwrap();
    assert_eq!((empty.width, empty.height), (0., 12.5));
    // Une largeur mesurée puis réutilisée ne provoque pas de retour à la ligne.
    let natural = text
        .measure_text("Ajouter un titre", 14., false, 1e6)
        .unwrap();
    let again = text
        .measure_text("Ajouter un titre", 14., false, natural.width)
        .unwrap();
    assert_eq!(again.height, natural.height);
    assert!(text.measure_text("x", 0., false, 10.).is_err());
    assert!(
        text.measure_font("x", 10., Font::new("", 400), 10.)
            .is_err()
    );
}
#[test]
fn explicit_line_height_and_single_line_layout_are_consistent() {
    let mut text = HeadlessTextMeasurer::new();
    let format = TextFormat {
        line_height: Some(30.),
        ..TextFormat::new(Default::default(), Font::system(false), 10.)
    };
    let size = text.measure_formatted("ab cd", format, 12.).unwrap();
    assert_eq!((size.width, size.height), (12., 60.));
    let line = text.layout_formatted_line("aé", format).unwrap();
    assert_eq!(line.height, 30.);
    assert_eq!(line.width, 12.);
    assert_eq!(
        line.carets
            .iter()
            .map(|c| (c.byte, c.x))
            .collect::<Vec<_>>(),
        [(0, 0.), (1, 6.), (3, 12.)]
    );
    assert_eq!(line.clusters[1].start..line.clusters[1].end, 1..3);
}

#[test]
fn wrapped_editing_lines_keep_break_spaces_and_line_feeds() {
    let mut text = HeadlessTextMeasurer::new();
    let format = TextFormat::new(Default::default(), Font::system(false), 10.);
    // 6 px par caractère : 30 px tiennent 5 caractères.
    let lines = text
        .layout_wrapped_text("ab cd efghijk\n\nz  ", format, 30.)
        .unwrap();
    let ranges: Vec<_> = lines.iter().map(|l| l.range.clone()).collect();
    assert_eq!(ranges, [0..6, 6..11, 11..13, 14..14, 15..18]);
    assert_eq!(
        lines[1]
            .line
            .carets
            .iter()
            .map(|c| (c.byte, c.x))
            .next_back(),
        Some((11, 30.))
    );
    assert!((lines[2].line.height - 12.5).abs() < 1e-4);
    let empty = text.layout_wrapped_text("", format, 30.).unwrap();
    assert_eq!(empty.len(), 1);
    assert_eq!(empty[0].range, 0..0);
}
