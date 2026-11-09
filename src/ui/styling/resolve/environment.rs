//! État d'interaction et de fenêtre lu par la résolution, et ce qu'une passe a changé.
use crate::ui::{element::ElementId, scene::PointF};

/// Text, background and border colours of an element, `None` when not set.
pub type ElementColors = [Option<crate::ui::theme::Color>; 3];
/// Declared width and height of an element.
pub type ElementSizes = [crate::ui::element::Length; 2];
/// Width and height shown by an animation, `None` for a dimension it leaves alone.
pub type AnimatedSizes = [Option<crate::ui::element::Length>; 2];
/// Interaction and viewport state used to match style conditions.
#[derive(Clone, Copy, Default)]
pub struct StyleEnvironment<'a> {
    /// Viewport size, for media queries and viewport units.
    pub viewport: crate::ui::element::SizeF,
    /// Retain the document root font when styling a newly materialized subtree.
    pub root_font: Option<f32>,
    /// Pointer position in window coordinates, for `:hover`.
    pub pointer: Option<PointF>,
    /// Elements under the pointer, as a browser hit-tests them: the topmost painted
    /// element at [`pointer`](Self::pointer) and its ancestors. When set, `:hover`
    /// matches only these elements, so an element covered by another one (overlay,
    /// `position: fixed` panel, open menu) is not hovered through it. `None` matches
    /// every element whose box contains the pointer.
    pub hovered: Option<&'a [ElementId]>,
    /// Pressed element, for `:active`.
    pub pressed: Option<ElementId>,
    /// Focused element, for `:focus` and `:focus-within`.
    pub focused: Option<ElementId>,
    /// Whether focus came from the keyboard, for `:focus-visible`.
    pub focus_visible: bool,
    /// Reports whether a text input is empty, overriding its declared value
    /// (for `:placeholder-shown`).
    pub input_empty: Option<&'a dyn Fn(ElementId) -> Option<bool>>,
    /// Maps a window point into an element's local space (e.g. under transforms);
    /// `None` from the callback means the point is not over the element.
    pub project: Option<&'a dyn Fn(ElementId, PointF) -> Option<PointF>>,
    /// Interpolates a colour of an element whose transition animates colours: receives
    /// the element, the property (`0` text, `1` background, `2` border), the new colour and
    /// the transition, and returns the colour to show now.
    pub transition_color: Option<
        &'a dyn Fn(
            ElementId,
            u8,
            crate::ui::theme::Color,
            crate::ui::animation::Transition,
        ) -> crate::ui::theme::Color,
    >,
    /// Interpolates a pixel dimension of an element whose transition animates sizes:
    /// receives the element, the property (`0` width, `1` height), the new length in
    /// pixels and the transition, and returns the length to lay out now.
    pub transition_length:
        Option<&'a dyn Fn(ElementId, u8, f32, crate::ui::animation::Transition) -> f32>,
    /// Colours of an element whose `@keyframes` animation animates colours: receives the
    /// element, its animation and its computed text, background and border colours, and
    /// returns the colours to show now.
    pub animate_colors: Option<
        &'a dyn Fn(ElementId, crate::ui::animation::Animation, ElementColors) -> ElementColors,
    >,
    /// Width and height of an element whose `@keyframes` animation animates sizes:
    /// receives the element, its animation and its declared width and height, and
    /// returns the lengths to lay out now (`None` for a dimension left unanimated).
    pub animate_sizes: Option<
        &'a dyn Fn(ElementId, crate::ui::animation::Animation, ElementSizes) -> AnimatedSizes,
    >,
    /// Physical pixels per logical pixel; border widths snap to whole physical pixels.
    /// `0` (the default) or `1` leaves them unchanged.
    pub pixel_scale: f32,
    /// The user asked to reduce animations (`prefers-reduced-motion: reduce`).
    pub reduced_motion: bool,
    /// Active colour scheme is dark (`prefers-color-scheme: dark`).
    pub dark: bool,
    /// High contrast is requested (`prefers-contrast: more`).
    pub high_contrast: bool,
    /// Resolve only these elements and what depends on them, keeping the computed styles
    /// of the rest (`None`: the whole tree).
    pub partial: Option<&'a super::Partial>,
}
impl StyleEnvironment<'_> {
    pub(super) fn pixel_scale(&self) -> f32 {
        if self.pixel_scale.is_finite() && self.pixel_scale > 0. {
            self.pixel_scale
        } else {
            1.
        }
    }
}
/// What a [`resolve`](super::resolve) pass changed.
#[derive(Default)]
pub struct StyleChanges {
    /// Metrics changed: a layout pass is required.
    pub layout: bool,
    /// Appearance changed: a repaint is required.
    pub paint: bool,
}
