/// Timing curve mapping linear progress to eased progress, mirroring CSS easing functions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// CSS `ease`: `cubic-bezier(0.25, 0.1, 0.25, 1)`.
    Ease,
    /// CSS `ease-in`: `cubic-bezier(0.42, 0, 1, 1)`.
    EaseIn,
    /// CSS `ease-out`: `cubic-bezier(0, 0, 0.58, 1)`.
    EaseOut,
    /// CSS `ease-in-out`: `cubic-bezier(0.42, 0, 0.58, 1)`.
    EaseInOut,
    /// Custom `cubic-bezier(x1, y1, x2, y2)`; `x1` and `x2` must lie in `0.0..=1.0` and all
    /// values must be finite.
    CubicBezier(f32, f32, f32, f32),
}
impl Easing {
    /// Returns the eased value for `progress`, which is clamped to `0.0..=1.0` first.
    /// Endpoints map to themselves; custom curves may overshoot in between.
    pub fn sample(self, progress: f32) -> f32 {
        let t = progress.clamp(0., 1.);
        if t == 0. || t == 1. || self == Self::Linear {
            return t;
        }
        let (x1, y1, x2, y2) = match self {
            Self::Ease => (0.25, 0.1, 0.25, 1.),
            Self::EaseIn => (0.42, 0., 1., 1.),
            Self::EaseOut => (0., 0., 0.58, 1.),
            Self::EaseInOut => (0.42, 0., 0.58, 1.),
            Self::CubicBezier(a, b, c, d) => (a, b, c, d),
            Self::Linear => unreachable!(),
        };
        let curve = |u: f32, a: f32, b: f32| {
            3. * (1. - u).powi(2) * u * a + 3. * (1. - u) * u * u * b + u.powi(3)
        };
        let (mut low, mut high) = (0., 1.);
        for _ in 0..24 {
            let u = (low + high) / 2.;
            if curve(u, x1, x2) < t {
                low = u;
            } else {
                high = u;
            }
        }
        curve((low + high) / 2., y1, y2)
    }
    pub(crate) fn valid(self) -> bool {
        match self {
            Self::CubicBezier(a, b, c, d) => {
                [a, b, c, d].iter().all(|v| v.is_finite())
                    && (0. ..=1.).contains(&a)
                    && (0. ..=1.).contains(&c)
            }
            _ => true,
        }
    }
}
