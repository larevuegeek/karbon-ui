//! Effets à couches, miroir de `graphics/layers.rs` (copie adaptée de
//! `macos/metal/encode/effects.rs`) :
//!
//! - groupe filtré : contenu rendu dans une couche bornée à la zone des filtres (bornes
//!   élargies du flou et de l'ombre), réduite d'un facteur k pour un grand flou
//!   (`shared::reduction`), filtrée (`filter/`), recomposée sous la découpe du parent ;
//!   le résultat est gardé d'une image à l'autre tant que la signature du contenu et sa
//!   place ne changent pas ;
//! - `backdrop-filter` : ce que la cible finale a reçu jusque-là dans la zone relue est
//!   redessiné dans une couche de cette taille (mêmes instances, même ordre), filtré,
//!   puis recomposé découpé par la forme arrondie ; ignoré dans un groupe filtré (Cairo
//!   n'y a pas de surface à relire) ;
//! - `mask-image` : le contenu est composé dans une couche (groupe Cairo), recomposée
//!   par un maillon de la chaîne des découpes (alpha du dégradé). Metal masque chaque
//!   élément, comme `CGContextClipToMask` ; ici le masque porte sur le groupe composé,
//!   comme Cairo et CSS (différent sous un contenu translucide superposé) ;
//! - `background-clip: text` : voir `text.rs`.
use super::{Bind, Builder, Layer, NO_CLIP, Scope, State, Target, intersect};
use crate::{
    platform::{
        linux::vulkan::{filter::Source, resources::Kept},
        shared::{
            gpu::{ClipNode, Instance, PAINT_TEXTURE, SHAPE_NONE, pixel},
            linear_points, normalized, placement, reduction,
        },
    },
    ui::{
        element::ElementId,
        rendering::{
            brush::LinearGradient,
            filter::{Filter, Filters},
            transform::Affine,
        },
        scene::{DrawCommand, RectF, signature::Signature},
    },
};
use std::sync::Arc;

