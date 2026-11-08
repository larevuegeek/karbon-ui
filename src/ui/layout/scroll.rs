//! Scroll geometry and translation, without re-measuring text on every step.
use crate::ui::{element::*, layout, scene::RectF};

/// Thickness, in logical pixels, reserved for a scroll bar track.
pub const BAR_SIZE: f32 = 12.;
/// Thickness of a thin scroll bar gutter (`scrollbar-width: thin`).
pub const THIN_BAR_SIZE: f32 = 10.;
/// Space between the track edges and the thumb.
const THUMB_INSET: f32 = 3.;
const MIN_THUMB: f32 = 24.;

/// Arranged geometry of a scroll container, as returned by
/// [`Element::scroll_metrics`](crate::ui::element::Element::scroll_metrics).
#[derive(Debug, Clone, Copy)]
pub struct ScrollMetrics {
    /// Scrolling axis.
    pub axis: Axis,
    /// Visible area of the content (the container minus the scroll bar track).
    pub viewport: RectF,
    /// Total content length along the axis.
    pub extent: f32,
    /// Current scroll offset, in `0.0..=max_offset()`.
    pub offset: f32,
    /// Scroll bar track rectangle.
    pub track: RectF,
    /// Scroll bar thumb rectangle.
    pub thumb: RectF,
    /// Thickness of the track (0 without a visible bar).
    pub thickness: f32,
}
impl ScrollMetrics {
    /// Viewport length along the scrolling axis.
    pub fn viewport_size(self) -> f32 {
        axis_size(self.viewport, self.axis)
    }
    /// Largest valid offset: content extent minus viewport size, never negative.
    pub fn max_offset(self) -> f32 {
        (self.extent - self.viewport_size()).max(0.)
    }
    pub(crate) fn update_thumb(&mut self) {
        let track_size = axis_size(self.track, self.axis);
        let length = if self.extent > 0. {
            (track_size * self.viewport_size() / self.extent)
                .max(MIN_THUMB)
                .min(track_size)
        } else {
            track_size
        };
        let travel = (track_size - length).max(0.);
        let start = if self.max_offset() > 0. {
            travel * self.offset / self.max_offset()
        } else {
            0.
        };
        let inset = THUMB_INSET.min(self.thickness / 2.);
        let thumb = (self.thickness - 2. * inset).max(0.);
        self.thumb = match self.axis {
            Axis::Vertical => RectF::new(
                self.track.left + inset,
                self.track.top + start,
                thumb,
                length,
            ),
            Axis::Horizontal => RectF::new(
                self.track.left + start,
                self.track.top + inset,
                length,
                thumb,
            ),
        };
    }
    pub(crate) fn drag_offset(self, coordinate: f32, grab: f32) -> f32 {
        let travel = axis_size(self.track, self.axis) - axis_size(self.thumb, self.axis);
        if travel <= 0. {
            self.offset
        } else {
            (coordinate - axis_start(self.track, self.axis) - grab) / travel * self.max_offset()
        }
    }
}
pub(crate) fn axis_size(rect: RectF, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => layout::width(rect),
        Axis::Vertical => layout::height(rect),
    }
}
pub(crate) fn axis_start(rect: RectF, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => rect.left,
        Axis::Vertical => rect.top,
    }
}
pub(crate) fn viewport(area: RectF, axis: Axis, bar: f32) -> RectF {
    match axis {
        Axis::Vertical => RectF::new(
            area.left,
            area.top,
            (layout::width(area) - bar).max(0.),
            layout::height(area),
        ),
        Axis::Horizontal => RectF::new(
            area.left,
            area.top,
            layout::width(area),
            (layout::height(area) - bar).max(0.),
        ),
    }
}
/// Géométrie d'un défilement : `bar` est l'épaisseur de la gouttière réservée.
pub(crate) fn metrics(
    area: RectF,
    axis: Axis,
    extent: f32,
    offset: f32,
    bar: f32,
) -> ScrollMetrics {
    let view = viewport(area, axis, bar);
    let track = match axis {
        Axis::Vertical => RectF::new(
            view.right,
            area.top,
            layout::width(area).min(bar),
            layout::height(area),
        ),
        Axis::Horizontal => RectF::new(
            area.left,
            view.bottom,
            layout::width(area),
            layout::height(area).min(bar),
        ),
    };
    let mut result = ScrollMetrics {
        axis,
        viewport: view,
        extent,
        offset: 0.,
        track,
        thumb: track,
        thickness: axis_size(
            track,
            match axis {
                Axis::Vertical => Axis::Horizontal,
                Axis::Horizontal => Axis::Vertical,
            },
        ),
    };
    result.offset = offset.clamp(0., result.max_offset());
    result.update_thumb();
    result
}
fn translated(r: RectF, dx: f32, dy: f32) -> RectF {
    RectF {
        left: r.left + dx,
        top: r.top + dy,
        right: r.right + dx,
        bottom: r.bottom + dy,
    }
}
// Recalculer le clip depuis les bounds : un élément précédemment invisible peut réapparaître.
pub(crate) fn translate(node: &mut Element, dx: f32, dy: f32, clip: RectF) {
    if layout::fixed(node) {
        return;
    }
    node.rect = translated(node.rect, dx, dy);
    node.clip = layout::node_clip(node, node.rect, clip);
    // Sous-arbre mémorisé : son dernier placement suit (voir `layout/lazy.rs`).
    if let Some(a) = node
        .layout_cache
        .as_deref_mut()
        .and_then(|m| m.arranged.as_mut())
    {
        a.bounds = translated(a.bounds, dx, dy);
        a.parent = translated(a.parent, dx, dy);
        a.clip = clip;
    }
    node.divider = node.divider.map(|r| translated(r, dx, dy));
    if let Some(s) = &mut node.scroll {
        s.viewport = translated(s.viewport, dx, dy);
        s.track = translated(s.track, dx, dy);
        s.thumb = translated(s.thumb, dx, dy);
    }
    let child_clip = if let Some(s) = node.scroll {
        layout::intersect(s.viewport, node.clip)
    } else {
        layout::child_clip(node, layout::content_rect(node))
    };
    for child in &mut node.children {
        // Comme la disposition complète : un enfant hors flux (`absolute`) est rogné à la
        // boîte de son parent, `padding` compris, pas à sa boîte de contenu.
        let clip = if layout::position::out_of_flow(child) {
            node.clip
        } else {
            child_clip
        };
        translate(child, dx, dy, clip);
    }
}
/// Renvoie le déplacement réellement consommé. Les enfants conservent leur mesure.
pub(crate) fn set_offset(node: &mut Element, requested: f32) -> f32 {
    let Some(mut s) = node.scroll else {
        return 0.;
    };
    if !requested.is_finite() {
        return 0.;
    }
    let next = requested.clamp(0., s.max_offset());
    let delta = next - s.offset;
    if delta == 0. {
        return 0.;
    }
    s.offset = next;
    s.update_thumb();
    node.scroll_offset = next;
    node.scroll = Some(s);
    let clip = layout::intersect(s.viewport, node.clip);
    for child in &mut node.children {
        translate(
            child,
            if s.axis == Axis::Horizontal {
                -delta
            } else {
                0.
            },
            if s.axis == Axis::Vertical { -delta } else { 0. },
            clip,
        );
    }
    delta
}
