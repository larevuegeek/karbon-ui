//! Portées : découpes rectangulaires et générales, groupes à opacité (couche
//! intermédiaire), et confection des étapes de dessin d'une couche (copie de
//! `macos/metal/encode/scopes.rs`).
use super::{Bind, Builder, Layer, NO_CLIP, Scope, State, Step, intersect};
use crate::platform::shared::gpu::{
    ClipNode, Instance, PAINT_TEXTURE, SHAPE_NONE, pixel, rect as array,
};
use crate::ui::{rendering::transform::Affine, scene::RectF};

impl Builder<'_> {
    pub(super) fn pop(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Plain(state)) => self.state = state,
            Some(Scope::Group {
                saved,
                child,
                opacity,
                rect,
                mask,
            }) => {
                self.state = saved;
                // Groupe vide (tout son contenu découpé) : rien à recomposer.
                if self.built.layers[child].instances.is_empty() {
                    return;
                }
                self.flush(child);
                let layer = &self.built.layers[child];
                let (w, h) = (layer.size[0] as f32, layer.size[1] as f32);
                let (tw, th) = (layer.texture_size[0] as f32, layer.texture_size[1] as f32);
                let mut i = Instance::new(rect, Affine::IDENTITY, SHAPE_NONE, PAINT_TEXTURE);
                i.uv = [0., 0., w / tw, h / th];
                i.t[3] = opacity;
                if let Some((bounds, gradient)) = mask {
                    // Opacité du groupe courant déjà appliquée au contenu, comme Cairo.
                    self.mask(bounds, &gradient);
                    self.state.alpha = 1.;
                }
                self.emit(i, Bind::Layer(child));
                self.state = saved;
            }
            None => {}
        }
    }
    /// Confie `layer`, complète, à une étape de dessin.
    pub(super) fn flush(&mut self, layer: usize) {
        self.built.steps.push(Step::Draw(layer));
    }
    /// Découpe à `rect` (repère courant).
    pub(super) fn clip_rect(&mut self, rect: RectF) {
        let t = self.state.transform;
        if t.m12 == 0. && t.m21 == 0. {
            let device = t.bounds(rect);
            self.state.clip = intersect(self.state.clip, array(device));
        } else {
            self.clip_chain(rect, [0.; 4]);
        }
    }
    pub(super) fn clip_chain(&mut self, rect: RectF, radii: [f32; 4]) {
        let t = self.state.transform;
        let Some(inverse) = t.inverse() else {
            // Repère dégénéré : rien n'est visible dans la découpe.
            self.state.clip = [0., 0., 0., 0.];
            return;
        };
        self.built.clips.push(ClipNode {
            m: [inverse.m11, inverse.m12, inverse.m21, inverse.m22],
            t: [inverse.dx, inverse.dy, pixel(t), 0.],
            rect: array(rect),
            radii,
            link: [self.state.chain, 0, 0, 0],
        });
        self.state.chain = self.built.clips.len() as i32 - 1;
        // Le lot reste limité aux bornes de la découpe.
        let device = t.bounds(rect);
        self.state.clip = intersect(self.state.clip, array(device));
    }
    /// Ouvre une portée d'effet ; `false` si son contenu est entièrement invisible.
    pub(super) fn effect(
        &mut self,
        transform: Affine,
        opacity: f32,
        bounds: RectF,
        layer: bool,
    ) -> bool {
        let saved = self.state;
        let combined = transform.then(self.state.transform);
        if !layer {
            self.scopes.push(Scope::Plain(saved));
            self.state.transform = combined;
            self.state.alpha *= opacity;
            return true;
        }
        let parent = &self.built.layers[self.state.layer];
        let device = intersect(
            intersect(array(combined.bounds(bounds)), self.state.clip),
            parent.area(),
        );
        let (left, top) = (device[0].floor(), device[1].floor());
        let (right, bottom) = (device[2].ceil(), device[3].ceil());
        if right <= left || bottom <= top || !(left.is_finite() && top.is_finite()) {
            return false;
        }
        let size = [(right - left) as usize, (bottom - top) as usize];
        let layer = Layer::new([left as i32, top as i32], size, [0.; 4]);
        self.built.layers.push(layer);
        let child = self.built.layers.len() - 1;
        self.scopes.push(Scope::Group {
            saved,
            child,
            opacity,
            rect: RectF {
                left,
                top,
                right,
                bottom,
            },
            mask: None,
        });
        // La découpe du parent s'applique à la recomposition, pas au contenu.
        self.state = State {
            transform: combined,
            alpha: 1.,
            clip: NO_CLIP,
            chain: -1,
            layer: child,
        };
        true
    }
}
