pub(super) use super::super::super::graphics::Renderer as CgRenderer;
pub(super) use super::super::{
    device::{Gpu, Texture},
    instance::{ClipNode, Instance, NO_CLIP, Stop},
    resources::Resources,
};
pub(super) use crate::ui::{
    rendering::{brush::Background, transform::Affine},
    scene::{DrawCommand, RectF, Scene},
};

/// Texture d'un lot : aucune, une texture de l'image, ou une couche intermédiaire.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Bind {
    None,
    Texture(usize),
    Layer(usize),
}
pub(crate) struct Batch {
    pub bind: Bind,
    /// Scissor en pixels de la couche : x, y, largeur, hauteur.
    pub scissor: [usize; 4],
    pub start: usize,
    pub count: usize,
}
/// Cible d'un passage de rendu : la fenêtre (couche 0) ou une texture intermédiaire.
pub(crate) struct Layer {
    /// Coin de la couche dans les pixels de son repère (cible finale, ou contenu d'un
    /// groupe filtré).
    pub origin: [i32; 2],
    /// Taille utile, et taille de la texture (arrondie pour être réutilisée).
    pub size: [usize; 2],
    pub texture_size: [usize; 2],
    /// Couleur d'effacement prémultipliée.
    pub clear: [f64; 4],
    pub instances: Vec<Instance>,
    pub batches: Vec<Batch>,
    /// Zone visible (pixels) de chaque instance de la cible finale, relue par
    /// `backdrop-filter`.
    pub(super) visible: Vec<[f32; 4]>,
}
impl Layer {
    pub(super) fn new(origin: [i32; 2], size: [usize; 2], clear: [f64; 4]) -> Self {
        Self {
            origin,
            size,
            texture_size: [bucket(size[0]), bucket(size[1])],
            clear,
            instances: Vec::new(),
            batches: Vec::new(),
            visible: Vec::new(),
        }
    }
    /// Ajoute `instance` (découpe déjà posée) au dernier lot s'il a la même texture et le
    /// même scissor, sinon à un nouveau lot.
    pub(super) fn append(&mut self, instance: Instance, bind: Bind) {
        // Scissor : bornes de la découpe arrondies vers l'extérieur, en pixels de la couche.
        let r = intersect(instance.clip, self.area());
        let x0 = (r[0].floor() as i32 - self.origin[0]).max(0) as usize;
        let y0 = (r[1].floor() as i32 - self.origin[1]).max(0) as usize;
        let x1 = ((r[2].ceil() as i32 - self.origin[0]).max(0) as usize).min(self.size[0]);
        let y1 = ((r[3].ceil() as i32 - self.origin[1]).max(0) as usize).min(self.size[1]);
        let scissor = [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)];
        self.instances.push(instance);
        let index = self.instances.len() - 1;
        match self.batches.last_mut() {
            Some(batch) if batch.bind == bind && batch.scissor == scissor => batch.count += 1,
            _ => self.batches.push(Batch {
                bind,
                scissor,
                start: index,
                count: 1,
            }),
        }
    }
    /// Zone de la couche dans les pixels de son repère.
    pub(super) fn area(&self) -> [f32; 4] {
        [
            self.origin[0] as f32,
            self.origin[1] as f32,
            (self.origin[0] + self.size[0] as i32) as f32,
            (self.origin[1] + self.size[1] as i32) as f32,
        ]
    }
}
/// Étape de l'image, exécutée dans l'ordre.
pub(crate) enum Step {
    /// Passage de rendu de la couche `layer` (effacée, puis tous ses lots).
    Draw(usize),
    /// Passage de filtre.
    Filter(super::super::filter::Pass),
}
/// Image prête à encoder.
#[derive(Default)]
pub(crate) struct Built {
    pub layers: Vec<Layer>,
    pub steps: Vec<Step>,
    pub textures: Vec<Texture>,
    pub stops: Vec<Stop>,
    pub clips: Vec<ClipNode>,
    /// Textures d'images nouvelles dont les niveaux de mipmap restent à calculer.
    pub mipmaps: Vec<Texture>,
    /// Couches vidées d'une image précédente, dont les vecteurs sont réutilisés.
    pub(super) spare: Vec<Layer>,
}
impl Built {
    /// Vide l'image encodée en gardant ses vecteurs pour la suivante (moins
    /// d'allocations, donc moins de fragmentation du tas pendant les animations) ;
    /// textures et passages sont relâchés.
    pub fn recycle(&mut self) {
        self.steps.clear();
        self.textures.clear();
        self.stops.clear();
        self.clips.clear();
        self.mipmaps.clear();
        self.spare.append(&mut self.layers);
    }
    /// Couche vide (vecteurs repris d'une couche recyclée s'il y en a).
    pub(super) fn layer(&mut self, origin: [i32; 2], size: [usize; 2], clear: [f64; 4]) -> Layer {
        let Some(mut layer) = self.spare.pop() else {
            return Layer::new(origin, size, clear);
        };
        layer.origin = origin;
        layer.size = size;
        layer.texture_size = [bucket(size[0]), bucket(size[1])];
        layer.clear = clear;
        layer.instances.clear();
        layer.batches.clear();
        layer.visible.clear();
        layer
    }
}
#[derive(Clone, Copy)]
pub(super) struct State {
    /// Repère logique courant → pixels de la couche (cible finale, ou contenu d'un
    /// groupe filtré).
    pub(super) transform: Affine,
    pub(super) alpha: f32,
    /// Découpe rectangulaire en pixels.
    pub(super) clip: [f32; 4],
    /// Dernier maillon de la chaîne des découpes générales et masques (-1 : aucun).
    pub(super) chain: i32,
    pub(super) layer: usize,
}
pub(super) enum Scope {
    Plain(State),
    /// Groupe rendu dans la couche `child`, recomposé à `rect` avec `opacity`.
    Group {
        saved: State,
        child: usize,
        opacity: f32,
        rect: RectF,
    },
}
pub(crate) struct Builder<'a> {
    pub gpu: &'a Gpu,
    pub resources: &'a mut Resources,
    pub cg: &'a mut CgRenderer,
    pub(super) built: Built,
    pub(super) state: State,
    pub(super) scopes: Vec<Scope>,
    /// Profondeur des groupes filtrés en cours (un `backdrop-filter` y est ignoré, comme
    /// dans la couche de filtre de Core Graphics).
    pub(super) filter_depth: usize,
    /// Remplissages du texte (`background-clip: text`) et leur zone.
    pub(super) text_fills: Vec<(Background, RectF)>,
    /// Index dans `built.textures` de chaque texture déjà liée (par adresse).
    pub(super) bound: std::collections::HashMap<usize, usize>,
}
/// Bornes d'un groupe arrondies à 64 pixels pour réutiliser les textures.
pub(super) fn bucket(n: usize) -> usize {
    n.div_ceil(64) * 64
}
pub(super) fn intersect(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}
impl<'a> Builder<'a> {
    /// Traduction de `scene` ; `built` est une image précédente recyclée
    /// (`Built::recycle`) ou vide.
    pub fn new(
        gpu: &'a Gpu,
        resources: &'a mut Resources,
        cg: &'a mut CgRenderer,
        scene: &Scene,
        (width, height): (usize, usize),
        scale: f32,
        mut built: Built,
    ) -> Self {
        built.recycle();
        let c = scene.clear;
        let a = f64::from(c.alpha) / 255.;
        let mut root = built.layer(
            [0, 0],
            [width, height],
            [
                f64::from(c.red) / 255. * a,
                f64::from(c.green) / 255. * a,
                f64::from(c.blue) / 255. * a,
                a,
            ],
        );
        root.texture_size = [width, height];
        built.layers.push(root);
        let transform = Affine {
            m11: scale,
            m22: scale,
            ..Affine::IDENTITY
        };
        Self {
            gpu,
            resources,
            cg,
            built,
            state: State {
                transform,
                alpha: 1.,
                clip: NO_CLIP,
                chain: -1,
                layer: 0,
            },
            scopes: Vec::new(),
            filter_depth: 0,
            text_fills: Vec::new(),
            bound: Default::default(),
        }
    }
    pub fn finish(mut self) -> Built {
        while !self.scopes.is_empty() {
            self.pop();
        }
        self.flush(0);
        self.built
    }
    pub fn run(&mut self, commands: &[DrawCommand]) -> crate::Result<()> {
        let mut index = 0;
        while index < commands.len() {
            let command = &commands[index];
            index += 1;
            match command {
                DrawCommand::PushEffect {
                    transform,
                    opacity,
                    bounds,
                } => {
                    let end = super::super::super::graphics::scope_end(commands, index - 1);
                    if *opacity <= 1. / 512. {
                        index = end + 1;
                        continue;
                    }
                    let layer = *opacity < 1.
                        && !super::super::super::graphics::single(&commands[index..end]);
                    if !self.effect(*transform, *opacity, *bounds, layer) {
                        index = end + 1;
                    }
                }
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    let end = super::super::super::graphics::scope_end(commands, index - 1);
                    self.filter(*id, &commands[index..end], *bounds, *filters)?;
                    index = end + 1;
                }
                other => self.draw(other)?,
            }
        }
        Ok(())
    }
    pub(super) fn draw(&mut self, command: &DrawCommand) -> crate::Result<()> {
        match command {
            DrawCommand::PopClip
            | DrawCommand::PopShapeClip
            | DrawCommand::PopEffect
            | DrawCommand::PopMask
            | DrawCommand::PopFilter => self.pop(),
            DrawCommand::PushClip(rect) => {
                self.scopes.push(Scope::Plain(self.state));
                self.clip_rect(*rect);
            }
            DrawCommand::PushShapeClip { rect, radii } => {
                self.scopes.push(Scope::Plain(self.state));
                let (rect, radii) = super::super::super::graphics::normalized(*rect, *radii, None);
                if radii.iter().all(|r| *r <= 0.) {
                    self.clip_rect(rect);
                } else {
                    self.clip_chain(rect, radii);
                }
            }
            DrawCommand::PushMask { bounds, gradient } => {
                self.scopes.push(Scope::Plain(self.state));
                self.mask(*bounds, gradient);
            }
            DrawCommand::PushTextFill { rect, fill } => self.text_fills.push((fill.clone(), *rect)),
            DrawCommand::PopTextFill => {
                self.text_fills.pop();
            }
            DrawCommand::Backdrop {
                rect,
                radii,
                filters,
            } => self.backdrop(*rect, *radii, *filters)?,
            DrawCommand::PushEffect { .. } | DrawCommand::PushFilter { .. } => {
                unreachable!("effects and filters handled by run")
            }
            other => self.paint(other)?,
        }
        Ok(())
    }
    /// Lie `texture` pour cette image et renvoie son lot.
    pub(crate) fn bind(&mut self, texture: &Texture) -> Bind {
        let key = objc2::rc::Retained::as_ptr(texture) as *const () as usize;
        let index = *self.bound.entry(key).or_insert_with(|| {
            self.built.textures.push(texture.clone());
            self.built.textures.len() - 1
        });
        Bind::Texture(index)
    }
    pub(crate) fn stops(
        &mut self,
        stops: &[crate::ui::rendering::brush::GradientStop],
    ) -> [u32; 2] {
        let first = self.built.stops.len() as u32;
        self.built.stops.extend(stops.iter().map(|s| Stop {
            color: super::super::instance::color(s.color),
            position: [s.position, 0., 0., 0.],
        }));
        [first, stops.len() as u32]
    }
    pub(crate) fn mipmaps(&mut self, texture: Texture) {
        self.built.mipmaps.push(texture);
    }
    pub(crate) fn transform(&self) -> Affine {
        self.state.transform
    }
    /// Ajoute `instance` à la couche courante avec la découpe et l'opacité en cours.
    pub(crate) fn emit(&mut self, mut instance: Instance, bind: Bind) {
        let state = self.state;
        let layer = &mut self.built.layers[state.layer];
        let area = layer.area();
        let visible = intersect(
            intersect(
                super::super::instance::rect(instance.device_bounds()),
                state.clip,
            ),
            area,
        );
        if !(visible[2] > visible[0] && visible[3] > visible[1]) || state.alpha <= 0. {
            return;
        }
        instance.clip = state.clip;
        instance.extra[0] = state.chain;
        instance.t[3] *= state.alpha;
        if state.layer == 0 {
            layer.visible.push(visible);
        }
        layer.append(instance, bind);
    }
}
