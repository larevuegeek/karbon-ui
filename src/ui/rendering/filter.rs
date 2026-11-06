//! Portable CSS filter descriptions; the Windows backend builds native effect graphs.
use crate::ui::{scene::RectF, theme::Color};
/// One CSS filter function. Amounts follow CSS: `1.0` leaves the image unchanged for
/// `Brightness`, `Contrast` and `Saturate` (range `0.0..=10000.0`), and `0.0` does for the
/// `0.0..=1.0` amounts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Filter {
    /// Gaussian blur with the given standard deviation in logical pixels.
    Blur(f32),
    /// Linear brightness multiplier.
    Brightness(f32),
    /// Contrast multiplier.
    Contrast(f32),
    /// Conversion to grayscale, `0.0..=1.0`.
    Grayscale(f32),
    /// Conversion to sepia, `0.0..=1.0`.
    Sepia(f32),
    /// Saturation multiplier.
    Saturate(f32),
    /// Hue rotation in degrees.
    HueRotate(f32),
    /// Colour inversion, `0.0..=1.0`.
    Invert(f32),
    /// Opacity multiplier, `0.0..=1.0`.
    Opacity(f32),
    /// Shadow of the content's alpha, like CSS `drop-shadow()`.
    DropShadow {
        /// Horizontal offset in logical pixels.
        x: f32,
        /// Vertical offset in logical pixels.
        y: f32,
        /// Blur standard deviation in logical pixels.
        blur: f32,
        /// Shadow colour.
        color: Color,
    },
}
/// Ordered filter chain applied to an element and its content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Filters {
    /// Filters applied in order (at most 16).
    pub values: &'static [Filter],
    /// Multiplier for blur radii and shadow offsets (layout scale); `1.0` by default.
    pub scale: f32,
}
impl Filters {
    /// Filter chain at scale `1.0`.
    pub const fn new(values: &'static [Filter]) -> Self {
        Self { values, scale: 1. }
    }
    /// `true` if the chain can be rendered: at most 16 finite values within their ranges and
    /// blur radii no larger than 250 px once scaled.
    pub fn valid(self) -> bool {
        self.scale.is_finite()
            && self.scale > 0.
            && self.values.len() <= 16
            && self.values.iter().all(|f| match f {
                Filter::Blur(n) => n.is_finite() && *n >= 0. && *n * self.scale <= 250.,
                Filter::DropShadow { x, y, blur, .. } => {
                    [x, y, blur].iter().all(|n| n.is_finite())
                        && *blur >= 0.
                        && *blur * self.scale <= 250.
                }
                Filter::HueRotate(n) => n.is_finite(),
                Filter::Opacity(n)
                | Filter::Invert(n)
                | Filter::Grayscale(n)
                | Filter::Sepia(n) => n.is_finite() && (0. ..=1.).contains(n),
                Filter::Brightness(n) | Filter::Contrast(n) | Filter::Saturate(n) => {
                    n.is_finite() && (0. ..=10000.).contains(n)
                }
            })
    }
    /// Extra margin, in pixels, that blurs and drop shadows may paint outside the element.
    pub fn padding(self) -> f32 {
        self.values
            .iter()
            .map(|f| match f {
                Filter::Blur(n) => 3. * n * self.scale,
                Filter::DropShadow { x, y, blur, .. } => {
                    (x.abs().max(y.abs()) + 3. * blur) * self.scale
                }
                _ => 0.,
            })
            .sum::<f32>()
            .ceil()
    }
    /// `rect` grown by [`padding`](Self::padding) on every side.
    pub fn bounds(self, rect: RectF) -> RectF {
        let p = self.padding();
        RectF::new(
            rect.left - p,
            rect.top - p,
            rect.right - rect.left + 2. * p,
            rect.bottom - rect.top + 2. * p,
        )
    }
}
impl Filter {
    /// D2D row-vector 5x4 matrix; coefficients follow Filter Effects Level 1 §13.
    pub fn matrix(self) -> Option<[f32; 20]> {
        let mut matrix = [
            [1., 0., 0., 0., 0.],
            [0., 1., 0., 0., 0.],
            [0., 0., 1., 0., 0.],
            [0., 0., 0., 1., 0.],
        ];
        match self {
            Self::Brightness(n) => {
                for (i, row) in matrix.iter_mut().enumerate().take(3) {
                    row[i] = n;
                }
            }
            Self::Contrast(n) => {
                for (i, row) in matrix.iter_mut().enumerate().take(3) {
                    row[i] = n;
                    row[4] = 0.5 * (1. - n);
                }
            }
            Self::Invert(n) => {
                for (i, row) in matrix.iter_mut().enumerate().take(3) {
                    row[i] = 1. - 2. * n;
                    row[4] = n;
                }
            }
            Self::Opacity(n) => matrix[3][3] = n,
            Self::Grayscale(n) | Self::Sepia(n) => {
                let target = if matches!(self, Self::Grayscale(_)) {
                    [[0.2126, 0.7152, 0.0722]; 3]
                } else {
                    [
                        [0.393, 0.769, 0.189],
                        [0.349, 0.686, 0.168],
                        [0.272, 0.534, 0.131],
                    ]
                };
                for i in 0..3 {
                    for j in 0..3 {
                        matrix[i][j] = if i == j { 1. - n } else { 0. } + n * target[i][j];
                    }
                }
            }
            Self::Saturate(n) => {
                let weights = [0.213, 0.715, 0.072];
                for (i, row) in matrix.iter_mut().enumerate().take(3) {
                    for j in 0..3 {
                        row[j] = (1. - n) * weights[j] + if i == j { n } else { 0. };
                    }
                }
            }
            Self::HueRotate(degrees) => {
                let (s, c) = degrees.rem_euclid(360.).to_radians().sin_cos();
                let base = [0.213, 0.715, 0.072];
                let cos = [
                    [0.787, -0.715, -0.072],
                    [-0.213, 0.285, -0.072],
                    [-0.213, -0.715, 0.928],
                ];
                let sin = [
                    [-0.213, -0.715, 0.928],
                    [0.143, 0.140, -0.283],
                    [-0.787, 0.715, 0.072],
                ];
                for i in 0..3 {
                    for (j, b) in base.iter().enumerate() {
                        matrix[i][j] = b + c * cos[i][j] + s * sin[i][j];
                    }
                }
            }
            Self::Blur(_) | Self::DropShadow { .. } => return None,
        }
        Some(std::array::from_fn(|i| matrix[i % 4][i / 4]))
    }
}
