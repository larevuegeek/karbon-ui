//! Filtres CSS sur le GPU, miroir de `graphics/filters.rs` : une chaîne de filtres devient
//! une suite de passages (`filters.metal`), chacun lisant la sortie du précédent dans des
//! textures RGBA 16 bits flottants (pas d'arrondi entre deux filtres, comme les plans
//! flottants du chemin Core Graphics).
//!
//! - flou : écart type en pixels ; au-delà de 6, calculé sur une image réduite d'un
//!   facteur k = ⌊σ/3⌋ ≤ 8 (moyenne des blocs, flou de σ/k, agrandissement bilinéaire),
//!   exactement comme `blur` côté Core Graphics ; flou gaussien séparable en deux passages ;
//! - ombre portée : alpha décalé d'un nombre entier de pixels et teinté, flouté, puis le
//!   contenu posé dessus ;
//! - matrices de couleur consécutives : un seul passage, bornées après chacune.
mod kernels;

use super::{device::Texture, encode::Step};
use crate::ui::rendering::filter::{Filter, Filters};
pub(super) use kernels::{FORMAT, Pipelines, encode};

/// Paramètres d'un passage, miroir de `FilterParams` (`filters.metal`).
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Params {
    /// Largeur et hauteur utiles de la source, facteur k, écart type.
    pub size: [f32; 4],
    /// Décalage de l'ombre, nombre de matrices, direction du flou (0 : x, 1 : y).
    pub shift: [f32; 4],
    /// Couleur de l'ombre, non prémultipliée.
    pub tint: [f32; 4],
    /// Seize matrices de cinq lignes.
    pub rows: [[f32; 4]; 80],
}
const _: () = assert!(std::mem::size_of::<Params>() == 83 * 16);
/// Plus grand nombre de matrices enchaînées en un passage.
const MATRICES: usize = 16;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Kernel {
    Downsample,
    Blur,
    Upsample,
    Matrix,
    Shade,
    Merge,
}
/// Image lue par un passage : une couche de l'image en cours (contenu d'un groupe
/// filtré) ou une texture.
#[derive(Clone)]
pub(super) enum Source {
    Layer(usize),
    Texture(Texture),
}
pub(super) struct Pass {
    pub kernel: Kernel,
    pub source: Source,
    /// Seconde source (ombre floutée sous le contenu).
    pub below: Option<Texture>,
    pub target: Texture,
    /// Zone utile de la cible, en pixels.
    pub size: [usize; 2],
    pub params: Box<Params>,
}
fn params(size: [usize; 2]) -> Box<Params> {
    Box::new(Params {
        size: [size[0] as f32, size[1] as f32, 1., 0.],
        shift: [0.; 4],
        tint: [0.; 4],
        rows: [[0.; 4]; 80],
    })
}
/// Textures intermédiaires prises par le planificateur.
pub(super) type Scratch<'a> = dyn FnMut([usize; 2]) -> crate::Result<Texture> + 'a;

/// Ajoute à `steps` les passages qui appliquent `filters` à `source` (zone utile `size`)
/// et écrivent le résultat, de même taille, dans `output` ; `scale` convertit les unités
/// logiques en pixels.
pub(super) fn plan(
    steps: &mut Vec<Step>,
    scratch: &mut Scratch<'_>,
    source: Source,
    size: [usize; 2],
    filters: Filters,
    scale: f32,
    output: &Texture,
) -> crate::Result<()> {
    let unit = filters.scale * scale;
    let mut passes = Vec::new();
    let mut current = source;
    let mut values = filters.values.iter().peekable();
    while let Some(filter) = values.next() {
        match *filter {
            Filter::Blur(sigma) => blur(&mut passes, scratch, &mut current, size, sigma * unit)?,
            Filter::DropShadow {
                x,
                y,
                blur: sigma,
                color,
            } => {
                let shade = scratch(size)?;
                let mut p = params(size);
                p.shift[0] = (x * unit).round();
                p.shift[1] = (y * unit).round();
                p.tint = super::instance::color(color);
                passes.push(Pass {
                    kernel: Kernel::Shade,
                    source: current.clone(),
                    below: None,
                    target: shade.clone(),
                    size,
                    params: p,
                });
                let mut blurred = Source::Texture(shade);
                blur(&mut passes, scratch, &mut blurred, size, sigma * unit)?;
                let Source::Texture(below) = blurred else {
                    unreachable!("le flou écrit dans une texture")
                };
                let merged = scratch(size)?;
                passes.push(Pass {
                    kernel: Kernel::Merge,
                    source: current,
                    below: Some(below),
                    target: merged.clone(),
                    size,
                    params: params(size),
                });
                current = Source::Texture(merged);
            }
            other => {
                let Some(first) = other.matrix() else {
                    continue;
                };
                let mut p = params(size);
                let mut count = 0;
                let push = |m: [f32; 20], p: &mut Params, count: &mut usize| {
                    for row in 0..5 {
                        p.rows[*count * 5 + row] = std::array::from_fn(|c| m[row * 4 + c]);
                    }
                    *count += 1;
                };
                push(first, &mut p, &mut count);
                while count < MATRICES
                    && let Some(m) = values.peek().and_then(|f| f.matrix())
                {
                    values.next();
                    push(m, &mut p, &mut count);
                }
                p.shift[2] = count as f32;
                let target = scratch(size)?;
                passes.push(Pass {
                    kernel: Kernel::Matrix,
                    source: current,
                    below: None,
                    target: target.clone(),
                    size,
                    params: p,
                });
                current = Source::Texture(target);
            }
        }
    }
    match passes.last_mut() {
        Some(last) => last.target = output.clone(),
        // Aucun effet (flou nul) : copie.
        None => passes.push(Pass {
            kernel: Kernel::Matrix,
            source: current,
            below: None,
            target: output.clone(),
            size,
            params: params(size),
        }),
    }
    steps.extend(passes.into_iter().map(Step::Filter));
    Ok(())
}
/// Flou gaussien d'écart type `sigma` pixels (sans effet en deçà de 0,05).
fn blur(
    passes: &mut Vec<Pass>,
    scratch: &mut Scratch<'_>,
    current: &mut Source,
    size: [usize; 2],
    sigma: f32,
) -> crate::Result<()> {
    if sigma.is_nan() || sigma <= 0.05 {
        return Ok(());
    }
    let k = ((sigma / 3.).floor() as usize).clamp(1, 8);
    let small = [size[0].div_ceil(k), size[1].div_ceil(k)];
    if k > 1 {
        let reduced = scratch(small)?;
        let mut p = params(size);
        p.size[2] = k as f32;
        passes.push(Pass {
            kernel: Kernel::Downsample,
            source: current.clone(),
            below: None,
            target: reduced.clone(),
            size: small,
            params: p,
        });
        *current = Source::Texture(reduced);
    }
    for direction in [0., 1.] {
        let target = scratch(small)?;
        let mut p = params(small);
        p.size[3] = sigma / k as f32;
        p.shift[3] = direction;
        passes.push(Pass {
            kernel: Kernel::Blur,
            source: current.clone(),
            below: None,
            target: target.clone(),
            size: small,
            params: p,
        });
        *current = Source::Texture(target);
    }
    if k > 1 {
        let target = scratch(size)?;
        let mut p = params(small);
        p.size[2] = k as f32;
        passes.push(Pass {
            kernel: Kernel::Upsample,
            source: current.clone(),
            below: None,
            target: target.clone(),
            size,
            params: p,
        });
        *current = Source::Texture(target);
    }
    Ok(())
}
