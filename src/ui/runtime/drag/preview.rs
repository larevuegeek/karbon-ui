//! L'aperçu d'un glisser : ce que la source dessine sous le pointeur, au-dessus de tout,
//! sans prendre le pointeur (les cibles dessous le reçoivent).
use crate::ui::{
    element::{Element, Length, Position},
    template::Node,
};

/// The preview of a drag: `content` placed beside the pointer at `(x, y)` (window
/// coordinates, from [`DragEvent`](super::DragEvent)), above everything in the window,
/// letting the pointer through to the targets below. Put it last in the view while the
/// source drags.
///
/// ```
/// use karbon_ui::ui::{drag, prelude::*};
///
/// let chip = text("chip", "Brique rouge");
/// let layer = drag::preview(chip, (120., 80.));
/// # let _ = layer;
/// ```
pub fn preview(content: Element, (x, y): (f32, f32)) -> Element {
    Node::element(content)
        .position(Position::Fixed)
        .offset(0, Length::Px(y + 14.))
        .offset(3, Length::Px(x + 14.))
        .z_index(i32::MAX)
        .pointer_events(false)
        .build()
}
