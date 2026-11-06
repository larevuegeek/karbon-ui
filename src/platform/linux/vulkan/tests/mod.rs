//! Tests du renderer Vulkan, hors écran : sur un GPU s'il y en a un, sinon sur llvmpipe
//! (Vulkan sur processeur). Sans Vulkan, ils se sautent en l'écrivant sur la sortie
//! d'erreur ; `KARBON_REQUIRE_VULKAN=1` en fait des échecs. `KARBON_VULKAN_DUMP=<dossier>`
//! y enregistre les images comparées à Cairo (`dump`).
mod filters;
mod images;
mod instances;
mod layers;
mod scenes;
mod text;

use super::{Cairo, Renderer};
use crate::ui::scene::Scene;

/// Renderer Vulkan et le renderer Cairo dont il reprend mises en page et images, comme
/// dans une fenêtre.
struct Gpu {
    vulkan: Renderer,
    cairo: Cairo,
}
impl std::ops::Deref for Gpu {
    type Target = Renderer;
    fn deref(&self) -> &Renderer {
        &self.vulkan
    }
}
impl std::ops::DerefMut for Gpu {
    fn deref_mut(&mut self) -> &mut Renderer {
        &mut self.vulkan
    }
}
impl Gpu {
    fn render(
        &mut self,
        scene: &Scene,
        width: usize,
        height: usize,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        self.vulkan
            .render(&mut self.cairo, scene, width, height, scale)
    }
}
fn renderer() -> Option<Gpu> {
    match Renderer::new(true) {
        Ok(vulkan) => Some(Gpu {
            vulkan,
            cairo: Cairo::new().unwrap(),
        }),
        Err(error) => {
            assert!(
                std::env::var_os("KARBON_REQUIRE_VULKAN").is_none(),
                "KARBON_REQUIRE_VULKAN : {error:?}"
            );
            eprintln!("tests Vulkan sautés : {error}");
            None
        }
    }
}
fn pixel(px: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * width + x) * 4;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}
/// Écart moyen par canal (sur 255) et part des canaux écartés de plus de 16.
fn difference(a: &[u8], b: &[u8]) -> (f64, f64) {
    let total: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    let far = a
        .iter()
        .zip(b)
        .filter(|(x, y)| x.abs_diff(**y) > 16)
        .count();
    (total as f64 / a.len() as f64, far as f64 / a.len() as f64)
}
/// Enregistre, si `KARBON_VULKAN_DUMP` nomme un dossier, les images Cairo et Vulkan de la
/// scène `name` à l'échelle `scale` et leur écart (amplifié 4 fois, sur fond noir).
fn dump(name: &str, scale: f32, (width, height): (usize, usize), cairo: &[u8], gpu: &[u8]) {
    let Some(dir) = std::env::var_os("KARBON_VULKAN_DUMP") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let gap: Vec<u8> = cairo
        .as_chunks::<4>()
        .0
        .iter()
        .zip(gpu.as_chunks::<4>().0)
        .flat_map(|(a, b)| {
            let d = |i: usize| a[i].abs_diff(b[i]).saturating_mul(4);
            [d(0), d(1), d(2), 255]
        })
        .collect();
    for (tag, pixels) in [("cairo", cairo), ("vulkan", gpu), ("ecart", &gap)] {
        let image =
            crate::ui::scene::Image::new(width as u32, height as u32, pixels.to_vec()).unwrap();
        let path = dir.join(format!("{name}-{scale}x-{tag}.png"));
        super::super::imaging::write_png(&image, &path).unwrap();
    }
}
/// Compare Vulkan et Cairo sur la scène `name` (`width` × `height` unités logiques) ;
/// écart moyen au plus `mean`, au plus `far` des canaux écartés de plus de 16.
///
/// Le renderer Vulkan garde ses caches d'une échelle à l'autre : une image affichée à une
/// autre taille qu'à l'image précédente y est d'abord étirée (taille animée, comme dans
/// le dessin Cairo), puis réduite exactement à l'image suivante. C'est cette seconde image
/// qui est comparée au rendu d'un renderer Cairo neuf.
pub(super) fn compare(name: &str, scene: &Scene, width: f32, height: f32, mean: f64, far: f64) {
    let Some(mut vulkan) = renderer() else {
        return;
    };
    // Le renderer Vulkan pose ses glyphes au quart de pixel comme Cairo ≥ 1.17.4 ; sur un
    // Cairo plus ancien (pixel entier), une scène avec du texte ne peut pas coïncider.
    let text = scene.commands.iter().any(|c| {
        use crate::ui::scene::DrawCommand as D;
        matches!(
            c,
            D::Text { .. }
                | D::FontText { .. }
                | D::FormattedText { .. }
                | D::RichText { .. }
                | D::PushTextFill { .. }
        )
    });
    if text && !crate::platform::linux::ffi::cairo::subpixel_glyphs() {
        eprintln!("{name} : Cairo sans positionnement sous-pixel (< 1.17.4), test sauté");
        return;
    }
    for s in [1f32, 1.5, 2.] {
        let (w, h) = ((width * s) as usize, (height * s) as usize);
        let cairo = super::super::graphics::Renderer::new()
            .unwrap()
            .render(scene, w as u32, h as u32, s)
            .unwrap();
        vulkan.render(scene, w, h, s).unwrap();
        let gpu = vulkan.render(scene, w, h, s).unwrap();
        dump(name, s, (w, h), &cairo, &gpu);
        let (m, f) = difference(&cairo, &gpu);
        eprintln!(
            "échelle {s} : écart moyen {m:.3}, au-delà de 16 : {:.4} %",
            f * 100.
        );
        assert!(
            m <= mean && f <= far,
            "échelle {s} : écart moyen {m:.3}, au-delà de 16 : {:.4} %",
            f * 100.
        );
    }
}
