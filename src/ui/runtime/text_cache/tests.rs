//! La mémoire des mesures de texte rend, à toute largeur, ce que le mesureur du backend
//! natif (Core Text, DirectWrite, ou les métriques headless) mesure lui-même : textes
//! coupés sur plusieurs lignes, ellipses de fin et de début, `line-clamp`, alignements,
//! espacement des lettres, paragraphes enrichis, justification.
use super::*;
use crate::{
    platform::{Backend, Native},
    ui::{
        font::Font,
        styling::{
            TextAlign, Typography, WhiteSpace,
            text::{RichParagraph, SpanStyle, TextFormat, TextSpan},
        },
    },
    window::RenderMode,
};

const LONG: &str = "Un paragraphe assez long pour se couper sur plusieurs lignes à la plupart des \
                    largeurs, avec des mots courts, des mots interminablement longs et une fin.";

fn formats() -> Vec<(&'static str, TextFormat)> {
    let base = TextFormat::new(Typography::default(), Font::system(false), 14.);
    let mut out = vec![("base", base)];
    let mut f = base;
    f.ellipsis = true;
    f.white_space = WhiteSpace::NoWrap;
    out.push(("ellipsis", f));
    f.ellipsis_start = true;
    out.push(("ellipsis-start", f));
    let mut f = base;
    f.max_lines = 2;
    f.ellipsis = true;
    out.push(("clamp", f));
    let mut f = base;
    f.letter_spacing = 1.5;
    f.font = Font::system(true);
    out.push(("spacing", f));
    let mut f = base;
    f.align = TextAlign::End;
    out.push(("end", f));
    let mut f = base;
    f.align = TextAlign::Center;
    f.letter_spacing = 1.5;
    f.font = Font::system(true);
    out.push(("center", f));
    let mut f = base;
    f.align = TextAlign::Justify;
    f.line_height = Some(20.);
    out.push(("justify", f));
    let mut f = base;
    f.white_space = WhiteSpace::NoWrap;
    f.italic = true;
    out.push(("nowrap", f));
    out
}

/// Largeurs décroissantes puis croissantes, fractionnaires, puis dans le désordre.
fn widths() -> Vec<f32> {
    let mut out: Vec<f32> = (0..400).map(|i| 620. - i as f32 * 1.537).collect();
    out.extend((0..300).map(|i| 4. + i as f32 * 2.113));
    let mut seed = 7u32;
    out.extend((0..300).map(|_| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        1. + (seed >> 16) as f32 % 700.
    }));
    out
}

#[test]
fn cached_measures_match_the_native_measurer_at_every_width() {
    let mut native = Native::offscreen(RenderMode::Software).unwrap();
    let texts = [
        LONG,
        "Kind of Blue",
        "OK Computer OKNOTOK 1997 2017",
        "a\nb c",
        "",
    ];
    for (name, format) in formats() {
        for text in texts {
            let mut cache = TextCache::default();
            let mut hits = 0;
            for width in widths() {
                let expected = native.measure_formatted(text, format, width).unwrap();
                let calls = cache.count;
                let measured = Cached {
                    inner: &mut native,
                    cache: &mut cache,
                }
                .measure_formatted(text, format, width)
                .unwrap();
                hits += usize::from(cache.count == calls);
                assert_eq!(
                    (measured.width.to_bits(), measured.height.to_bits()),
                    (expected.width.to_bits(), expected.height.to_bits()),
                    "{name} {text:?} at {width}: {measured:?} != {expected:?}"
                );
            }
            // La mémoire sert : hors justification, centrage et `line-clamp`, la
            // plupart des largeurs sont reprises.
            if !matches!(name, "justify" | "center" | "clamp") && !text.is_empty() {
                assert!(hits > 300, "{name} {text:?}: {hits} hits");
            }
        }
    }
    // Paragraphe enrichi : portions grasse, plus grande et italique.
    let paragraph = RichParagraph {
        text: LONG.into(),
        format: formats()[0].1,
        spans: vec![
            TextSpan {
                range: 3..13,
                style: SpanStyle {
                    weight: Some(700),
                    ..SpanStyle::new()
                },
            },
            TextSpan {
                range: 30..60,
                style: SpanStyle {
                    size: Some(20.),
                    italic: true,
                    ..SpanStyle::new()
                },
            },
        ],
    };
    let mut cache = TextCache::default();
    for width in widths() {
        let expected = native.measure_rich(&paragraph, width).unwrap();
        let measured = Cached {
            inner: &mut native,
            cache: &mut cache,
        }
        .measure_rich(&paragraph, width)
        .unwrap();
        assert_eq!(
            (measured.width.to_bits(), measured.height.to_bits()),
            (expected.width.to_bits(), expected.height.to_bits()),
            "rich at {width}"
        );
    }
}

#[test]
fn a_different_text_format_or_scale_is_measured_again() {
    struct Count(u32);
    impl TextMeasurer for Count {
        fn measure_text(
            &mut self,
            text: &str,
            size: f32,
            _: bool,
            max: f32,
        ) -> crate::Result<SizeF> {
            self.0 += 1;
            Ok(SizeF {
                width: (text.len() as f32 * size * 0.5).min(max),
                height: size,
            })
        }
    }
    let mut inner = Count(0);
    let mut cache = TextCache::default();
    cache.begin(1.);
    let format = TextFormat::new(Typography::default(), Font::system(false), 10.);
    let measure = |cache: &mut TextCache, inner: &mut Count, text: &str, f: TextFormat, w| {
        Cached { inner, cache }
            .measure_formatted(text, f, w)
            .unwrap()
    };
    measure(&mut cache, &mut inner, "abc", format, 100.);
    measure(&mut cache, &mut inner, "abc", format, 90.);
    assert_eq!(
        inner.0, 1,
        "a narrower width that still fits reuses the measure"
    );
    measure(&mut cache, &mut inner, "abc", format, 120.);
    assert_eq!(inner.0, 2, "a wider width may fit more words");
    measure(&mut cache, &mut inner, "abd", format, 100.);
    let mut bold = format;
    bold.font = Font::system(true);
    measure(&mut cache, &mut inner, "abc", bold, 100.);
    assert_eq!(inner.0, 4);
    cache.begin(2.);
    measure(&mut cache, &mut inner, "abc", format, 100.);
    assert_eq!(inner.0, 5, "a new display scale forgets every measure");
    cache.disabled = true;
    measure(&mut cache, &mut inner, "abc", format, 100.);
    assert_eq!(inner.0, 6);
}
