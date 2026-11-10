pub(super) use super::super::{
    super::graphics::Renderer as Cairo,
    filter::{Params, Pass},
    memory::Image,
    resources::{Kept, Resources},
};
pub(super) use crate::{
    platform::shared::{
        gpu::{ClipNode, Instance, NO_CLIP, Stop, rect as array},
        normalized, scope_end, single,
    },
    ui::{
        rendering::{brush::Background, transform::Affine},
        scene::{DrawCommand, RectF, Scene},
    },
};

/// Texture d'un lot : aucune, une vue vivante pendant toute l'image (page de l'atlas,
/// image, rastérisation), une couche intermédiaire ou une cible de filtre.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Bind {
    None,
    Texture(u64),
    Layer(usize),
    Target(usize),
}
pub(crate) struct Batch {
    pub bind: Bind,
    /// Ciseau en pixels de la couche : x, y, largeur, hauteur.
    pub scissor: [usize; 4],
    pub start: usize,
    pub count: usize,
}
/// Cible d'un passage de rendu : la fenêtre (couche 0) ou une couche intermédiaire.
pub(crate) struct Layer {
    /// Coin de la couche dans les pixels de la cible finale.
    pub origin: [i32; 2],
    /// Taille utile, et taille de la texture (arrondie pour être réutilisée).
    pub size: [usize; 2],
    pub texture_size: [usize; 2],
    /// Couleur d'effacement prémultipliée.
    pub clear: [f32; 4],
    pub instances: Vec<Instance>,
    pub batches: Vec<Batch>,
    /// Zone visible (pixels) de chaque instance de la cible finale, relue par
    /// `backdrop-filter`.
    pub(super) visible: Vec<[f32; 4]>,
}
impl Layer {
    pub(super) fn new(origin: [i32; 2], size: [usize; 2], clear: [f32; 4]) -> Self {
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
    /// même ciseau, sinon à un nouveau lot.
    pub(super) fn append(&mut self, instance: Instance, bind: Bind) {
        // Ciseau : bornes de la découpe arrondies vers l'extérieur, en pixels de la couche.
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
    /// Zone de la couche dans les pixels de la cible finale.
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
#[derive(Debug)]
pub(crate) enum Step {
    /// Passage de rendu de la couche (effacée, puis tous ses lots).
    Draw(usize),
    /// Passage de filtre.
    Filter(Pass),
}
/// Cible d'un passage de filtre (`filter::FORMAT`).
pub(crate) struct Target {
    /// Taille de la texture ; la zone utile de chaque passage tient dedans.
    pub size: [usize; 2],
    /// Image déjà prête (résultat gardé d'un groupe filtré) ; sinon prise dans la réserve
    /// à l'exécution.
    pub image: Option<Image>,
    /// Résultat d'un groupe filtré, rendu aux ressources après l'image.
    pub keep: Option<Kept>,
}
/// Image prête à envoyer.
#[derive(Default)]
pub(crate) struct Built {
    pub layers: Vec<Layer>,
    pub steps: Vec<Step>,
    pub stops: Vec<Stop>,
    pub clips: Vec<ClipNode>,
    pub targets: Vec<Target>,
    /// Paramètres des passages de filtre.
    pub params: Vec<Params>,
}
impl Built {
    /// Cible intermédiaire de filtre d'au moins `size` pixels (arrondis pour être
    /// réutilisée).
    pub(crate) fn scratch(&mut self, size: [usize; 2]) -> usize {
        self.targets.push(Target {
            size: [bucket(size[0]), bucket(size[1])],
            image: None,
            keep: None,
        });
        self.targets.len() - 1
    }
}
impl Built {
    /// Image d'une seule couche de `width` × `height` : `instances` en un lot sans texture
    /// (tests du shader).
    #[cfg(test)]
    pub fn single(
        (width, height): (usize, usize),
        clear: [f32; 4],
        instances: &[Instance],
        stops: &[Stop],
        clips: &[ClipNode],
    ) -> Self {
        let mut layer = Layer::new([0, 0], [width, height], clear);
        layer.texture_size = [width, height];
        layer.instances = instances.to_vec();
        layer.batches.push(Batch {
            bind: Bind::None,
            scissor: [0, 0, width, height],
            start: 0,
            count: instances.len(),
        });
        Self {
            layers: vec![layer],
            steps: vec![Step::Draw(0)],
            stops: stops.to_vec(),
            clips: clips.to_vec(),
            ..Default::default()
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct State {
    /// Repère logique courant → pixels de la cible finale.
    pub(super) transform: Affine,
    pub(super) alpha: f32,
    /// Découpe rectangulaire en pixels.
    pub(super) clip: [f32; 4],
    /// Dernier maillon de la chaîne des découpes générales (-1 : aucun).
    pub(super) chain: i32,
    pub(super) layer: usize,
}
pub(super) enum Scope {
    Plain(State),
    /// Groupe rendu dans la couche `child`, recomposé à `rect` avec `opacity`, et masqué
    /// par l'alpha du dégradé `mask` posé sur ses bornes (`mask-image`).
    Group {
        saved: State,
        child: usize,
        opacity: f32,
        rect: RectF,
        mask: Option<(
            RectF,
            std::sync::Arc<crate::ui::rendering::brush::LinearGradient>,
        )>,
    },
}
pub(crate) struct Builder<'a> {
    pub resources: &'a mut Resources,
    /// Mises en page du texte, partagées avec le dessin Cairo.
    pub cairo: &'a mut Cairo,
    pub(super) built: Built,
    pub(super) state: State,
    pub(super) scopes: Vec<Scope>,
    /// Profondeur des groupes filtrés en cours (un `backdrop-filter` y est ignoré, comme
    /// dans la couche de filtre de Cairo).
    pub(super) filter_depth: usize,
    /// Remplissages du texte (`background-clip: text`) et leur zone.
    pub(super) text_fills: Vec<(Background, RectF)>,
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
    /// Traduction de `scene` vers une cible de `width` × `height` pixels à `scale`.
    pub fn new(
        resources: &'a mut Resources,
        cairo: &'a mut Cairo,
        scene: &Scene,
        (width, height): (usize, usize),
        scale: f32,
    ) -> Self {
        let c = scene.clear;
        let a = f32::from(c.alpha) / 255.;
        let mut root = Layer::new(
            [0, 0],
            [width, height],
            [
                f32::from(c.red) / 255. * a,
                f32::from(c.green) / 255. * a,
                f32::from(c.blue) / 255. * a,
                a,
            ],
        );
        root.texture_size = [width, height];
        let transform = Affine {
            m11: scale,
            m22: scale,
            ..Affine::IDENTITY
        };
        Self {
            resources,
            cairo,
            built: Built {
                layers: vec![root],
                ..Default::default()
            },
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
                    let end = scope_end(commands, index - 1);
                    if *opacity <= 1. / 512. {
                        index = end + 1;
                        continue;
                    }
                    let layer = *opacity < 1. && !single(&commands[index..end]);
                    if !self.effect(*transform, *opacity, *bounds, layer) {
                        index = end + 1;
                    }
                }
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    let end = scope_end(commands, index - 1);
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
                let (rect, radii) = normalized(*rect, *radii, None);
                if radii.iter().all(|r| *r <= 0.) {
                    self.clip_rect(rect);
                } else {
                    self.clip_chain(rect, radii);
                }
            }
            DrawCommand::PushMask { bounds, gradient } => self.mask_group(*bounds, gradient),
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
    pub(crate) fn stops(
        &mut self,
        stops: &[crate::ui::rendering::brush::GradientStop],
    ) -> [u32; 2] {
        let first = self.built.stops.len() as u32;
        self.built.stops.extend(stops.iter().map(|s| Stop {
            color: crate::platform::shared::gpu::color(s.color),
            position: [s.position, 0., 0., 0.],
        }));
        [first, stops.len() as u32]
    }
    pub(crate) fn transform(&self) -> Affine {
        self.state.transform
    }
    /// Ajoute `instance` à la couche courante avec la découpe et l'opacité en cours.
    pub(crate) fn emit(&mut self, mut instance: Instance, bind: Bind) {
        let state = self.state;
        let layer = &mut self.built.layers[state.layer];
        let visible = intersect(
            intersect(array(instance.device_bounds()), state.clip),
            layer.area(),
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
