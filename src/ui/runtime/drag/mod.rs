//! Free drag and drop between panels: any element carries typed data
//! ([`Element::drag_payload`](crate::ui::element::Element::drag_payload)); dragged past
//! 4 logical units with the main button, it goes wherever the pointer goes. The elements
//! under the pointer say whether they accept it
//! ([`Element::on_drag_over`](crate::ui::element::Element::on_drag_over)) and receive it
//! when it is dropped ([`Element::on_drop`](crate::ui::element::Element::on_drop)); the
//! source follows the whole gesture
//! ([`Element::on_drag`](crate::ui::element::Element::on_drag)) and draws its preview
//! where it likes ([`preview`]). Escape, or the window losing the keyboard, cancels it.
//!
//! It is distinct from [`drag_drop`](crate::ui::drag_drop) (lines reordered within a
//! group, buttons panned), which stays as it is.
mod events;
mod payload;
mod preview;
mod session;

pub(crate) use events::DragHandler;
pub use events::{DragEvent, DragPhase};
pub use payload::Payload;
pub use preview::preview;
pub(crate) use session::Session;
