//! Espacements donnés en expression (`em`, `rem`, `vw`, `vh`, `calc()`…) : gardés tels
//! quels par les règles, puis évalués par la cascade avec le contexte d'unités de
//! l'élément et écrits en px, comme si des px avaient été déclarés.
use super::{CssVariables, LayoutProperties};
use crate::ui::layout::{ComputedLength, UnitContext};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Padding, margins and gaps of a rule given as expressions (`padding: 1em`,
/// `gap: calc(1rem + 2px)`), evaluated while styles resolve against the element's
/// font size, the root font size and the viewport.
///
/// An expression slot set by a rule wins over the pixel value of the same edge in that
/// rule; edge arrays are ordered top, right, bottom, left. Slot indices used by
/// [`set`](Self::set): `0..4` padding edges, `4..8` margin edges, `8` row gap, `9`
/// column gap.
pub struct SpacingExpressions {
    /// Padding edges.
    pub padding: [Option<ComputedLength>; 4],
    /// Margin edges (may evaluate to a negative value).
    pub margin: [Option<ComputedLength>; 4],
    /// Gap between rows.
    pub row_gap: Option<ComputedLength>,
    /// Gap between columns.
    pub column_gap: Option<ComputedLength>,
}
impl SpacingExpressions {
    /// Sets slot `index` (see the type documentation); other indices are ignored.
    pub fn set(&mut self, index: usize, value: ComputedLength) {
        match index {
            0..4 => self.padding[index] = Some(value),
            4..8 => self.margin[index - 4] = Some(value),
            8 => self.row_gap = Some(value),
            9 => self.column_gap = Some(value),
            _ => {}
        }
    }
    /// Efface l'emplacement `index` : un espacement en px déclaré après l'expression.
    pub(crate) fn clear(&mut self, index: usize) {
        match index {
            0..4 => self.padding[index] = None,
            4..8 => self.margin[index - 4] = None,
            8 => self.row_gap = None,
            9 => self.column_gap = None,
            _ => {}
        }
    }
    /// Returns `true` when no slot is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    /// Returns `true` when every set expression is valid.
    pub fn valid(&self) -> bool {
        self.slots().all(|v| v.valid())
    }
    fn slots(&self) -> impl Iterator<Item = ComputedLength> + '_ {
        self.padding
            .iter()
            .chain(&self.margin)
            .chain([&self.row_gap, &self.column_gap])
            .flatten()
            .copied()
    }
}
impl LayoutProperties {
    /// Superpose les expressions d'espacement de `self` sur celles de `b` : un côté que
    /// `self` déclare, en px ou en expression, écarte l'expression de `b`.
    pub(super) fn spacing_over(&self, b: &Self) -> SpacingExpressions {
        let own = &self.spacing;
        let base = &b.spacing;
        SpacingExpressions {
            padding: std::array::from_fn(|i| {
                own.padding[i].or(if self.padding_edges[i].is_some() {
                    None
                } else {
                    base.padding[i]
                })
            }),
            margin: std::array::from_fn(|i| {
                own.margin[i].or(if self.margin.is_some() || self.margin_edges[i].is_some() {
                    None
                } else {
                    base.margin[i]
                })
            }),
            row_gap: own
                .row_gap
                .or(if self.gap.is_some() || self.row_gap.is_some() {
                    None
                } else {
                    base.row_gap
                }),
            column_gap: own
                .column_gap
                .or(if self.gap.is_some() || self.column_gap.is_some() {
                    None
                } else {
                    base.column_gap
                }),
        }
    }
    /// Évalue les expressions d'espacement avec le contexte d'unités de l'élément et
    /// les écrit dans les côtés en px (padding et gaps bornés à 0, comme en CSS) ; une
    /// valeur non finie est une erreur.
    pub(crate) fn resolve_spacing(
        &mut self,
        context: UnitContext,
        variables: Option<&CssVariables>,
    ) -> crate::Result<()> {
        if self.spacing.is_empty() {
            return Ok(());
        }
        let spacing = std::mem::take(&mut self.spacing);
        let eval = |v: ComputedLength| {
            let n = v.evaluate(context, 0., variables);
            if n.is_finite() {
                Ok(n)
            } else {
                Err(crate::Error::invalid_input(
                    "spacing expression evaluates to a non-finite value",
                ))
            }
        };
        for i in 0..4 {
            if let Some(v) = spacing.padding[i] {
                self.padding_edges[i] = Some(eval(v)?.max(0.));
            }
            if let Some(v) = spacing.margin[i] {
                self.margin_edges[i] = Some(Some(eval(v)?));
            }
        }
        if let Some(v) = spacing.row_gap {
            self.row_gap = Some(eval(v)?.max(0.));
        }
        if let Some(v) = spacing.column_gap {
            self.column_gap = Some(eval(v)?.max(0.));
        }
        Ok(())
    }
}
