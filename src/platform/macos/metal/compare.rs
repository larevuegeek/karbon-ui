//! Diagnostic `KARBON_METAL_COMPARE=<dossier>` : une image présentée par Metal est aussi
//! rendue par Metal vers une texture relue et par Core Graphics (rendu de référence),
//! une seconde après la première image puis au plus une fois par seconde ; les deux images et leur écart amplifié sont écrits dans
//! le dossier, et l'écart (moyen par canal, maximal, part des pixels au-delà de 16) sur la
//! sortie d'erreur. Sert à comparer les deux chemins sur une vraie fenêtre (RustMusic).
use super::{CgRenderer, Renderer};
use crate::ui::scene::{Image, Scene};
use std::time::{Duration, Instant};

/// Écart entre deux images BGRA de même taille : maximal, moyen par canal, part des
/// pixels dont un canal s'écarte de plus de 16.
pub(super) fn difference(a: &[u8], b: &[u8]) -> (u8, f64, f64) {
    let mut max = 0u8;
    let mut sum = 0u64;
    let mut outliers = 0usize;
    for (p, q) in a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0) {
        let d = p.iter().zip(q).map(|(x, y)| x.abs_diff(*y));
        let worst = d.clone().max().unwrap_or(0);
        sum += d.map(u64::from).sum::<u64>();
        max = max.max(worst);
        if worst > 16 {
            outliers += 1;
        }
    }
    let n = (a.len() / 4).max(1);
    (
        max,
        sum as f64 / a.len().max(1) as f64,
        outliers as f64 / n as f64,
    )
}
/// Image d'écart amplifié (×4), sur fond blanc.
pub(super) fn amplified(a: &[u8], b: &[u8], width: u32, height: u32) -> Image {
    let pixels = a
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .flat_map(|(p, q)| {
            let d = p
                .iter()
                .zip(q)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            let v = 255u8.saturating_sub(d.saturating_mul(4));
            [v, v, 255, 255]
        })
        .collect();
    Image {
        width,
        height,
        pixels,
    }
}
/// Comparaison périodique des deux chemins d'une fenêtre.
#[derive(Default)]
pub(in crate::platform::macos) struct Comparison {
    last: Option<Instant>,
    count: usize,
}
impl Comparison {
    /// Compare `scene` rendue par les deux chemins si le diagnostic est demandé.
    pub(in crate::platform::macos) fn run(
        &mut self,
        metal: &mut Renderer,
        cg: &mut CgRenderer,
        scene: &Scene,
        (width, height): (usize, usize),
        scale: f32,
    ) {
        let Some(dir) = std::env::var_os("KARBON_METAL_COMPARE") else {
            return;
        };
        // Première comparaison une seconde après la première image (animations
        // d'ouverture terminées), puis au plus une par seconde.
        let last = *self.last.get_or_insert_with(Instant::now);
        if last.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last = Some(Instant::now());
        self.count += 1;
        let (w, h) = (width as u32, height as u32);
        let result = metal
            .render(cg, scene, w, h, scale)
            .and_then(|m| Ok((m, cg.render(scene, w, h, scale)?)));
        // L'image Core Graphics suivante de la fenêtre doit être dessinée en entier.
        cg.forget();
        let (m, c) = match result {
            Ok(pair) => pair,
            Err(error) => {
                eprintln!("karbon-ui: comparaison Metal impossible : {error}");
                return;
            }
        };
        let dir = std::path::Path::new(&dir);
        let n = self.count;
        for (kind, pixels) in [("metal", &m), ("cg", &c)] {
            let image = Image {
                width: w,
                height: h,
                pixels: pixels.clone(),
            };
            let _ = super::super::imaging::write_png(&image, &dir.join(format!("{n}-{kind}.png")));
        }
        let _ = super::super::imaging::write_png(
            &amplified(&m, &c, w, h),
            &dir.join(format!("{n}-diff.png")),
        );
        let (max, mean, outliers) = difference(&m, &c);
        eprintln!(
            "karbon-ui: comparaison {n} ({width}×{height}) : écart maximal {max}, moyen {mean:.3}, au-delà de 16 {:.3} %",
            outliers * 100.
        );
    }
}
