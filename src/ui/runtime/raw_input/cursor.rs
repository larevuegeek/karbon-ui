//! Le curseur choisi par l'application pour un élément (`Element::cursor_shape`), en plus
//! des curseurs du CSS (`Cursor`, qui ne change pas) : un curseur caché, ou une image
//! dessinée par l'application (pipette, main fermée qui tire la toile).
//!
//! Le curseur d'un élément vaut au-dessus de lui et de ses descendants ; pendant un geste
//! pris par `on_pointer`, celui de l'élément qui tient le geste vaut partout, même hors de
//! lui. Les contrôles du runtime gardent la main : un menu d'édition ou une liste ouverte,
//! un séparateur qu'on tire, une barre de défilement, un champ de texte.
use super::super::hit_test::fixed_result;
use super::super::*;
use super::pointer::target;
use std::sync::Arc;

/// A picture used as the mouse cursor: its pixels and the point of the picture that
/// aims (the hotspot), in pixels from its top left corner. The picture is shown at its
/// size in pixels of the screen. Two cursor images are equal when they share the same
/// picture (the same `Arc`) and aim at the same point.
#[derive(Clone)]
pub struct CursorImage {
    /// The picture, premultiplied BGRA like every [`Image`](crate::ui::scene::Image).
    pub image: Arc<crate::ui::scene::Image>,
    /// The aiming point, in pixels from the top left corner of the picture.
    pub hotspot: (u32, u32),
}

impl PartialEq for CursorImage {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.image, &other.image) && self.hotspot == other.hotspot
    }
}

impl std::fmt::Debug for CursorImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CursorImage")
            .field("size", &(self.image.width, self.image.height))
            .field("hotspot", &self.hotspot)
            .finish()
    }
}

impl CursorImage {
    /// The cursor `image` aiming at `hotspot` (kept inside the picture).
    pub fn new(image: Arc<crate::ui::scene::Image>, hotspot: (u32, u32)) -> Self {
        let hotspot = (
            hotspot.0.min(image.width.saturating_sub(1)),
            hotspot.1.min(image.height.saturating_sub(1)),
        );
        Self { image, hotspot }
    }
}

/// The cursor an element asks for with
/// [`Element::cursor_shape`](crate::ui::element::Element::cursor_shape), beyond the
/// named cursors of CSS.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CursorShape {
    /// One of the named cursors of the system.
    Named(Cursor),
    /// No cursor at all (a view that draws its own aim).
    Hidden,
    /// A picture drawn by the application.
    Image(CursorImage),
}

/// Le curseur choisi par `start` ou par ses ancêtres, le plus proche d'abord.
fn shape_of(node: &Element, start: ElementId) -> Option<Option<CursorShape>> {
    if node.id == start {
        let own = node.listeners.as_ref().and_then(|l| l.cursor.clone());
        return Some(own);
    }
    let inner = node.children.iter().find_map(|c| shape_of(c, start))?;
    Some(inner.or_else(|| node.listeners.as_ref().and_then(|l| l.cursor.clone())))
}

impl UiRuntime {
    /// The cursor chosen by the application for the element under the pointer (or for
    /// the element that holds the pointer gesture in progress), if any: the backend shows
    /// it instead of [`cursor`](Self::cursor). `None` leaves [`cursor`](Self::cursor) in
    /// charge, as when no element asks for a shape.
    pub fn cursor_shape(&self) -> Option<CursorShape> {
        let root = self.root.as_ref()?;
        if let Some(held) = self.raw_pointer.target {
            return shape_of(root, held).flatten();
        }
        if self.controls_own_the_cursor() {
            return None;
        }
        let scope = layers::active(root);
        let p = self.interaction.pointer?;
        let found = fixed_result(scope, p, &self.motions, true, target)
            .or_else(|| target(scope, p, &self.motions, true))?;
        shape_of(scope, found).flatten()
    }

    /// Un contrôle du runtime décide du curseur ici : menu d'édition, liste ouverte,
    /// séparateur tiré ou survolé, barre de défilement, champ de texte.
    fn controls_own_the_cursor(&self) -> bool {
        use super::super::Hit;
        let components = &self.components;
        components.edit_menu.cursor().is_some()
            || components.selects.cursor().is_some()
            || components.splits.drag.is_some()
            || matches!(
                self.interaction.hovered,
                Some(Hit::Divider(..) | Hit::ScrollThumb(_) | Hit::ScrollTrack(_) | Hit::Input(_))
            )
    }
}
