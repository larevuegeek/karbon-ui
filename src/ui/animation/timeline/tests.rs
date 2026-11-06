use super::value::Interpolate;
use crate::ui::theme::Color;
#[test]
fn colours_interpolate_in_premultiplied_alpha() {
    let white = Color::rgba(255, 255, 255, 255);
    let transparent = Color::rgba(0, 0, 0, 0);
    // `transparent` ↔ blanc : seule l'opacité change, jamais de gris.
    assert_eq!(
        white.lerp(transparent, 0.5),
        Color::rgba(255, 255, 255, 128)
    );
    assert_eq!(
        transparent.lerp(white, 0.25),
        Color::rgba(255, 255, 255, 64)
    );
    assert_eq!(white.lerp(transparent, 1.), transparent);
    // Couleurs opaques : interpolation directe des canaux.
    let blue = Color::rgba(13, 110, 253, 255);
    assert_eq!(white.lerp(blue, 0.5), Color::rgba(134, 183, 254, 255));
    // Opacités différentes : moyenne pondérée par l'alpha.
    let half_red = Color::rgba(255, 0, 0, 128);
    let mid = half_red.lerp(Color::rgba(0, 0, 255, 255), 0.5);
    assert_eq!(mid.alpha, 192);
    assert!(mid.blue > mid.red, "{mid:?}");
}
