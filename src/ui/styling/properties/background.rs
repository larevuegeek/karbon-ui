//! Couches d'images de fond (`background-image`) et leur placement.
/// Number of image layers kept by [`BackgroundImages`].
pub const BACKGROUND_LAYERS: usize = 2;
/// Length of a [`BackgroundSize`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BackgroundLength {
    /// Derived from the image ratio and the other dimension.
    #[default]
    Auto,
    /// Logical pixels.
    Px(f32),
    /// Percentage of the background positioning area (`50.0` is half).
    Percent(f32),
}
/// Size of a background image layer (CSS `background-size`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BackgroundSize {
    /// Natural size; an SVG without intrinsic size fills the area like
    /// [`Contain`](Self::Contain).
    #[default]
    Auto,
    /// Largest size that fits inside the area, keeping the ratio.
    Contain,
    /// Smallest size that covers the area, keeping the ratio.
    Cover,
    /// Explicit width and height.
    Explicit(BackgroundLength, BackgroundLength),
}
/// Position of a background image layer (CSS `background-position`): on each axis, the
/// fraction of the free space (`0.0` start, `1.0` end) plus an offset in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BackgroundPosition {
    /// Horizontal fraction and offset.
    pub x: (f32, f32),
    /// Vertical fraction and offset.
    pub y: (f32, f32),
}
/// Image layers of an element (first layer on top) and their placement; each list
/// applies by layer index.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BackgroundImages {
    /// Layers; `None` entries are `none`.
    pub layers: Option<[Option<&'static crate::ui::rendering::svg::SvgImage>; BACKGROUND_LAYERS]>,
    /// Position of each layer (top left by default).
    pub position: Option<[BackgroundPosition; BACKGROUND_LAYERS]>,
    /// Size of each layer (`auto` by default).
    pub size: Option<[BackgroundSize; BACKGROUND_LAYERS]>,
    /// Whether each layer is tiled (`repeat`, the default) or drawn once (`no-repeat`).
    pub repeat: Option<[bool; BACKGROUND_LAYERS]>,
}
impl BackgroundImages {
    /// Layers `self` over `base`: each list set on `self` wins.
    pub fn over(self, base: Self) -> Self {
        Self {
            layers: self.layers.or(base.layers),
            position: self.position.or(base.position),
            size: self.size.or(base.size),
            repeat: self.repeat.or(base.repeat),
        }
    }
    /// `true` when no layer draws an image.
    pub fn is_empty(&self) -> bool {
        !self.layers.is_some_and(|l| l.iter().any(Option::is_some))
    }
    pub(super) fn valid(&self) -> bool {
        let finite = |n: f32| n.is_finite();
        let length = |l: BackgroundLength| match l {
            BackgroundLength::Auto => true,
            BackgroundLength::Px(n) | BackgroundLength::Percent(n) => n.is_finite() && n >= 0.,
        };
        self.position.is_none_or(|p| {
            p.iter()
                .all(|p| [p.x.0, p.x.1, p.y.0, p.y.1].into_iter().all(finite))
        }) && self.size.is_none_or(|s| {
            s.iter().all(|s| match s {
                BackgroundSize::Explicit(w, h) => length(*w) && length(*h),
                _ => true,
            })
        }) && self
            .layers
            .is_none_or(|l| l.iter().flatten().all(|i| i.validate().is_ok()))
    }
}
