pub(super) use super::super::{super::graphics::Renderer as CgRenderer, Renderer};
pub(super) use crate::ui::scene::Scene;

/// Renderer Metal, ou `None` (test sauté) sans GPU.
pub(crate) fn metal() -> Option<Renderer> {
    if !Renderer::available() {
        assert!(
            std::env::var_os("KARBON_REQUIRE_METAL").is_none(),
            "Metal requis, aucun périphérique"
        );
        eprintln!("Metal indisponible : test sauté");
        return None;
    }
    // Un périphérique existe : toute autre erreur (shaders, pipeline) est un échec.
    Some(Renderer::new().expect("renderer Metal"))
}
/// Écart entre deux rendus.
#[derive(Debug)]
pub(crate) struct Difference {
    /// Plus grand écart d'un canal.
    pub max: u8,
    /// Écart moyen par canal.
    pub mean: f64,
    /// Part des pixels dont un canal s'écarte de plus de 16.
    pub outliers: f64,
}
pub(super) fn difference(a: &[u8], b: &[u8]) -> Difference {
    let (max, mean, outliers) = super::super::compare::difference(a, b);
    Difference {
        max,
        mean,
        outliers,
    }
}
pub(super) fn dump(name: &str, scale: f32, width: u32, height: u32, metal: &[u8], cg: &[u8]) {
    let Some(dir) = std::env::var_os("KARBON_METAL_DUMP") else {
        return;
    };
    for (kind, pixels) in [("metal", metal), ("cg", cg)] {
        let image = crate::ui::scene::Image {
            width,
            height,
            pixels: pixels.to_vec(),
        };
        let path = std::path::Path::new(&dir).join(format!("{name}-{scale}x-{kind}.png"));
        super::super::super::imaging::write_png(&image, &path).unwrap();
    }
    // Écart amplifié, sur fond blanc.
    let image = super::super::compare::amplified(metal, cg, width, height);
    let path = std::path::Path::new(&dir).join(format!("{name}-{scale}x-diff.png"));
    super::super::super::imaging::write_png(&image, &path).unwrap();
    let d = difference(metal, cg);
    eprintln!(
        "{name} à {scale}x : écart maximal {}, moyen {:.3}, hors seuil {:.4}",
        d.max, d.mean, d.outliers
    );
}
/// Rend `scene` (`width` × `height` points) par les deux chemins à `scale`.
pub(crate) fn both(
    metal: &mut Renderer,
    cg: &mut CgRenderer,
    scene: &Scene,
    (width, height): (f32, f32),
    scale: f32,
) -> (Vec<u8>, Vec<u8>, u32, u32) {
    let (w, h) = (
        (width * scale).round() as u32,
        (height * scale).round() as u32,
    );
    let m = metal.render(cg, scene, w, h, scale).unwrap();
    let c = cg.render(scene, w, h, scale).unwrap();
    (m, c, w, h)
}
/// Compare les deux rendus de `scene` à 1x et 2x : écart moyen ≤ `mean`, au plus
/// `outliers` des pixels au-delà de 16 (bords anticrénelés).
pub(crate) fn close(name: &str, scene: &Scene, size: (f32, f32), mean: f64, outliers: f64) {
    let Some(mut metal) = metal() else {
        return;
    };
    let mut cg = CgRenderer::new().unwrap();
    for scale in [1f32, 2.] {
        let (m, c, w, h) = both(&mut metal, &mut cg, scene, size, scale);
        dump(name, scale, w, h, &m, &c);
        let d = difference(&m, &c);
        assert!(
            d.mean <= mean && d.outliers <= outliers,
            "{name} à {scale}x : {d:?} (tolérance : moyenne {mean}, hors seuil {outliers})"
        );
    }
}
pub(crate) fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}
