//! Ombres floutées rastérisées : couverture de la forme, flou en trois passes de boîte
//! sur une grille éventuellement réduite, puis découpe nette de l'élément.
use super::{BoxShadow, Frame, Paint, normalized};
use crate::ui::scene::Image;
use std::sync::Arc;
fn distance(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> f32 {
    if w <= 0. || h <= 0. {
        return f32::INFINITY;
    }
    let (cx, cy, radius) = if x < r[0] && y < r[0] {
        (r[0], r[0], r[0])
    } else if x > w - r[1] && y < r[1] {
        (w - r[1], r[1], r[1])
    } else if x > w - r[2] && y > h - r[2] {
        (w - r[2], h - r[2], r[2])
    } else if x < r[3] && y > h - r[3] {
        (r[3], h - r[3], r[3])
    } else {
        return (-x).max(x - w).max(-y).max(y - h);
    };
    (x - cx).hypot(y - cy) - radius
}
pub(super) fn blur(input: &mut [f32], width: usize, height: usize, sigma: f32) {
    if sigma <= 0. {
        return;
    }
    let ideal = (4. * sigma * sigma + 1.).sqrt();
    let mut low = ideal.floor() as usize;
    if low.is_multiple_of(2) {
        low = low.saturating_sub(1);
    }
    low = low.max(1);
    let high = low + 2;
    let count = ((12. * sigma * sigma - 3. * (low * low) as f32 - 12. * low as f32 - 9.)
        / (-4. * low as f32 - 4.))
        .round()
        .clamp(0., 3.) as usize;
    let mut scratch = vec![0f32; input.len()];
    for pass in 0..3 {
        let size = if pass < count { low } else { high };
        let radius = (size / 2) as isize;
        let norm = 1. / size as f32;
        for y in 0..height {
            let mut sum = 0.;
            for x in -radius..=radius {
                if (0..width as isize).contains(&x) {
                    sum += input[y * width + x as usize];
                }
            }
            for x in 0..width {
                scratch[y * width + x] = sum * norm;
                let old = x as isize - radius;
                let next = x as isize + radius + 1;
                if old >= 0 {
                    sum -= input[y * width + old as usize];
                }
                if next < width as isize {
                    sum += input[y * width + next as usize];
                }
            }
        }
        for x in 0..width {
            let mut sum = 0.;
            for y in -radius..=radius {
                if (0..height as isize).contains(&y) {
                    sum += scratch[y as usize * width + x];
                }
            }
            for y in 0..height {
                input[y * width + x] = sum * norm;
                let old = y as isize - radius;
                let next = y as isize + radius + 1;
                if old >= 0 {
                    sum -= scratch[old as usize * width + x];
                }
                if next < height as isize {
                    sum += scratch[next as usize * width + x];
                }
            }
        }
    }
}
pub(super) fn generate(
    w: f32,
    h: f32,
    radii: [f32; 4],
    s: BoxShadow,
    scale: f32,
) -> crate::Result<Frame> {
    generate_with(w, h, radii, s, scale, 8.)
}
/// `max_down` : facteur de réduction maximal de la grille du flou (1 = pleine résolution).
pub(super) fn generate_with(
    w: f32,
    h: f32,
    radii: [f32; 4],
    s: BoxShadow,
    scale: f32,
    max_down: f32,
) -> crate::Result<Frame> {
    let (dx, dy, sigma, spread) = (
        s.x * scale,
        s.y * scale,
        s.blur * scale / 2.,
        s.spread * scale,
    );
    let pad = (dx.abs().max(dy.abs()) + spread.abs() + sigma * 3. + 2.).ceil();
    let width = (w + pad * 2.).ceil() as usize;
    let height = (h + pad * 2.).ceil() as usize;
    if width > 8192 || height > 8192 || width.checked_mul(height).is_none_or(|n| n > 4_194_304) {
        return Err(crate::Error::invalid_input(
            "component shadow exceeds its 4M pixel allocation limit",
        ));
    }
    let original = normalized(w, h, radii);
    let grow = if s.inset { -spread } else { spread };
    let sw = (w + 2. * grow).max(0.);
    let sh = (h + 2. * grow).max(0.);
    let sr = normalized(sw, sh, radii.map(|r| (r + grow).max(0.)));
    // Flou large : la forme floutée n'a pas de détail fin, elle se calcule sur une grille
    // réduite (au moins 2 px de flou effectif) puis s'interpole ; seule la découpe nette
    // de l'élément reste en pleine résolution. Coût divisé par ~`down²`.
    let down = (sigma / 2.).floor().clamp(1., max_down.max(1.));
    let lw = (width as f32 / down).ceil() as usize + 1;
    let lh = (height as f32 / down).ceil() as usize + 1;
    let mut low = vec![0f32; lw * lh];
    for y in 0..lh {
        for x in 0..lw {
            let alpha = (0.5
                - distance(
                    (x as f32 + 0.5) * down - pad - dx + grow,
                    (y as f32 + 0.5) * down - pad - dy + grow,
                    sw,
                    sh,
                    sr,
                ) / down)
                .clamp(0., 1.);
            low[y * lw + x] = if s.inset { 1. - alpha } else { alpha };
        }
    }
    blur(&mut low, lw, lh, sigma / down);
    // Échantillonnage bilinéaire de la grille réduite : indices et poids d'une colonne ou
    // d'une ligne, calculés une fois par colonne et par ligne (mêmes opérations, mêmes
    // valeurs qu'un calcul par pixel).
    let weights = |count: usize, cells: usize| -> Vec<(usize, usize, f32)> {
        (0..count)
            .map(|i| {
                let f = ((i as f32 + 0.5) / down - 0.5).max(0.);
                let i0 = f as usize;
                (i0, (i0 + 1).min(cells - 1), f - i0 as f32)
            })
            .collect()
    };
    let (columns, rows) = (weights(width, lw), weights(height, lh));
    let sample = |x: usize, y: usize| -> f32 {
        if down == 1. {
            return low[y * lw + x];
        }
        let ((x0, x1, tx), (y0, y1, ty)) = (columns[x], rows[y]);
        let top = low[y0 * lw + x0] * (1. - tx) + low[y0 * lw + x1] * tx;
        let bottom = low[y1 * lw + x0] * (1. - tx) + low[y1 * lw + x1] * tx;
        top * (1. - ty) + bottom * ty
    };
    let corner = original.iter().copied().fold(0f32, f32::max) + 1.;
    // Couverture de l'élément : 0 ou 1 loin de son bord, distance exacte près du bord et
    // dans les coins arrondis seulement.
    let own = |px: f32, py: f32| -> f32 {
        if px < -1. || py < -1. || px > w + 1. || py > h + 1. {
            return 0.;
        }
        let inner = px > 1. && py > 1. && px < w - 1. && py < h - 1.;
        let in_corner = (px < corner || px > w - corner) && (py < corner || py > h - corner);
        if inner && !in_corner {
            return 1.;
        }
        (0.5 - distance(px, py, w, h, original)).clamp(0., 1.)
    };
    let mut pixels = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let own = own(x as f32 + 0.5 - pad, y as f32 + 0.5 - pad);
            if (!s.inset && own >= 1.) || (s.inset && own <= 0.) {
                continue;
            }
            let alpha = (sample(x, y).clamp(0., 1.)
                * if s.inset { own } else { 1. - own }
                * s.color.alpha as f32)
                .round() as u8;
            let i = (y * width + x) * 4;
            pixels[i] = (s.color.blue as u32 * alpha as u32 / 255) as u8;
            pixels[i + 1] = (s.color.green as u32 * alpha as u32 / 255) as u8;
            pixels[i + 2] = (s.color.red as u32 * alpha as u32 / 255) as u8;
            pixels[i + 3] = alpha;
        }
    }
    Ok(Frame {
        layer: 0,
        paint: Paint::Image(Arc::new(Image::new(width as u32, height as u32, pixels)?)),
        x: -pad,
        y: -pad,
        width: width as f32,
        height: height as f32,
        inset: s.inset,
    })
}
