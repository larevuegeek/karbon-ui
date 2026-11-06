//! Per-item colours that the stylesheet cannot compute yet. Solid colours read the
//! inherited custom properties at runtime (`style:--h={…}` with `oklch(… var(--h))` in
//! the CSS), but gradients only resolve `var()` at compile time: the genre tiles and the
//! playlist cards, tinted per item, still get their gradients from these helpers, which
//! evaluate the formulas of `app.css`.
use karbon_ui::ui::prelude::*;

/// `oklch(l c h / alpha)` converted to sRGB (Björn Ottosson's OKLab matrices).
pub fn oklch(l: f32, c: f32, h: f32, alpha: f32) -> Color {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    oklab(l, a, b, alpha)
}

fn oklab(l: f32, a: f32, b: f32, alpha: f32) -> Color {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l3, m3, s3) = (l_.powi(3), m_.powi(3), s_.powi(3));
    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let bl = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;
    let encode = |x: f32| {
        let x = x.clamp(0., 1.);
        let x = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1. / 2.4) - 0.055
        };
        (x * 255.).round() as u8
    };
    Color::rgba(
        encode(r),
        encode(g),
        encode(bl),
        (alpha.clamp(0., 1.) * 255.).round() as u8,
    )
}

/// sRGB → OKLab (inverse of [`oklab`]).
fn to_oklab(c: Color) -> [f32; 3] {
    let lin = |v: u8| {
        let v = f32::from(v) / 255.;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(c.red), lin(c.green), lin(c.blue));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// `#rrggbb` → colour; the playlists store their colour as text.
pub fn hex(text: &str) -> Color {
    Color::hex(u32::from_str_radix(text.trim_start_matches('#'), 16).unwrap_or(0x22c55e))
}

/// `hue()` of `src/lib/helper/tools/hue.ts`: a stable hue (0–359) for a text.
pub fn hue(text: &str) -> f32 {
    let mut h: u32 = 0;
    for unit in text.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(u32::from(unit));
    }
    (h % 360) as f32
}

/// `.dark .genre-tuile`: corner glow, top-left sheen and a tinted base (front first).
pub fn genre_tile(h: f32) -> Vec<Background> {
    vec![
        RadialGradient::new(
            [1., 1.],
            [0.85, 1.1],
            [
                stop(0., oklch(0.55, 0.17, h, 0.5)),
                stop(0.62, oklch(0.55, 0.17, h, 0.)),
            ],
        )
        .into(),
        RadialGradient::new(
            [0., 0.],
            [1.1, 0.8],
            [
                stop(0., Color::rgba(255, 255, 255, 15)),
                stop(0.55, Color::rgba(255, 255, 255, 0)),
            ],
        )
        .into(),
        LinearGradient::new(
            70.,
            [
                stop(0., oklch(0.25, 0.03, h, 1.)),
                stop(1., oklch(0.17, 0.018, h, 1.)),
            ],
        )
        .into(),
    ]
}

/// PlaylistCard: `linear-gradient(140deg, couleur, color-mix(in oklab, couleur 45%, black))`
pub fn playlist_fond(couleur: &str) -> LinearGradient {
    let base = hex(couleur);
    let [l, a, b] = to_oklab(base);
    // Le noir vaut (0, 0, 0) en OKLab : le mélange n'est qu'une mise à l'échelle.
    // Angle de l'API Rust = angle CSS − 90° (0 vers la droite).
    LinearGradient::new(
        50.,
        [
            stop(0., base),
            stop(1., oklab(l * 0.45, a * 0.45, b * 0.45, 1.)),
        ],
    )
}

fn stop(position: f32, color: Color) -> GradientStop {
    GradientStop { position, color }
}
