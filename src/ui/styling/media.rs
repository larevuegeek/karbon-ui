use crate::ui::element::SizeF;
/// Viewport condition used by [`StyleState::Media`](super::StyleState::Media).
///
/// Bounds are inclusive and expressed in device-independent pixels; use `0.` and
/// `f32::INFINITY` for unbounded sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaQuery {
    /// Minimum viewport width (inclusive).
    pub min_width: f32,
    /// Maximum viewport width (inclusive).
    pub max_width: f32,
    /// Minimum viewport height (inclusive).
    pub min_height: f32,
    /// Maximum viewport height (inclusive).
    pub max_height: f32,
    /// `Some(true)` requires landscape (width > height), `Some(false)` portrait, `None` either.
    pub landscape: Option<bool>,
    /// `Some(true)` requires `prefers-reduced-motion: reduce`, `Some(false)`
    /// `no-preference`, `None` either; checked by style resolution against
    /// [`StyleEnvironment::reduced_motion`](super::StyleEnvironment::reduced_motion).
    pub reduced_motion: Option<bool>,
}
impl MediaQuery {
    /// Returns `true` when `viewport` satisfies every bound of the query.
    pub fn matches(self, viewport: SizeF) -> bool {
        viewport.width >= self.min_width
            && viewport.width <= self.max_width
            && viewport.height >= self.min_height
            && viewport.height <= self.max_height
            && self
                .landscape
                .is_none_or(|v| v == (viewport.width > viewport.height))
    }
}
/// Kind of query container declared by `container-type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerType {
    /// `inline-size`: width queries only.
    InlineSize,
    /// `size`: width and height queries.
    Size,
}
/// Query container declared on an element (`container-type`, `container-name`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Container {
    /// Axes the container answers for.
    pub kind: ContainerType,
    /// Names given by `container-name`; empty for an anonymous container.
    pub names: &'static [&'static str],
}
/// Condition on the content box of the nearest matching ancestor container, used by
/// [`StyleState::Container`](super::StyleState::Container) (`@container`).
///
/// Bounds are inclusive, in device-independent pixels; use `0.` and `f32::INFINITY`
/// for unbounded sides. A query naming no container matches the nearest container;
/// without a matching container the condition is false.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerQuery {
    /// Name of the container to query (`@container sidebar (…)`), if any.
    pub name: Option<&'static str>,
    /// Minimum width (inclusive).
    pub min_width: f32,
    /// Maximum width (inclusive).
    pub max_width: f32,
    /// Minimum height (inclusive); requires a `size` container when bounded.
    pub min_height: f32,
    /// Maximum height (inclusive); requires a `size` container when bounded.
    pub max_height: f32,
}
impl ContainerQuery {
    /// Whether the query needs the container height.
    pub fn uses_height(self) -> bool {
        self.min_height > 0. || self.max_height.is_finite()
    }
    /// Whether `container` can answer this query (name and axes).
    pub fn accepts(self, container: Container) -> bool {
        self.name.is_none_or(|n| container.names.contains(&n))
            && (container.kind == ContainerType::Size || !self.uses_height())
    }
    /// Returns `true` when `size` (the container's content box) satisfies the bounds.
    pub fn matches(self, size: SizeF) -> bool {
        size.width >= self.min_width
            && size.width <= self.max_width
            && size.height >= self.min_height
            && size.height <= self.max_height
    }
}
