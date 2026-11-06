// Calcul de l’image d’ombre en Rust ; aucune dépendance à une API d’OS.
use crate::ui::geometry::Size;
/// Shadow extent around the window, in logical pixels (multiply by the DPI scale).
pub const SHADOW_MARGIN: f32 = 32.0;
const BLUR: f32 = 9.0;
const OFFSET_Y: f32 = 6.0;
const OPACITY: f32 = 72.0; // Maximum sur 255, puis atténué par le dégradé.

fn rounded_rect_distance(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    let dx = x.abs() - width * 0.5 + radius;
    let dy = y.abs() - height * 0.5 + radius;
    dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius
}

/// Render the window drop shadow for a `width` × `height` window at the given DPI `scale`.
///
/// Returns the bitmap size (window size plus [`SHADOW_MARGIN`] × `scale` on every side) and
/// premultiplied BGRA pixels packed as `u32`; only alpha is non-zero and the window area
/// itself stays fully transparent.
pub fn generate_pixels(width: i32, height: i32, scale: f32) -> (Size, Vec<u32>) {
    let margin = (SHADOW_MARGIN * scale).ceil() as i32;
    let size = Size {
        width: width + 2 * margin,
        height: height + 2 * margin,
    };
    let mut pixels = vec![0; size.width as usize * size.height as usize];
    let radius = (8.0 * scale).min(width.min(height) as f32 * 0.5);
    let blur = BLUR * scale;
    let center_x = margin as f32 + width as f32 * 0.5;
    let center_y = margin as f32 + height as f32 * 0.5;

    // L'ombre est symétrique de gauche à droite : seule la moitié gauche est calculée,
    // puis recopiée en miroir. Sur une ligne, la forme est convexe : une fois le corps de
    // la fenêtre atteint, le reste jusqu'au milieu est transparent. Seules les bandes du
    // bord coûtent (≈ 10× moins de pixels qu'une fenêtre entière, à chaque redimensionnement).
    let row = size.width as usize;
    for y in 0..size.height {
        for x in 0..(size.width + 1) / 2 {
            let px = x as f32 + 0.5 - center_x;
            let py = y as f32 + 0.5 - center_y;
            let inside = rounded_rect_distance(px, py, width as f32, height as f32, radius);
            if inside <= -0.5 {
                break; // Le milieu reste complètement transparent.
            }
            let distance = rounded_rect_distance(
                px,
                py - OFFSET_Y * scale,
                width as f32,
                height as f32,
                radius,
            );
            // Profil gaussien : dégradé progressif, un peu plus marqué sous la fenêtre.
            let attenuation = ((distance + blur).max(0.0) / blur).powi(2) * -0.5;
            let edge = (inside + 0.5).clamp(0.0, 1.0);
            let alpha = (OPACITY * attenuation.exp() * edge).round() as u32;
            // Noir prémultiplié : RGB = 0, seule la transparence change.
            let line = y as usize * row;
            pixels[line + x as usize] = alpha << 24;
            pixels[line + row - 1 - x as usize] = alpha << 24;
        }
    }
    (size, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_is_transparent_inside_and_fades_outside() {
        let (size, pixels) = generate_pixels(200, 120, 1.0);
        let alpha = |x: usize, y: usize| pixels[y * size.width as usize + x] >> 24;
        assert_eq!(alpha(132, 92), 0);
        assert!(alpha(29, 92) > alpha(16, 92));
        assert!(alpha(16, 92) > 0);
        assert!(alpha(132, 160) > alpha(132, 24));
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(size.width as usize - 1, size.height as usize - 1), 0);
        assert!(pixels.iter().all(|pixel| pixel & 0x00FFFFFF == 0));
    }

    /// Calcul complet, pixel par pixel : référence de l'image en bandes et en miroir.
    fn brute_force(width: i32, height: i32, scale: f32) -> Vec<u32> {
        let margin = (SHADOW_MARGIN * scale).ceil() as i32;
        let (w, h) = (width + 2 * margin, height + 2 * margin);
        let radius = (8.0 * scale).min(width.min(height) as f32 * 0.5);
        let blur = BLUR * scale;
        let (cx, cy) = (
            margin as f32 + width as f32 * 0.5,
            margin as f32 + height as f32 * 0.5,
        );
        let mut pixels = vec![0; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let (px, py) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let inside = rounded_rect_distance(px, py, width as f32, height as f32, radius);
                if inside <= -0.5 {
                    continue;
                }
                let d = rounded_rect_distance(
                    px,
                    py - OFFSET_Y * scale,
                    width as f32,
                    height as f32,
                    radius,
                );
                let attenuation = ((d + blur).max(0.0) / blur).powi(2) * -0.5;
                let edge = (inside + 0.5).clamp(0.0, 1.0);
                pixels[(y * w + x) as usize] =
                    ((OPACITY * attenuation.exp() * edge).round() as u32) << 24;
            }
        }
        pixels
    }

    #[test]
    fn banded_mirrored_shadow_matches_the_full_computation() {
        for (w, h, scale) in [
            (200, 120, 1.0),
            (601, 451, 1.5),
            (1921, 1351, 1.25),
            (10, 7, 2.0),
        ] {
            assert!(
                generate_pixels(w, h, scale).1 == brute_force(w, h, scale),
                "{w}x{h}@{scale}"
            );
        }
    }

    #[test]
    fn shadow_scales_with_dpi() {
        let (size, pixels) = generate_pixels(600, 450, 1.5);
        assert_eq!((size.width, size.height), (696, 546));
        assert_eq!(pixels.len(), 696 * 546);
        assert!(pixels.iter().any(|pixel| pixel >> 24 > 0));
        assert!(pixels.iter().all(|pixel| pixel >> 24 <= OPACITY as u32));
    }
}
