//! Interfaces Text et EditableText d'un champ : offsets en caractères (AT-SPI) convertis
//! en octets (moteur), frontières de caractère, de mot et de ligne (lignes visuelles du
//! champ), sélection, étendues des caractères (clusters), remplacement du texte.
use super::properties::Prop;
use crate::{
    platform::linux::ffi::dbus::{Reader, Writer},
    ui::{
        runtime::{AccessibilityAction, AccessibleElement},
        text_input::AccessibleText,
    },
};
use std::sync::Arc;

/// Frontières de `GetTextAtOffset` (`AtspiTextBoundaryType`) et granularités de
/// `GetStringAtOffset` (`AtspiTextGranularity`) ; les autres valent la ligne.
const BOUNDARY_CHAR: u32 = 0;
const BOUNDARY_WORD_START: u32 = 1;
const BOUNDARY_WORD_END: u32 = 2;
const GRANULARITY_WORD: u32 = 1;
const GRANULARITY_PARAGRAPH: u32 = 4;

/// Octet du caractère `offset` (borné au texte).
fn byte(text: &str, offset: i32) -> usize {
    let offset = offset.max(0) as usize;
    text.char_indices()
        .nth(offset)
        .map_or(text.len(), |(b, _)| b)
}
/// Caractère de l'octet `byte`.
fn offset(text: &str, byte: usize) -> i32 {
    text.get(..byte.min(text.len()))
        .map_or(0, |t| t.chars().count() as i32)
}
pub(super) fn property(e: &AccessibleElement, name: &str) -> Option<Prop> {
    let t = e.text.as_ref()?;
    Some(match name {
        "CharacterCount" => Prop::Int(t.value.chars().count() as i32),
        "CaretOffset" => Prop::Int(offset(&t.value, t.selection.end)),
        _ => return None,
    })
}
/// Frontière « début de mot » : octets du mot commencé à ou avant `at` (blancs qui le
/// suivent compris), jusqu'au début du mot suivant.
fn word(text: &str, at: usize) -> (usize, usize) {
    let space = |c: char| c.is_whitespace();
    let start = text[..at]
        .trim_end_matches(space)
        .trim_end_matches(|c: char| !space(c))
        .len();
    let rest = &text[start..];
    let word_end = start + rest.find(space).unwrap_or(rest.len());
    let after = &text[word_end..];
    let end = word_end + (after.len() - after.trim_start_matches(space).len());
    (start, end)
}
/// Frontière « fin de mot » : de la fin du mot précédent (blancs compris) à la fin du mot
/// qui contient `at` ou le suit.
fn word_end(text: &str, at: usize) -> (usize, usize) {
    let space = |c: char| c.is_whitespace();
    let current = text[..at].trim_end_matches(|c: char| !space(c)).len();
    let start = text[..current].trim_end_matches(space).len();
    let rest = &text[current..];
    let next = current + (rest.len() - rest.trim_start_matches(space).len());
    let word = &text[next..];
    let end = next + word.find(space).unwrap_or(word.len());
    (start, end)
}
/// Ligne visuelle (ou paragraphe) qui contient `at`.
fn line(t: &AccessibleText, at: usize, paragraph: bool) -> (usize, usize) {
    if !paragraph && let Some(l) = t.lines.iter().rev().find(|l| l.range.start <= at) {
        // Le retour à la ligne final appartient à la ligne.
        let end = if t.value[l.range.end..].starts_with('\n') {
            l.range.end + 1
        } else {
            l.range.end
        };
        return (l.range.start, end.min(t.value.len()));
    }
    let start = t.value[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = t.value[at..]
        .find('\n')
        .map_or(t.value.len(), |i| at + i + 1);
    (start, end)
}
/// Morceau de texte autour de l'octet `at` selon `unit` (frontière ou granularité).
pub(super) fn piece(t: &AccessibleText, at: usize, unit: u32, granularity: bool) -> (usize, usize) {
    let text = &*t.value;
    let char_end = |b: usize| text[b..].chars().next().map_or(b, |c| b + c.len_utf8());
    match (granularity, unit) {
        (false, BOUNDARY_CHAR) | (true, 0) => (at, char_end(at)),
        (false, BOUNDARY_WORD_START) | (true, GRANULARITY_WORD) => word(text, at),
        (false, BOUNDARY_WORD_END) => word_end(text, at),
        (true, GRANULARITY_PARAGRAPH) => line(t, at, true),
        _ => line(t, at, false),
    }
}
/// Rectangle (repère de la fenêtre) des octets `range` : union de leurs clusters.
fn extents(t: &AccessibleText, range: std::ops::Range<usize>) -> [i32; 4] {
    let mut bounds: Option<[f32; 4]> = None;
    for c in t
        .clusters
        .iter()
        .filter(|c| c.start < range.end.max(range.start + 1) && c.end > range.start)
    {
        let r = c.rect;
        bounds = Some(match bounds {
            Some([l, top, right, b]) => [
                l.min(r.left),
                top.min(r.top),
                right.max(r.right),
                b.max(r.bottom),
            ],
            None => [r.left, r.top, r.right, r.bottom],
        });
    }
    bounds.map_or([0; 4], |[l, top, r, b]| {
        [
            l.floor() as i32,
            top.floor() as i32,
            (r - l).ceil() as i32,
            (b - top).ceil() as i32,
        ]
    })
}
/// Répond aux méthodes de Text et EditableText ; `act` applique une action au moteur.
pub(super) fn call(
    e: &AccessibleElement,
    interface: &str,
    member: &str,
    args: &mut Reader<'_>,
    out: &mut Writer<'_>,
    mut act: impl FnMut(AccessibilityAction) -> bool,
) -> bool {
    let Some(t) = &e.text else {
        return false;
    };
    let text = &*t.value;
    let select = |range: std::ops::Range<usize>| AccessibilityAction::SelectText {
        id: e.id,
        range,
        expected: Arc::clone(&t.value),
    };
    if interface == "org.a11y.atspi.EditableText" {
        let replaced = match member {
            "SetTextContents" => args.string(),
            "InsertText" => {
                let at = byte(text, args.i32().unwrap_or(0));
                let insert = args.string().unwrap_or_default();
                // Seuls les `length` premiers caractères sont insérés (négatif : tous).
                let insert: String = match usize::try_from(args.i32().unwrap_or(-1)) {
                    Ok(length) => insert.chars().take(length).collect(),
                    Err(_) => insert,
                };
                Some(format!("{}{insert}{}", &text[..at], &text[at..]))
            }
            "DeleteText" => {
                let start = byte(text, args.i32().unwrap_or(0));
                let end = byte(text, args.i32().unwrap_or(0)).max(start);
                Some(format!("{}{}", &text[..start], &text[end..]))
            }
            "CopyText" | "CutText" | "PasteText" => None,
            _ => return false,
        };
        let done =
            replaced.is_some_and(|value| act(AccessibilityAction::SetValue { id: e.id, value }));
        out.bool(done);
        return true;
    }
    match member {
        "GetText" => {
            let start = byte(text, args.i32().unwrap_or(0));
            let end = match args.i32().unwrap_or(-1) {
                -1 => text.len(),
                end => byte(text, end),
            };
            out.string(text.get(start..end.max(start)).unwrap_or(""));
        }
        "GetTextAtOffset" | "GetStringAtOffset" => {
            let at = byte(text, args.i32().unwrap_or(0));
            let unit = args.u32().unwrap_or(0);
            let (start, end) = piece(t, at, unit, member == "GetStringAtOffset");
            out.string(&text[start..end])
                .i32(offset(text, start))
                .i32(offset(text, end));
        }
        "GetTextBeforeOffset" | "GetTextAfterOffset" => {
            let at = byte(text, args.i32().unwrap_or(0));
            let unit = args.u32().unwrap_or(0);
            let (start, end) = piece(t, at, unit, false);
            let (start, end) = if member == "GetTextBeforeOffset" {
                match text[..start].char_indices().next_back().map(|(b, _)| b) {
                    Some(b) => piece(t, b, unit, false),
                    None => (0, 0),
                }
            } else if end < text.len() {
                piece(t, end, unit, false)
            } else {
                (text.len(), text.len())
            };
            out.string(&text[start..end])
                .i32(offset(text, start))
                .i32(offset(text, end));
        }
        "GetCharacterAtOffset" => {
            let c = text[byte(text, args.i32().unwrap_or(0))..].chars().next();
            out.i32(c.map_or(0, |c| c as i32));
        }
        "GetNSelections" => {
            out.i32(i32::from(t.selection.start != t.selection.end));
        }
        "GetSelection" => {
            let (a, b) = (
                t.selection.start.min(t.selection.end),
                t.selection.start.max(t.selection.end),
            );
            out.i32(offset(text, a)).i32(offset(text, b));
        }
        "SetSelection" | "AddSelection" => {
            if member == "SetSelection" {
                args.i32();
            }
            let start = byte(text, args.i32().unwrap_or(0));
            let end = byte(text, args.i32().unwrap_or(0));
            out.bool(act(select(start.min(end)..start.max(end))));
        }
        "RemoveSelection" => {
            out.bool(act(select(t.selection.end..t.selection.end)));
        }
        "SetCaretOffset" => {
            let at = byte(text, args.i32().unwrap_or(0));
            out.bool(act(select(at..at)));
        }
        "GetCharacterExtents" => {
            let at = byte(text, args.i32().unwrap_or(0));
            let [x, y, w, h] = extents(t, at..at);
            out.i32(x).i32(y).i32(w).i32(h);
        }
        "GetRangeExtents" => {
            let start = byte(text, args.i32().unwrap_or(0));
            let end = byte(text, args.i32().unwrap_or(0));
            let [x, y, w, h] = extents(t, start.min(end)..start.max(end));
            out.i32(x).i32(y).i32(w).i32(h);
        }
        "GetOffsetAtPoint" => {
            let (x, y) = (
                args.i32().unwrap_or(-1) as f32,
                args.i32().unwrap_or(-1) as f32,
            );
            let found = t.clusters.iter().find(|c| {
                x >= c.rect.left && x < c.rect.right && y >= c.rect.top && y < c.rect.bottom
            });
            out.i32(found.map_or(-1, |c| offset(text, c.start)));
        }
        "GetAttributes" | "GetAttributeRun" => {
            out.open(b'a', Some("{ss}")).close();
            out.i32(0).i32(text.chars().count() as i32);
        }
        "GetDefaultAttributes" | "GetDefaultAttributeSet" => {
            out.open(b'a', Some("{ss}")).close();
        }
        "GetAttributeValue" => {
            out.string("");
        }
        "GetBoundedRanges" => {
            out.open(b'a', Some("(iisv)")).close();
        }
        _ => return false,
    }
    true
}
