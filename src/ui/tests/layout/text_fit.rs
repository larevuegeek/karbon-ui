//! Une largeur de texte mesurée doit contenir son texte quand le layout la rend :
//! un paragraphe ajusté à son contenu (ligne flex, largeur `Auto`) ne repasse pas à
//! la ligne au dessin (« Lectur/e », « 602 » → « 60 »), quel que soit l'arrondi de
//! sa position.
use crate::ui::tests::support::*;
use crate::ui::{
    styling::text::SpanStyle,
    view::{AUTO, widget},
    widgets::RichText,
};

const WORDS: [&str; 6] = [
    "Lecture",
    "602",
    "Bibliothèque",
    "Reprendre la lecture",
    "Écouté 12 fois",
    "Ensuite",
];

fn tree() -> Element {
    let mut rows = Vec::new();
    let mut id = 1;
    for size in [11., 13., 14., 17., 23.] {
        for (k, word) in WORDS.iter().enumerate() {
            for step in 0..8 {
                // Décalages fractionnaires : `right - left` perd des bits en f32.
                let offset = 0.13 + (k * 8 + step) as f32 * 0.37;
                let spacer = Element::row(id, Vec::new()).width(Length::Px(offset));
                let plain = widget(id + 1, Text::new(*word).size(size)).width(AUTO);
                let rich = RichText::new()
                    .size(size)
                    .text("› ")
                    .styled(*word, SpanStyle::new().bold())
                    .element(id + 2)
                    .width(AUTO);
                rows.push(Element::row(id + 3, vec![spacer, plain, rich]).gap(0.29));
                id += 4;
            }
        }
    }
    Element::column(0, rows)
}

/// Dispose l'arbre avec `text`, puis vérifie que chaque texte, mesuré à la largeur de
/// son rectangle final, garde une seule ligne.
pub(crate) fn verify_text_fits(text: &mut dyn TextMeasurer) {
    let mut root = tree();
    let bounds = RectF::new(0.31, 0.17, 2000., 30000.);
    layout::validate(&root).unwrap();
    layout::arrange(&mut root, bounds, bounds, text).unwrap();
    let mut checked = 0;
    for row in &root.children {
        for node in &row.children[1..] {
            let widget = node.widget.as_ref().unwrap();
            let rect = layout::content_rect(node);
            let width = rect.right - rect.left;
            let tall = SizeF {
                width: 16384.,
                height: 16384.,
            };
            let line = widget.measure(tall, text).unwrap();
            let laid = widget
                .measure(
                    SizeF {
                        width,
                        height: 16384.,
                    },
                    text,
                )
                .unwrap();
            assert!(
                laid.height <= line.height + 0.01,
                "{:?} wraps at its own width {width} (natural {})",
                widget.semantics().name,
                line.width
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 5 * WORDS.len() * 8 * 2);
}

/// Mesureur sans tolérance, comme DirectWrite : un texte plus large que la boîte, même
/// d'un ulp, passe à la ligne (le mesureur headless tolère 1e-3 avance).
struct Strict(crate::platform::headless::HeadlessTextMeasurer);
impl Strict {
    fn strict(natural: SizeF, measured: SizeF, width: f32) -> SizeF {
        if width < natural.width {
            SizeF {
                width: width.min(natural.width),
                height: natural.height * 2.,
            }
        } else {
            measured
        }
    }
}
impl TextMeasurer for Strict {
    fn measure_formatted(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        let natural = self.0.measure_formatted(value, format, 16384.)?;
        let measured = self.0.measure_formatted(value, format, width)?;
        Ok(Self::strict(natural, measured, width))
    }
    fn measure_rich(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
    ) -> crate::Result<SizeF> {
        let natural = self.0.measure_rich(paragraph, 16384.)?;
        let measured = self.0.measure_rich(paragraph, width)?;
        Ok(Self::strict(natural, measured, width))
    }
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: crate::ui::font::Font,
        width: f32,
    ) -> crate::Result<SizeF> {
        let natural = self.0.measure_font(text, size, font, 16384.)?;
        let measured = self.0.measure_font(text, size, font, width)?;
        Ok(Self::strict(natural, measured, width))
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        width: f32,
    ) -> crate::Result<SizeF> {
        let natural = self.0.measure_text(text, size, bold, 16384.)?;
        let measured = self.0.measure_text(text, size, bold, width)?;
        Ok(Self::strict(natural, measured, width))
    }
}

#[test]
fn headless_text_fits_its_measured_width() {
    verify_text_fits(&mut crate::platform::headless::HeadlessTextMeasurer);
    verify_text_fits(&mut Strict(crate::platform::headless::HeadlessTextMeasurer));
}
