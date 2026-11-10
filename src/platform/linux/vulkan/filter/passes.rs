pub(super) use super::super::encode::{Built, Step};
pub(super) use crate::{
    platform::shared::{boxes, gpu::color},
    ui::rendering::filter::{Filter, Filters},
};

/// Paramètres d'un passage, miroir de `FilterParams` (`filter.frag`).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Params {
    /// Largeur et hauteur utiles de la source, facteur k, rayon de la boîte.
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
pub(super) const MATRICES: usize = 16;

/// Calcul d'un passage (`kernel` du shader).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kernel {
    Downsample,
    Box,
    Upsample,
    Matrix,
    Shade,
    Merge,
}
/// Image lue par un passage : une couche de l'image en cours (contenu d'un groupe
/// filtré, zone d'un `backdrop-filter`), ou une cible de filtre.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Source {
    Layer(usize),
    Target(usize),
}
#[derive(Debug)]
pub(crate) struct Pass {
    pub kernel: Kernel,
    pub source: Source,
    /// Seconde source (ombre floutée sous le contenu).
    pub below: Option<Source>,
    /// Cible écrite (`Built::targets`).
    pub target: usize,
    /// Zone utile de la cible, en pixels.
    pub size: [usize; 2],
    /// Paramètres (`Built::params`).
    pub params: usize,
}
pub(super) fn params(size: [usize; 2]) -> Params {
    Params {
        size: [size[0] as f32, size[1] as f32, 1., 0.],
        shift: [0.; 4],
        tint: [0.; 4],
        rows: [[0.; 4]; 80],
    }
}
/// Passages en préparation : ils n'entrent dans les étapes qu'une fois la chaîne finie
/// (le dernier écrit alors dans la cible demandée).
pub(super) struct Plan<'a> {
    pub(super) built: &'a mut Built,
    pub(super) passes: Vec<Pass>,
}
impl Plan<'_> {
    pub(super) fn push(
        &mut self,
        kernel: Kernel,
        (source, below): (Source, Option<Source>),
        size: [usize; 2],
        p: Params,
    ) -> usize {
        let target = self.built.scratch(size);
        self.built.params.push(p);
        self.passes.push(Pass {
            kernel,
            source,
            below,
            target,
            size,
            params: self.built.params.len() - 1,
        });
        target
    }
    /// Flou d'écart type `sigma` pixels de `current` (sans effet en deçà de 0,05), comme
    /// `blur` côté CPU.
    pub(super) fn blur(&mut self, current: &mut Source, size: [usize; 2], sigma: f32) {
        if sigma.is_nan() || sigma <= 0.05 {
            return;
        }
        let k = ((sigma / 3.).floor() as usize).clamp(1, 8);
        let small = [size[0].div_ceil(k), size[1].div_ceil(k)];
        if k > 1 {
            let mut p = params(size);
            p.size[2] = k as f32;
            *current = Source::Target(self.push(Kernel::Downsample, (*current, None), small, p));
        }
        for width in boxes(sigma / k as f32) {
            let radius = (width - 1) / 2;
            if radius == 0 {
                continue;
            }
            for direction in [0., 1.] {
                let mut p = params(small);
                p.size[3] = radius as f32;
                p.shift[3] = direction;
                *current = Source::Target(self.push(Kernel::Box, (*current, None), small, p));
            }
        }
        if k > 1 {
            let mut p = params(small);
            p.size[2] = k as f32;
            *current = Source::Target(self.push(Kernel::Upsample, (*current, None), size, p));
        }
    }
}
impl Built {
    /// Ajoute aux étapes les passages qui appliquent `filters` à `source` (zone utile
    /// `size`) et écrivent le résultat, de même taille, dans la cible `output` ; `scale`
    /// convertit les unités logiques en pixels.
    pub(crate) fn plan(
        &mut self,
        source: Source,
        size: [usize; 2],
        filters: Filters,
        scale: f32,
        output: usize,
    ) {
        let unit = filters.scale * scale;
        let mut plan = Plan {
            built: self,
            passes: Vec::new(),
        };
        let mut current = source;
        let mut values = filters.values.iter().peekable();
        while let Some(filter) = values.next() {
            match *filter {
                Filter::Blur(sigma) => plan.blur(&mut current, size, sigma * unit),
                Filter::DropShadow {
                    x,
                    y,
                    blur: sigma,
                    color: tint,
                } => {
                    let mut p = params(size);
                    p.shift[0] = (x * unit).round();
                    p.shift[1] = (y * unit).round();
                    p.tint = color(tint);
                    let mut shade =
                        Source::Target(plan.push(Kernel::Shade, (current, None), size, p));
                    plan.blur(&mut shade, size, sigma * unit);
                    let merged =
                        plan.push(Kernel::Merge, (current, Some(shade)), size, params(size));
                    current = Source::Target(merged);
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
                    current = Source::Target(plan.push(Kernel::Matrix, (current, None), size, p));
                }
            }
        }
        let mut passes = plan.passes;
        match passes.last_mut() {
            Some(last) => last.target = output,
            // Aucun effet (flou nul) : copie.
            None => {
                self.params.push(params(size));
                passes.push(Pass {
                    kernel: Kernel::Matrix,
                    source: current,
                    below: None,
                    target: output,
                    size,
                    params: self.params.len() - 1,
                });
            }
        }
        self.steps.extend(passes.into_iter().map(Step::Filter));
    }
}
