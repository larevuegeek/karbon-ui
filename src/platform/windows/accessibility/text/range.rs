//! Plage de texte COM : bornes en octets, rebasées après une édition du champ.
use super::*;
use windows::{
    Win32::{
        System::Variant::{VARIANT, VT_BOOL},
        UI::Accessibility::{
            ITextRangeProvider_Impl, TextPatternRangeEndpoint, TextUnit, UIA_TEXTATTRIBUTE_ID,
            UiaGetReservedNotSupportedValue,
        },
    },
    core::{BOOL, BSTR, Ref, implement},
};
use windows_sys::Win32::Globalization::{FIND_FROMEND, FIND_FROMSTART, FindStringOrdinal};

struct RangeState {
    value: Arc<str>,
    span: Range<usize>,
}
type CurrentRange = (Arc<Node>, Arc<TextData>, Range<usize>);
#[implement(ITextRangeProvider, Agile = false)]
pub(super) struct TextRange {
    pub(super) bridge: Arc<Bridge>,
    key: NodeId,
    state: Mutex<RangeState>,
}
pub(super) fn new_range(
    bridge: &Arc<Bridge>,
    key: NodeId,
    value: Arc<str>,
    span: Range<usize>,
) -> ITextRangeProvider {
    TextRange {
        bridge: Arc::clone(bridge),
        key,
        state: Mutex::new(RangeState { value, span }),
    }
    .into()
}
impl TextRange {
    pub(super) fn current(&self) -> Result<CurrentRange> {
        let (node, text) = data(&self.bridge, self.key)?;
        let mut state = lock(&self.state);
        if state.value != text.value {
            state.span = rebase(&state.value, &text.value, state.span.clone());
            state.value = Arc::clone(&text.value);
        }
        state.span = text.snap(state.span.start)..text.snap(state.span.end);
        Ok((node, text, state.span.clone()))
    }
    pub(super) fn update(&self, value: &Arc<str>, span: Range<usize>) {
        *lock(&self.state) = RangeState {
            value: Arc::clone(value),
            span,
        };
    }
    fn spawn(&self, value: &Arc<str>, span: Range<usize>) -> ITextRangeProvider {
        new_range(&self.bridge, self.key, Arc::clone(value), span)
    }
    /// Plage cible : uniquement l'une des nôtres, sur le même pont et le même champ.
    fn target(&self, raw: Ref<'_, ITextRangeProvider>) -> Result<Range<usize>> {
        let interface = raw.as_ref().ok_or_else(|| fail(INVALID_ARGUMENT))?;
        // Transtypage dynamique de windows-rs : échoue pour un proxy ou un objet étranger.
        let other = interface
            .cast_object_ref::<TextRange>()
            .map_err(|_| fail(INVALID_ARGUMENT))?;
        if !Arc::ptr_eq(&self.bridge, &other.bridge) || self.key != other.key {
            return Err(fail(INVALID_ARGUMENT));
        }
        Ok(other.current()?.2)
    }
}
// Repositionnement conservateur après une édition : préfixe/suffixe communs.
fn rebase(old: &str, new: &str, span: Range<usize>) -> Range<usize> {
    let mut prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let mut suffix = old[prefix..]
        .bytes()
        .rev()
        .zip(new[prefix..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    let map = |p: usize, end: bool| {
        if p < prefix {
            p
        } else if p >= old.len() - suffix {
            new.len() - suffix + p.saturating_sub(old.len() - suffix)
        } else if end {
            new.len() - suffix
        } else {
            prefix
        }
    };
    let start = map(span.start, false).min(new.len());
    let end = if span.is_empty() {
        start
    } else {
        map(span.end, true).min(new.len()).max(start)
    };
    start..end
}
fn endpoint(span: &Range<usize>, which: TextPatternRangeEndpoint) -> Result<usize> {
    match which.0 {
        0 => Ok(span.start),
        1 => Ok(span.end),
        _ => Err(fail(INVALID_ARGUMENT)),
    }
}
fn set_endpoint(
    span: Range<usize>,
    which: TextPatternRangeEndpoint,
    byte: usize,
) -> Result<Range<usize>> {
    match which.0 {
        0 => Ok(byte..span.end.max(byte)),
        1 => Ok(span.start.min(byte)..byte),
        _ => Err(fail(INVALID_ARGUMENT)),
    }
}
fn enclosing_unit(points: &[usize], byte: usize) -> Range<usize> {
    if points.len() < 2 {
        return 0..0;
    }
    let index = points
        .partition_point(|&p| p <= byte)
        .saturating_sub(1)
        .min(points.len() - 2);
    points[index]..points[index + 1]
}
fn moved_endpoint(points: &[usize], byte: usize, count: i32) -> (usize, i32) {
    if count == 0 {
        return (byte, 0);
    }
    let index = if count > 0 {
        points.partition_point(|&p| p <= byte)
    } else {
        points.partition_point(|&p| p < byte)
    };
    if count > 0 {
        let available = points.len() - index;
        let moved = (count as usize).min(available);
        (
            if moved == 0 {
                byte
            } else {
                points[index + moved - 1]
            },
            moved as i32,
        )
    } else {
        let moved = (count.unsigned_abs() as usize).min(index);
        (
            if moved == 0 {
                byte
            } else {
                points[index - moved]
            },
            -(moved as i32),
        )
    }
}
impl ITextRangeProvider_Impl for TextRange_Impl {
    fn Clone(&self) -> Result<ITextRangeProvider> {
        boundary(|| {
            let (_, text, span) = self.current()?;
            Ok(self.spawn(&text.value, span))
        })
    }
    fn Compare(&self, other: Ref<'_, ITextRangeProvider>) -> Result<BOOL> {
        boundary(|| {
            let span = self.current()?.2;
            Ok((span == self.target(other)?).into())
        })
    }
    fn CompareEndpoints(
        &self,
        which: TextPatternRangeEndpoint,
        other: Ref<'_, ITextRangeProvider>,
        other_which: TextPatternRangeEndpoint,
    ) -> Result<i32> {
        boundary(|| {
            let a = endpoint(&self.current()?.2, which)?;
            let b = endpoint(&self.target(other)?, other_which)?;
            Ok(a.cmp(&b) as i32)
        })
    }
    fn ExpandToEnclosingUnit(&self, unit: TextUnit) -> Result<()> {
        boundary(|| {
            let (_, text, span) = self.current()?;
            self.update(
                &text.value,
                enclosing_unit(&text.boundaries(unit.0)?, span.start),
            );
            Ok(())
        })
    }
    fn FindAttribute(
        &self,
        id: UIA_TEXTATTRIBUTE_ID,
        value: &VARIANT,
        _backward: BOOL,
    ) -> Result<ITextRangeProvider> {
        boundary(|| {
            let (node, text, span) = self.current()?;
            if id.0 != 40015 {
                return Err(fail(NOT_IMPLEMENTED));
            }
            // SAFETY: `vt` occupe les mêmes octets dans toutes les variantes de l'union.
            if unsafe { value.Anonymous.Anonymous.vt } != VT_BOOL {
                return Err(fail(INVALID_ARGUMENT));
            }
            // SAFETY: VT_BOOL vient d'être vérifié, `boolVal` est donc la donnée active.
            let wanted = unsafe { value.Anonymous.Anonymous.Anonymous.boolVal }.0 != 0;
            if wanted == node.read_only {
                Ok(self.spawn(&text.value, span))
            } else {
                nothing()
            }
        })
    }
    fn FindText(
        &self,
        value: &BSTR,
        backward: BOOL,
        ignore_case: BOOL,
    ) -> Result<ITextRangeProvider> {
        boundary(|| {
            let needle: &[u16] = value;
            if needle.is_empty() || needle.len() > 262_144 {
                return Err(fail(INVALID_ARGUMENT));
            }
            String::from_utf16(needle).map_err(|_| fail(INVALID_ARGUMENT))?;
            let (_, text, span) = self.current()?;
            let source: Vec<_> = text.value[span.clone()].encode_utf16().collect();
            // SAFETY: les deux tampons UTF-16 restent valides et leurs longueurs sont exactes.
            let found = unsafe {
                FindStringOrdinal(
                    if backward.as_bool() {
                        FIND_FROMEND
                    } else {
                        FIND_FROMSTART
                    },
                    source.as_ptr(),
                    source.len() as i32,
                    needle.as_ptr(),
                    needle.len() as i32,
                    ignore_case.0,
                )
            };
            if found < 0 {
                return nothing();
            }
            let found = found as usize;
            let a = String::from_utf16(&source[..found])
                .map_err(|_| fail(INVALID_ARGUMENT))?
                .len()
                + span.start;
            let b = a + String::from_utf16(&source[found..found + needle.len()])
                .map_err(|_| fail(INVALID_ARGUMENT))?
                .len();
            Ok(self.spawn(&text.value, a..b))
        })
    }
    fn GetAttributeValue(&self, id: UIA_TEXTATTRIBUTE_ID) -> Result<VARIANT> {
        boundary(|| {
            let (node, _, _) = self.current()?;
            Ok(if id.0 == 40015 {
                Variant::boolean(node.read_only)
            } else {
                // SAFETY: valeur réservée par UIA, sans argument.
                Variant::unknown(unsafe { UiaGetReservedNotSupportedValue() }?)
            }
            .into_raw())
        })
    }
    fn GetBoundingRectangles(&self) -> Result<*mut SAFEARRAY> {
        boundary(|| {
            let (node, text, span) = self.current()?;
            doubles(&if node.offscreen {
                Vec::new()
            } else {
                text.rectangles(span)
            })
        })
    }
    fn GetEnclosingElement(&self) -> Result<IRawElementProviderSimple> {
        boundary(|| {
            self.current()?;
            self.bridge.interface(self.key)
        })
    }
    fn GetText(&self, max: i32) -> Result<BSTR> {
        boundary(|| {
            if max < -1 {
                return Err(fail(INVALID_ARGUMENT));
            }
            let (_, text, span) = self.current()?;
            let value = &text.value[span];
            let mut bytes = 0;
            let mut units = 0;
            for c in value.chars() {
                if max >= 0 && units + c.len_utf16() > max as usize {
                    break;
                }
                units += c.len_utf16();
                bytes += c.len_utf8();
            }
            bstr(&value[..bytes])
        })
    }
    fn Move(&self, unit: TextUnit, count: i32) -> Result<i32> {
        boundary(|| {
            let (_, text, span) = self.current()?;
            let points = text.boundaries(unit.0)?;
            if count == 0 {
                return Ok(0);
            }
            let (next, moved) = if span.is_empty() {
                let (byte, moved) = moved_endpoint(&points, span.start, count);
                (byte..byte, moved)
            } else if points.len() < 2 {
                (span, 0)
            } else {
                let index = points
                    .partition_point(|&p| p <= span.start)
                    .saturating_sub(1)
                    .min(points.len() - 2);
                let next =
                    (index as i64 + count as i64).clamp(0, (points.len() - 2) as i64) as usize;
                if next == index {
                    (span, 0)
                } else {
                    (points[next]..points[next + 1], next as i32 - index as i32)
                }
            };
            self.update(&text.value, next);
            Ok(moved)
        })
    }
    fn MoveEndpointByUnit(
        &self,
        which: TextPatternRangeEndpoint,
        unit: TextUnit,
        count: i32,
    ) -> Result<i32> {
        boundary(|| {
            let (_, text, span) = self.current()?;
            let (byte, moved) =
                moved_endpoint(&text.boundaries(unit.0)?, endpoint(&span, which)?, count);
            self.update(&text.value, set_endpoint(span, which, byte)?);
            Ok(moved)
        })
    }
    fn MoveEndpointByRange(
        &self,
        which: TextPatternRangeEndpoint,
        other: Ref<'_, ITextRangeProvider>,
        other_which: TextPatternRangeEndpoint,
    ) -> Result<()> {
        boundary(|| {
            let (_, text, span) = self.current()?;
            let byte = endpoint(&self.target(other)?, other_which)?;
            self.update(&text.value, set_endpoint(span, which, byte)?);
            Ok(())
        })
    }
    fn Select(&self) -> Result<()> {
        boundary(|| {
            let (_, text, span) = self.current()?;
            check(self.bridge.dispatch(Command::SelectText(
                self.key,
                span,
                Arc::clone(&text.value),
            )))
        })
    }
    fn AddToSelection(&self) -> Result<()> {
        boundary(|| self.multiple_selection())
    }
    fn RemoveFromSelection(&self) -> Result<()> {
        boundary(|| self.multiple_selection())
    }
    fn ScrollIntoView(&self, top: BOOL) -> Result<()> {
        boundary(|| {
            let (_, _, span) = self.current()?;
            check(self.bridge.dispatch(Command::RevealText(
                self.key,
                if top.as_bool() { span.start } else { span.end },
            )))
        })
    }
    fn GetChildren(&self) -> Result<*mut SAFEARRAY> {
        boundary(|| {
            self.current()?;
            unknowns(&[])
        })
    }
}
impl TextRange {
    /// Sélection unique : ajouter/retirer une plage n'est jamais possible.
    fn multiple_selection(&self) -> Result<()> {
        self.current()?;
        Err(fail(INVALID_OPERATION))
    }
}
