//! Teinte, saturation et valeur d'une couleur, et son écriture hexadécimale.

/// A colour as hue, saturation and value: what a colour picker shows, a square of
/// saturation (across) and value (up) under a bar of hue. Kept apart from red, green and
/// blue because a grey or a black has no hue of its own: the picker keeps the one the
/// user chose.
///
/// ```
/// use karbon_ui::ui::editor::Hsv;
///
/// let orange = Hsv::from_rgb([1.0, 0.5, 0.0]);
/// assert!((orange.hue - 30.0).abs() < 1e-4);
/// assert_eq!((orange.saturation, orange.value), (1.0, 1.0));
/// assert_eq!(orange.to_rgb(), [1.0, 0.5, 0.0]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    /// Hue in degrees, from 0 (red) through 120 (green) and 240 (blue), below 360.
    pub hue: f32,
    /// From 0 (grey) to 1 (pure colour).
    pub saturation: f32,
    /// From 0 (black) to 1 (brightest).
    pub value: f32,
}

impl Hsv {
    /// The hue, saturation and value of `rgb` (red, green, blue from 0 to 1, clamped).
    /// A grey has the hue 0 and the saturation 0.
    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        let [r, g, b] = rgb.map(|c| c.clamp(0., 1.));
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let spread = max - min;
        let hue = if spread <= 0. {
            0.
        } else if max == r {
            60. * ((g - b) / spread).rem_euclid(6.)
        } else if max == g {
            60. * ((b - r) / spread + 2.)
        } else {
            60. * ((r - g) / spread + 4.)
        };
        Self {
            hue: if hue >= 360. { 0. } else { hue },
            saturation: if max <= 0. { 0. } else { spread / max },
            value: max,
        }
    }

    /// The red, green and blue of the colour, from 0 to 1.
    pub fn to_rgb(self) -> [f32; 3] {
        let hue = self.hue.rem_euclid(360.) / 60.;
        let saturation = self.saturation.clamp(0., 1.);
        let value = self.value.clamp(0., 1.);
        let chroma = value * saturation;
        let second = chroma * (1. - (hue % 2. - 1.).abs());
        let [r, g, b] = match hue as u32 {
            0 => [chroma, second, 0.],
            1 => [second, chroma, 0.],
            2 => [0., chroma, second],
            3 => [0., second, chroma],
            4 => [second, 0., chroma],
            _ => [chroma, 0., second],
        };
        let lift = value - chroma;
        [r + lift, g + lift, b + lift]
    }

    /// The pure colour of the hue: full saturation and value.
    pub fn pure(self) -> [f32; 3] {
        Self {
            saturation: 1.,
            value: 1.,
            ..self
        }
        .to_rgb()
    }
}

/// `rgb` (from 0 to 1, clamped) written `#RRGGBB`, in capitals.
///
/// ```
/// use karbon_ui::ui::editor::hex_of;
///
/// assert_eq!(hex_of([1.0, 0.5, 0.0]), "#FF8000");
/// ```
pub fn hex_of(rgb: [f32; 3]) -> String {
    let [r, g, b] = rgb.map(byte);
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// The colour written in `text`: six or three hexadecimal digits (`FF8000`, `f80`), with
/// or without `#`, spaces around ignored; `None` for anything else.
///
/// ```
/// use karbon_ui::ui::editor::parse_hex;
///
/// assert_eq!(parse_hex(" #f80 "), Some([1.0, 0x88 as f32 / 255.0, 0.0]));
/// assert_eq!(parse_hex("orange"), None);
/// ```
pub fn parse_hex(text: &str) -> Option<[f32; 3]> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |part: &str| u8::from_str_radix(part, 16).ok();
    let bytes = match digits.len() {
        6 => [
            channel(&digits[0..2])?,
            channel(&digits[2..4])?,
            channel(&digits[4..6])?,
        ],
        3 => {
            let doubled = |i: usize| channel(&digits[i..=i].repeat(2));
            [doubled(0)?, doubled(1)?, doubled(2)?]
        }
        _ => return None,
    };
    Some(bytes.map(|b| f32::from(b) / 255.))
}

/// Un canal de 0 à 1 en octet, arrondi.
pub(super) fn byte(channel: f32) -> u8 {
    (channel.clamp(0., 1.) * 255.).round() as u8
}