/// Zone entière en pixels de la couche courante : gauche, haut, largeur, hauteur.
#[derive(Clone, Copy)]
struct Area {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}
impl Area {
    fn rect(self) -> RectF {
        RectF::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32)
    }
}
impl Builder<'_> {
    /// Facteur d'échelle moyen du repère courant (pixels par unité logique).
    fn scale(&self) -> f32 {
        let t = self.state.transform;
        (t.m11 * t.m22 - t.m12 * t.m21).abs().sqrt()
    }
    /// Pixels couverts par `rect` (repère courant), bornés à `bounds` et à la découpe
    /// courante élargie de `margin` pixels (`Canvas::area`).
    fn area(&self, rect: RectF, margin: f32, bounds: [f32; 4]) -> Option<Area> {
        let device = self.state.transform.bounds(rect);
        let clip = self.state.clip;
        let left = device.left.max(clip[0] - margin).max(bounds[0]).floor();
        let top = device.top.max(clip[1] - margin).max(bounds[1]).floor();
        let right = device.right.min(clip[2] + margin).min(bounds[2]).ceil();
        let bottom = device.bottom.min(clip[3] + margin).min(bounds[3]).ceil();
        if !(right > left && bottom > top) {
            return None;
        }
        Some(Area {
            x: left as usize,
            y: top as usize,
            w: (right - left) as usize,
            h: (bottom - top) as usize,
        })
    }
    /// Pose la cible `target` (zone utile `size`) étirée sur `area`, sous la découpe
    /// courante.
    fn composite(&mut self, target: usize, size: [usize; 2], area: Area) {
        let [tw, th] = self.built.targets[target].size.map(|n| n as f32);
        let mut i = Instance::new(area.rect(), Affine::IDENTITY, SHAPE_NONE, PAINT_TEXTURE);
        i.uv = [0., 0., size[0] as f32 / tw, size[1] as f32 / th];
        self.emit(i, Bind::Target(target));
    }
    /// Groupe filtré (`Canvas::filter`).
    pub(super) fn filter(
        &mut self,
        id: ElementId,
        commands: &[DrawCommand],
        bounds: RectF,
        filters: Filters,
    ) -> crate::Result<()> {
        let scale = self.scale();
        let margin = filters.padding() * scale;
        let output = filters.bounds(bounds);
        let extent = self.built.layers[self.state.layer].area();
        // Zone entière tant qu'elle est raisonnable : elle ne dépend alors pas de la
        // découpe courante et le résultat reste en cache pendant un défilement.
        let area = match self.area(output, f32::INFINITY, extent) {
            Some(a) if a.w * a.h <= 4_194_304 => Some(a),
            _ => self.area(output, margin, extent),
        };
        let Some(area) = area else {
            return Ok(());
        };
        let layer_transform = self
            .state
            .transform
            .then(Affine::translation(-(area.x as f32), -(area.y as f32)));
        let key = Signature::new(commands, bounds, filters, scale);
        let place = placement(layer_transform, area.w, area.h);
        if let Some(hit) = self.resources.take_filtered(id, &key, place) {
            let size = hit.kept.size;
            self.built.targets.push(Target {
                size: [hit.image.width, hit.image.height],
                image: Some(hit.image),
                keep: Some(hit.kept),
            });
            self.composite(self.built.targets.len() - 1, size, area);
            return Ok(());
        }
        if area.w * area.h > 16_777_216 {
            // Groupe trop grand pour une couche : dessiné sans ses filtres plutôt que de
            // faire échouer l'image entière, comme Cairo.
            return self.run(commands);
        }
        // Un grand flou efface les détails : le contenu est dessiné réduit d'un facteur
        // `k`, flouté, puis agrandi à la recomposition.
        let k = reduction(filters, scale);
        let size = [area.w.div_ceil(k), area.h.div_ceil(k)];
        self.built.layers.push(Layer::new([0, 0], size, [0.; 4]));
        let child = self.built.layers.len() - 1;
        let saved = self.state;
        self.state = State {
            transform: layer_transform.then(Affine {
                m11: 1. / k as f32,
                m22: 1. / k as f32,
                ..Affine::IDENTITY
            }),
            alpha: 1.,
            clip: NO_CLIP,
            chain: -1,
            layer: child,
        };
        self.filter_depth += 1;
        let result = self.run(commands);
        self.filter_depth -= 1;
        self.state = saved;
        result?;
        let reused = self.resources.filter_output(id, size);
        if self.built.layers[child].instances.is_empty() {
            // Contenu transparent : les filtres le laissent transparent (aucune matrice
            // ne décale l'alpha).
            return Ok(());
        }
        self.flush(child);
        self.built.targets.push(Target {
            size,
            image: reused,
            keep: Some(Kept {
                id,
                key,
                placement: place,
                size,
                _source: commands.to_vec(),
            }),
        });
        let output = self.built.targets.len() - 1;
        self.built.plan(
            Source::Layer(child),
            size,
            filters,
            scale / k as f32,
            output,
        );
        self.composite(output, size, area);
        Ok(())
    }
    /// `backdrop-filter` (`Canvas::backdrop`).
    pub(super) fn backdrop(
        &mut self,
        rect: RectF,
        radii: [f32; 4],
        filters: Filters,
    ) -> crate::Result<()> {
        if self.filter_depth > 0 {
            return Ok(());
        }
        let blur = filters
            .values
            .iter()
            .map(|f| match f {
                Filter::Blur(n) => *n,
                _ => 0.,
            })
            .sum::<f32>()
            * filters.scale;
        let margin = (blur * 3.).ceil();
        let grown = RectF {
            left: rect.left - margin,
            top: rect.top - margin,
            right: rect.right + margin,
            bottom: rect.bottom + margin,
        };
        let root = self.built.layers[0].area();
        let Some(area) = self.area(grown, margin * self.scale(), root) else {
            return Ok(());
        };
        // Ce que la cible a reçu dans la zone, redessiné dans une couche de sa taille.
        let size = [area.w, area.h];
        let source = self.replay(area);
        let output = self.built.scratch(size);
        let scale = self.scale();
        self.built
            .plan(Source::Layer(source), size, filters, scale, output);
        let saved = self.state;
        let (shape, radii) = normalized(rect, radii, None);
        if radii.iter().all(|r| *r <= 0.) {
            self.clip_rect(shape);
        } else {
            self.clip_chain(shape, radii);
        }
        self.composite(output, size, area);
        self.state = saved;
        Ok(())
    }
    /// Couche de la zone `area` de la cible finale : son effacement puis, dans l'ordre,
    /// ses instances visibles dans la zone, avec leurs découpes et leurs textures.
    fn replay(&mut self, area: Area) -> usize {
        let a = [
            area.x as f32,
            area.y as f32,
            (area.x + area.w) as f32,
            (area.y + area.h) as f32,
        ];
        let root = &self.built.layers[0];
        let mut layer = Layer::new([area.x as i32, area.y as i32], [area.w, area.h], root.clear);
        for batch in &root.batches {
            for index in batch.start..batch.start + batch.count {
                let v = intersect(root.visible[index], a);
                if v[2] > v[0] && v[3] > v[1] {
                    layer.append(root.instances[index], batch.bind);
                }
            }
        }
        self.built.layers.push(layer);
        let index = self.built.layers.len() - 1;
        self.flush(index);
        index
    }
    /// `mask-image` : le contenu jusqu'à `PopMask` est rendu dans une couche couvrant la
    /// découpe courante (comme le groupe de Cairo), puis recomposé masqué (`pop`).
    pub(super) fn mask_group(&mut self, bounds: RectF, gradient: &Arc<LinearGradient>) {
        let saved = self.state;
        let parent = &self.built.layers[self.state.layer];
        let device = intersect(self.state.clip, parent.area());
        let (left, top) = (device[0].floor(), device[1].floor());
        let (right, bottom) = (device[2].ceil(), device[3].ceil());
        if right <= left || bottom <= top {
            // Rien de visible : le contenu est découpé entièrement.
            self.scopes.push(Scope::Plain(saved));
            self.state.clip = [0., 0., 0., 0.];
            return;
        }
        let size = [(right - left) as usize, (bottom - top) as usize];
        self.built
            .layers
            .push(Layer::new([left as i32, top as i32], size, [0.; 4]));
        let child = self.built.layers.len() - 1;
        self.scopes.push(Scope::Group {
            saved,
            child,
            opacity: 1.,
            rect: RectF {
                left,
                top,
                right,
                bottom,
            },
            mask: Some((bounds, Arc::clone(gradient))),
        });
        // Découpes du parent à la recomposition ; opacité du groupe courant sur chaque
        // élément, comme la canvas Cairo qui la garde dans le groupe.
        self.state = State {
            transform: saved.transform,
            alpha: saved.alpha,
            clip: NO_CLIP,
            chain: -1,
            layer: child,
        };
    }
    /// Maillon multipliant ce qui suit par l'alpha du dégradé posé sur `bounds` et
    /// prolongé au-delà (`Canvas::mask`).
    pub(super) fn mask(&mut self, bounds: RectF, gradient: &LinearGradient) {
        let t = self.state.transform;
        let Some(inverse) = t.inverse() else {
            self.state.clip = [0., 0., 0., 0.];
            return;
        };
        let (a, b) = linear_points(gradient, bounds);
        let [first, count] = self.stops(&gradient.stops);
        self.built.clips.push(ClipNode {
            m: [inverse.m11, inverse.m12, inverse.m21, inverse.m22],
            t: [inverse.dx, inverse.dy, pixel(t), 0.],
            rect: [a.x, a.y, b.x, b.y],
            radii: [0.; 4],
            link: [self.state.chain, 1, first as i32, count as i32],
        });
        self.state.chain = self.built.clips.len() as i32 - 1;
    }
}
