//! Filtres CSS sur CPU, appliqués à des pixels BGRA prémultipliés : flou gaussien
//! (trois flous en boîte), matrices de couleur (Filter Effects Level 1, comme les effets
//! Direct2D) et ombre portée.
use crate::ui::rendering::filter::{Filter, Filters};

/// Pixels flottants prémultipliés, canaux B, G, R, A.
struct Plane {
    width: usize,
    height: usize,
    data: Vec<[f32; 4]>,
}
impl Plane {
    fn from_bgra(pixels: &[u8], width: usize, height: usize) -> Self {
        let data = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p.map(f32::from))
            .collect();
        Self {
            width,
            height,
            data,
        }
    }
    fn write(&self, pixels: &mut [u8]) {
        for (target, value) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(&self.data) {
            let alpha = value[3].round().clamp(0., 255.);
            *target = [
                value[0].round().clamp(0., alpha) as u8,
                value[1].round().clamp(0., alpha) as u8,
                value[2].round().clamp(0., alpha) as u8,
                alpha as u8,
            ];
        }
    }
}
/// Applique `filters` à une surface de `width` × `height` pixels ; `scale` convertit les
/// unités logiques (rayons, décalages) en pixels.
pub(in crate::platform) fn apply(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    filters: Filters,
    scale: f32,
) {
    let mut plane = Plane::from_bgra(pixels, width, height);
    let unit = filters.scale * scale;
    for filter in filters.values {
        match *filter {
            Filter::Blur(sigma) => blur(&mut plane, sigma * unit),
            Filter::DropShadow {
                x,
                y,
                blur: sigma,
                color,
            } => {
                shadow(&mut plane, x * unit, y * unit, sigma * unit, color);
            }
            other => {
                if let Some(matrix) = other.matrix() {
                    color_matrix(&mut plane, &matrix);
                }
            }
        }
    }
    plane.write(pixels);
}
/// Largeurs des trois boîtes approchant une gaussienne d'écart type `sigma`
/// (W. Jarosz, « Fast Image Convolutions ») ; reprises par les passages du renderer Vulkan.
pub(in crate::platform) fn boxes(sigma: f32) -> [usize; 3] {
    let ideal = (12. * sigma * sigma / 3. + 1.).sqrt();
    let mut lower = ideal.floor() as usize;
    if lower.is_multiple_of(2) {
        lower = lower.saturating_sub(1);
    }
    let lower = lower.max(1);
    let upper = lower + 2;
    let l = lower as f32;
    let m = ((12. * sigma * sigma - 3. * l * l - 12. * l - 9.) / (-4. * l - 4.)).round();
    std::array::from_fn(|i| if (i as f32) < m { lower } else { upper })
}
/// Flou en boîte horizontal de rayon `radius` (sommes glissantes) ; l'extérieur est
/// transparent.
fn box_horizontal(plane: &mut Plane, radius: usize) {
    let w = plane.width;
    let norm = 1. / (2 * radius + 1) as f32;
    let mut line = vec![[0f32; 4]; w];
    for row in plane.data.chunks_exact_mut(w) {
        line.copy_from_slice(row);
        let mut sum = [0f32; 4];
        for value in line.iter().take(radius.min(w)) {
            add(&mut sum, value, 1.);
        }
        for (x, target) in row.iter_mut().enumerate() {
            if x + radius < w {
                add(&mut sum, &line[x + radius], 1.);
            }
            if x > radius {
                add(&mut sum, &line[x - radius - 1], -1.);
            }
            *target = sum.map(|v| v * norm);
        }
    }
}
/// Flou en boîte vertical : une somme par colonne, les lignes parcourues dans l'ordre de
/// la mémoire.
fn box_vertical(plane: &mut Plane, radius: usize) {
    let (w, h) = (plane.width, plane.height);
    let norm = 1. / (2 * radius + 1) as f32;
    let source = plane.data.clone();
    let row = |y: usize| &source[y * w..(y + 1) * w];
    let mut sums = vec![[0f32; 4]; w];
    for y in 0..radius.min(h) {
        for (sum, value) in sums.iter_mut().zip(row(y)) {
            add(sum, value, 1.);
        }
    }
    for y in 0..h {
        if y + radius < h {
            for (sum, value) in sums.iter_mut().zip(row(y + radius)) {
                add(sum, value, 1.);
            }
        }
        if y > radius {
            for (sum, value) in sums.iter_mut().zip(row(y - radius - 1)) {
                add(sum, value, -1.);
            }
        }
        for (target, sum) in plane.data[y * w..(y + 1) * w].iter_mut().zip(&sums) {
            *target = sum.map(|v| v * norm);
        }
    }
}
fn add(sum: &mut [f32; 4], value: &[f32; 4], sign: f32) {
    for c in 0..4 {
        sum[c] += sign * value[c];
    }
}
fn blur_full(plane: &mut Plane, sigma: f32) {
    for size in boxes(sigma) {
        let radius = (size - 1) / 2;
        if radius > 0 {
            box_horizontal(plane, radius);
            box_vertical(plane, radius);
        }
    }
}
/// Flou gaussien d'écart type `sigma` pixels. Au-delà de quelques pixels, il est calculé
/// sur une image réduite d'un facteur `k` (moyenne des blocs) puis agrandie
/// (bilinéaire) : le résultat est visuellement équivalent pour un coût divisé par k².
fn blur(plane: &mut Plane, sigma: f32) {
    if sigma.is_nan() || sigma <= 0.05 {
        return;
    }
    let k = ((sigma / 3.).floor() as usize).clamp(1, 8);
    if k == 1 {
        blur_full(plane, sigma);
        return;
    }
    let mut small = downsample(plane, k);
    blur_full(&mut small, sigma / k as f32);
    upsample(&small, plane, k);
}
fn downsample(plane: &Plane, k: usize) -> Plane {
    let (w, h) = (plane.width, plane.height);
    let (sw, sh) = (w.div_ceil(k), h.div_ceil(k));
    let mut data = vec![[0f32; 4]; sw * sh];
    for y in 0..h {
        let target = &mut data[(y / k) * sw..(y / k + 1) * sw];
        for (x, value) in plane.data[y * w..(y + 1) * w].iter().enumerate() {
            add(&mut target[x / k], value, 1.);
        }
    }
    for (sy, row) in data.chunks_exact_mut(sw).enumerate() {
        let rows = (h - sy * k).min(k);
        for (sx, value) in row.iter_mut().enumerate() {
            let count = (rows * (w - sx * k).min(k)) as f32;
            *value = value.map(|v| v / count);
        }
    }
    Plane {
        width: sw,
        height: sh,
        data,
    }
}
fn upsample(small: &Plane, plane: &mut Plane, k: usize) {
    let (sw, sh) = (small.width, small.height);
    let at = |x: usize, y: usize| &small.data[y.min(sh - 1) * sw + x.min(sw - 1)];
    // Coordonnée source et poids de chaque colonne, calculés une fois.
    let coordinate = |i: usize, n: usize| {
        let s = ((i as f32 + 0.5) / k as f32 - 0.5).max(0.);
        let base = (s.floor() as usize).min(n - 1);
        (base, s - base as f32)
    };
    let columns = (0..plane.width)
        .map(|x| coordinate(x, sw))
        .collect::<Vec<_>>();
    let w = plane.width;
    for y in 0..plane.height {
        let (y0, fy) = coordinate(y, sh);
        for (x, target) in plane.data[y * w..(y + 1) * w].iter_mut().enumerate() {
            let (x0, fx) = columns[x];
            let (a, b, c, d) = (
                at(x0, y0),
                at(x0 + 1, y0),
                at(x0, y0 + 1),
                at(x0 + 1, y0 + 1),
            );
            *target = std::array::from_fn(|i| {
                let top = a[i] + (b[i] - a[i]) * fx;
                let bottom = c[i] + (d[i] - c[i]) * fx;
                top + (bottom - top) * fy
            });
        }
    }
}
/// Matrice 5 × 4 en vecteur ligne (`m[ligne * 4 + colonne]`, ligne 4 = décalage) sur
/// les couleurs non prémultipliées, comme l'effet ColorMatrix de Direct2D.
fn color_matrix(plane: &mut Plane, m: &[f32; 20]) {
    for pixel in &mut plane.data {
        let a = pixel[3] / 255.;
        if a <= 0. && m[19] <= 0. {
            continue;
        }
        let straight = if a > 0. {
            [
                pixel[2] / 255. / a,
                pixel[1] / 255. / a,
                pixel[0] / 255. / a,
                a,
            ]
        } else {
            [0., 0., 0., 0.]
        };
        let out: [f32; 4] = std::array::from_fn(|col| {
            let v = (0..4)
                .map(|row| straight[row] * m[row * 4 + col])
                .sum::<f32>()
                + m[16 + col];
            v.clamp(0., 1.)
        });
        let alpha = out[3];
        *pixel = [
            out[2] * alpha * 255.,
            out[1] * alpha * 255.,
            out[0] * alpha * 255.,
            alpha * 255.,
        ];
    }
}
/// Ombre de l'alpha du contenu, décalée et floutée, peinte sous le contenu.
fn shadow(plane: &mut Plane, dx: f32, dy: f32, sigma: f32, color: crate::ui::theme::Color) {
    let (w, h) = (plane.width, plane.height);
    let (ox, oy) = (dx.round() as isize, dy.round() as isize);
    let tint = [
        f32::from(color.blue) / 255.,
        f32::from(color.green) / 255.,
        f32::from(color.red) / 255.,
        f32::from(color.alpha) / 255.,
    ];
    let mut shade = Plane {
        width: w,
        height: h,
        data: vec![[0.; 4]; w * h],
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            let (sx, sy) = (x - ox, y - oy);
            if sx < 0 || sy < 0 || sx >= w as isize || sy >= h as isize {
                continue;
            }
            let a = plane.data[sy as usize * w + sx as usize][3] * tint[3];
            shade.data[y as usize * w + x as usize] = [a * tint[0], a * tint[1], a * tint[2], a];
        }
    }
    blur(&mut shade, sigma);
    for (top, below) in plane.data.iter_mut().zip(&shade.data) {
        let rest = 1. - top[3] / 255.;
        for c in 0..4 {
            top[c] += below[c] * rest;
        }
    }
}
/// Facteur de réduction d'un groupe filtré : son plus petit flou, en pixels, borne la
/// résolution utile. Une ombre portée garde la pleine résolution (contenu net). Partagé
/// avec le renderer Metal, qui réduit ses groupes filtrés de la même façon.
pub(in crate::platform) fn reduction(filters: Filters, scale: f32) -> usize {
    if filters
        .values
        .iter()
        .any(|f| matches!(f, Filter::DropShadow { .. }))
    {
        return 1;
    }
    let sigma = filters
        .values
        .iter()
        .filter_map(|f| match f {
            Filter::Blur(n) => Some(*n),
            _ => None,
        })
        .fold(f32::INFINITY, f32::min)
        * filters.scale
        * scale;
    if !sigma.is_finite() {
        return 1;
    }
    ((sigma / 3.).floor() as usize).clamp(1, 8)
}
