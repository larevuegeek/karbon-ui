//! Grille de vignettes virtualisée : des cases de même taille rangées par lignes d'une
//! [`VirtualList`], seules les lignes proches de ce qu'on voit construites.
use crate::ui::{element::*, virtual_list::VirtualList};
use std::rc::Rc;

/// A grid of `count` cells of equal width, `columns` per row, that only builds the rows
/// near the viewport: a [`VirtualList`] whose rows each hold up to `columns` cells.
///
/// Attach it with [`Element::virtual_grid`]. Cells take an equal share of the row; the
/// last row is padded with empty space so that its cells keep that width. Pick the
/// number of columns from the width the grid gets with [`VirtualGrid::columns_for`].
pub struct VirtualGrid {
    count: usize,
    columns: usize,
    row_height: f32,
    gap: f32,
    build: Rc<dyn Fn(usize) -> Element>,
    reveal: Option<usize>,
}

impl VirtualGrid {
    /// A grid of `count` cells, `columns` per row (at least one), rows `row_height`
    /// logical pixels high; `build(index)` creates cell `index`.
    ///
    /// `build` must be deterministic and side-effect free, with stable and unique IDs.
    pub fn new(
        count: usize,
        columns: usize,
        row_height: f32,
        build: impl Fn(usize) -> Element + 'static,
    ) -> Self {
        Self {
            count,
            columns: columns.max(1),
            row_height,
            gap: 0.,
            build: Rc::new(build),
            reveal: None,
        }
    }

    /// Space between two cells of a row, in logical pixels (rows are spaced with the
    /// gap of the list element, [`Element::gap`]).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.);
        self
    }

    /// Scroll, at the next layout of this view, for the row of cell `index` to be fully
    /// visible (see [`VirtualList::reveal`]).
    pub fn reveal(mut self, index: usize) -> Self {
        self.reveal = Some(index);
        self
    }

    /// How many cells at least `tile` wide fit across `width`, `gap` apart: at least one.
    pub fn columns_for(width: f32, tile: f32, gap: f32) -> usize {
        if !(width.is_finite() && tile.is_finite()) || tile <= 0. {
            return 1;
        }
        (((width + gap) / (tile + gap)).floor() as usize).max(1)
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.count.div_ceil(self.columns)
    }

    /// The row of cell `index`.
    pub fn row_of(&self, index: usize) -> usize {
        index / self.columns
    }

    /// The virtual list of the rows; the IDs of the rows are taken from `id`.
    pub fn into_list(self, id: u64) -> VirtualList {
        let Self {
            count,
            columns,
            row_height,
            gap,
            build,
            reveal,
        } = self;
        let rows = count.div_ceil(columns);
        let scope = ElementId(id).child("virtual-grid");
        let list = VirtualList::new(rows, row_height, move |row| {
            let first = row * columns;
            let mut cells: Vec<Element> = (first..(first + columns).min(count))
                .map(|index| build(index).width(Length::Fill(1.)))
                .collect();
            for pad in cells.len()..columns {
                let filler = scope.child("pad").item((row * columns + pad) as u64);
                cells.push(Element::column(filler.0, Vec::new()).width(Length::Fill(1.)));
            }
            Element::row(scope.item(row as u64).0, cells)
                .gap(gap)
                .height(Length::Px(row_height))
        });
        match reveal {
            Some(index) => list.reveal(index / columns),
            None => list,
        }
    }
}

impl Element {
    /// Virtualised grid of cells of equal width; see [`VirtualGrid`].
    pub fn virtual_grid(id: u64, grid: VirtualGrid) -> Self {
        Element::virtual_list(id, grid.into_list(id))
    }
}
