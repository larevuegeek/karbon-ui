//! Chaîne attribuée Core Text d'un paragraphe : une police par portion, l'espacement des
//! lettres et la couleur prise dans le contexte de dessin.
use super::super::super::ffi::{cf_string, range, required};
use super::Fonts;
use crate::ui::styling::text::{RichParagraph, SpanStyle};
use objc2_core_foundation::{
    CFAttributedString, CFBoolean, CFDictionary, CFMutableAttributedString, CFNumber, CFRetained,
    CFString, CFType,
};
use objc2_core_text::{
    CTFont, CTLine, kCTFontAttributeName, kCTForegroundColorFromContextAttributeName,
    kCTKernAttributeName,
};

/// Portion de style homogène, en unités UTF-16.
pub(in crate::platform::macos) struct Segment {
    pub start: usize,
    pub end: usize,
    pub font: CFRetained<CTFont>,
    pub style: SpanStyle,
}
/// Portions du paragraphe (les intervalles hors `spans` prennent le style de base).
fn segments(paragraph: &RichParagraph) -> Vec<(std::ops::Range<usize>, SpanStyle)> {
    let mut result = Vec::new();
    let mut at = 0;
    for span in &paragraph.spans {
        if span.range.start > at {
            result.push((at..span.range.start, SpanStyle::new()));
        }
        if span.range.end > span.range.start {
            result.push((span.range.clone(), span.style));
        }
        at = span.range.end;
    }
    if at < paragraph.text.len() || result.is_empty() {
        result.push((at..paragraph.text.len(), SpanStyle::new()));
    }
    result
}
/// Police d'une portion : famille, graisse, taille et italique de base surchargés.
pub(super) fn font(
    fonts: &mut Fonts,
    paragraph: &RichParagraph,
    style: SpanStyle,
) -> crate::Result<CFRetained<CTFont>> {
    let f = paragraph.format;
    fonts.font(
        style.family.unwrap_or(f.font.family),
        style.weight.unwrap_or(f.font.weight),
        f.italic || style.italic,
        style.size.unwrap_or(f.size),
        f.tabular_numbers,
    )
}
/// Attributs d'une portion.
fn attributes(font: &CTFont, spacing: f32) -> CFRetained<CFDictionary<CFString, CFType>> {
    // SAFETY : clés constantes de Core Text.
    let (font_key, context_key, kern_key) = unsafe {
        (
            kCTFontAttributeName,
            kCTForegroundColorFromContextAttributeName,
            kCTKernAttributeName,
        )
    };
    let from_context = CFBoolean::new(true);
    if spacing == 0. {
        return CFDictionary::from_slices(
            &[font_key, context_key],
            &[font.as_ref(), from_context.as_ref()],
        );
    }
    let kern = CFNumber::new_f64(f64::from(spacing));
    CFDictionary::from_slices(
        &[font_key, context_key, kern_key],
        &[font.as_ref(), from_context.as_ref(), kern.as_ref()],
    )
}
/// Chaîne attribuée et portions de `paragraph` ; `utf16` donne l'index UTF-16 de chaque octet.
pub(super) fn build(
    fonts: &mut Fonts,
    paragraph: &RichParagraph,
    utf16: &[usize],
) -> crate::Result<(CFRetained<CFMutableAttributedString>, Vec<Segment>)> {
    let string = required(
        CFMutableAttributedString::new(None, 0),
        "CFAttributedStringCreateMutable",
    )?;
    let text = cf_string(&paragraph.text);
    // SAFETY : chaîne mutable vide ; remplacement de l'intervalle vide initial.
    unsafe { CFMutableAttributedString::replace_string(Some(&string), range(0, 0), Some(&text)) };
    let mut result = Vec::new();
    for (bytes, style) in segments(paragraph) {
        let font = font(fonts, paragraph, style)?;
        let (start, end) = (utf16[bytes.start], utf16[bytes.end]);
        if end > start {
            let attributes = attributes(&font, paragraph.format.letter_spacing);
            // SAFETY : intervalle dans la chaîne ; dictionnaire d'attributs valide.
            unsafe {
                CFMutableAttributedString::set_attributes(
                    Some(&string),
                    range(start, end - start),
                    Some(attributes.as_opaque()),
                    false,
                )
            };
        }
        result.push(Segment {
            start,
            end,
            font,
            style,
        });
    }
    Ok((string, result))
}
/// Ligne d'ellipse (`…`) dans la police et l'espacement donnés.
pub(super) fn ellipsis(font: &CTFont, spacing: f32) -> crate::Result<CFRetained<CTLine>> {
    let attributes = attributes(font, spacing);
    // SAFETY : chaîne et dictionnaire valides.
    let string = unsafe {
        CFAttributedString::new(
            None,
            Some(&cf_string("\u{2026}")),
            Some(attributes.as_opaque()),
        )
    };
    let string = required(string, "CFAttributedStringCreate")?;
    // SAFETY : chaîne attribuée valide.
    Ok(unsafe { CTLine::with_attributed_string(&string) })
}
