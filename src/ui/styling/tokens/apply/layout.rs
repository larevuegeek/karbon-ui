//! Boîte et flex : marges internes et externes, dimensions, flex et espacements.
use super::super::{border::sides, reader::Reader, token::Token};
use super::{DynamicProperty, side};
use crate::ui::{element::Insets, styling::Properties};

/// Applique une propriété de boîte ou de flex.
pub(super) fn apply(property: DynamicProperty, r: &mut Reader, out: &mut Properties) {
    use DynamicProperty as P;
    match property {
        P::Padding => {
            let mut values = Vec::new();
            while !r.done() {
                match r.px() {
                    Some(px) => values.push(px.max(0.)),
                    None => return,
                }
            }
            if let Some([t, rr, b, l]) = sides(&values) {
                out.padding = Some(Insets::edges(t, rr, b, l));
                out.layout.padding_edges = [None; 4];
                out.layout.spacing.padding = [None; 4];
            }
        }
        P::PaddingTop | P::PaddingRight | P::PaddingBottom | P::PaddingLeft => {
            if let Some(px) = r.px().filter(|_| r.done()) {
                out.layout.padding_edges[side(property)] = Some(px.max(0.));
                out.layout.spacing.padding[side(property)] = None;
            }
        }
        P::Margin => {
            let mut values = Vec::new();
            while !r.done() {
                if r.peek() == Some(Token::Ident("auto")) {
                    r.at += 1;
                    values.push(None);
                    continue;
                }
                match r.px() {
                    Some(px) => values.push(Some(px)),
                    None => return,
                }
            }
            if let Some(m) = sides(&values) {
                out.layout.margin = Some(m);
                out.layout.margin_edges = [None; 4];
                out.layout.spacing.margin = [None; 4];
            }
        }
        P::MarginTop | P::MarginRight | P::MarginBottom | P::MarginLeft => {
            let value = if r.peek() == Some(Token::Ident("auto")) {
                r.at += 1;
                Some(None)
            } else {
                r.px().map(Some)
            };
            if let Some(v) = value.filter(|_| r.done()) {
                out.layout.margin_edges[side(property)] = Some(v);
                out.layout.spacing.margin[side(property)] = None;
            }
        }
        P::Width | P::Height | P::FlexBasis => {
            if let Some(len) = r.length().filter(|_| r.done()) {
                match property {
                    P::Width => out.layout.width = Some(len),
                    P::Height => out.layout.height = Some(len),
                    _ => out.layout.flex_basis = Some(len),
                }
            }
        }
        P::MinWidth | P::MinHeight | P::MaxWidth | P::MaxHeight => {
            let value = match r.peek() {
                Some(Token::Ident("none")) if matches!(property, P::MaxWidth | P::MaxHeight) => {
                    r.at += 1;
                    Some(f32::INFINITY)
                }
                Some(Token::Ident("auto")) => {
                    r.at += 1;
                    Some(0.)
                }
                _ => r.px(),
            };
            if let Some(v) = value.filter(|_| r.done()) {
                match property {
                    P::MinWidth => out.layout.min_width = Some(v.max(0.)),
                    P::MinHeight => out.layout.min_height = Some(v.max(0.)),
                    P::MaxWidth => out.layout.max_width = Some(v.max(0.)),
                    _ => out.layout.max_height = Some(v.max(0.)),
                }
            }
        }
        P::FlexGrow | P::FlexShrink => {
            if let Some(n) = r.number().filter(|n| *n >= 0. && r.done()) {
                if property == P::FlexGrow {
                    out.layout.flex_grow = Some(n);
                } else {
                    out.layout.flex_shrink = Some(n);
                }
            }
        }
        P::Gap => {
            let row = r.px();
            let column = if r.done() { row } else { r.px() };
            if let (Some(row), Some(column)) = (row, column)
                && r.done()
            {
                out.layout.spacing.row_gap = None;
                out.layout.spacing.column_gap = None;
                if row == column {
                    out.layout.gap = Some(row.max(0.));
                } else {
                    out.layout.row_gap = Some(row.max(0.));
                    out.layout.column_gap = Some(column.max(0.));
                }
            }
        }
        P::RowGap | P::ColumnGap => {
            if let Some(px) = r.px().filter(|_| r.done()) {
                if property == P::RowGap {
                    out.layout.row_gap = Some(px.max(0.));
                    out.layout.spacing.row_gap = None;
                } else {
                    out.layout.column_gap = Some(px.max(0.));
                    out.layout.spacing.column_gap = None;
                }
            }
        }
        _ => {}
    }
}
