//! Layout in logical units: rows, columns, grids, flex, split panes, scrolling and virtual lists.
//!
//! Layout runs in two passes over an [`Element`] tree: [`measure`] computes natural sizes, then
//! [`arrange`] assigns final rectangles. [`validate`] rejects invalid trees beforehand. The
//! runtime drives these passes; applications normally only build elements.
mod arrange;
mod position;
pub(crate) use position::{fixed, hidden, out_of_flow};
mod sticky;
pub(crate) use sticky::sticky;
pub(crate) mod containment;
mod lazy;
pub(crate) use lazy::unsettle;
mod measure;
mod memo;
pub(crate) use memo::Key as MeasureKey;
pub mod scroll;
mod validation;
pub mod virtual_list;
pub use arrange::arrange;
pub use measure::measure;
pub(crate) use split::resizable_divider;
pub use split::split_extent;
pub(crate) use validation::duplicate_id;
pub use validation::{MAX_ELEMENTS, validate};

pub mod expression;
mod flex;
mod grid;
mod tracks;
pub use expression::{ComputedLength, Expression, UnitContext};
pub(crate) use tracks::LayoutPass;
pub use tracks::{GridTrack, TrackBreadth};
mod engine;
mod layers;
mod linear;
mod scrolling;
mod split;
pub use engine::*;
