//! A surface shared on the graphics card: another renderer (a 3D engine) draws into a
//! texture that the window shows in place of an element, without the image ever passing
//! through the memory of the computer and without repainting the interface for each
//! image.
//!
//! - [`SharedSurface`] makes the texture on a given graphics card, gives its handle to
//!   the producer, and shows what the producer drew ([`SharedSurface::present`]).
//! - An element paints [`SharedSurface::placeholder`] where the surface must appear: the
//!   window then cuts a hole in the interface at that place and keeps the surface under
//!   it, at the size and place of the element. Menus, tooltips and anything painted
//!   after the element stay above the surface.
//!
//! Windows only (Direct3D 11 and DirectComposition); elsewhere [`SharedSurface::new`]
//! answers [`Error::Unsupported`](crate::Error::Unsupported) and the application keeps
//! copying images ([`LiveImage`](crate::ui::scene::LiveImage)). See `docs/shared-surface.md`.
mod registry;
mod surface;
#[cfg(not(windows))]
mod unavailable;

#[cfg_attr(not(windows), allow(unused_imports))]
pub(crate) use registry::surface_of;
#[cfg_attr(not(windows), allow(unused_imports))]
pub(crate) use surface::Shared;
pub use surface::{AdapterLuid, SharedSurface, SurfacePlacement};
