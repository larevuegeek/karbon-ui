//! Traduction d'une scène en couches d'instances et en étapes ordonnées, prêtes à envoyer
//! au GPU.
//!
//! La pile des portées suit `graphics/canvas.rs` : une découpe rectangulaire sous une
//! transformation qui garde les axes devient un rectangle en pixels (scissor du lot et
//! couverture anticrénelée dans le shader) ; une découpe arrondie ou tournée, ou un masque
//! en dégradé, devient un maillon de la chaîne évaluée par le shader. Un effet compose sa
//! transformation ; son opacité s'applique directement à un élément seul, et un groupe de
//! plusieurs éléments est rendu dans une texture intermédiaire limitée à ses bornes,
//! recomposée avec l'opacité ; un contenu d'opacité nulle est sauté (`scopes.rs`). Les
//! groupes filtrés, `backdrop-filter` et les masques sont dans `effects.rs`, le texte
//! rempli dans `text.rs`.
//!
//! Ordre de rendu (`Step`) : une couche est dessinée quand elle est refermée, avant que sa
//! recomposition n'entre dans la couche parente ; la cible finale est dessinée en un seul
//! passage, en dernier. Un `backdrop-filter` ne relit pas la cible : ce qu'elle a reçu
//! jusque-là dans sa zone est redessiné dans une couche de la taille de cette zone
//! (`effects.rs`), ce qui évite de couper le passage de la cible (une écriture et une
//! relecture de toute la fenêtre par coupure).
mod effects;
mod raster;
mod scopes;
mod shapes;
mod text;

use super::super::graphics::Renderer as CgRenderer;
use super::{
    device::{Gpu, Texture},
    instance::{ClipNode, Instance, NO_CLIP, Stop},
    resources::Resources,
};
use crate::ui::{
    rendering::{brush::Background, transform::Affine},
    scene::{DrawCommand, RectF, Scene},
};

/// Texture d'un lot : aucune, une texture de l'image, ou une couche intermédiaire.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Bind {
    None,
    Texture(usize),
    Layer(usize),
}
pub(super) struct Batch {
    pub bind: Bind,
    /// Scissor en pixels de la couche : x, y, largeur, hauteur.
    pub scissor: [usize; 4],
    pub start: usize,
    pub count: usize,
}
/// Cible d'un passage de rendu : la fenêtre (couche 0) ou une texture intermédiaire.
pub(super) struct Layer {
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
    visible: Vec<[f32; 4]>,
}
impl Layer {
    fn new(origin: [i32; 2], size: [usize; 2], clear: [f64; 4]) -> Self {
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
    fn append(&mut self, instance: Instance, bind: Bind) {
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
    fn area(&self) -> [f32; 4] {
        [
            self.origin[0] as f32,
            self.origin[1] as f32,
            (self.origin[0] + self.size[0] as i32) as f32,
            (self.origin[1] + self.size[1] as i32) as f32,
        ]
    }
}
/// Étape de l'image, exécutée dans l'ordre.
pub(super) enum Step {
    /// Passage de rendu de la couche `layer` (effacée, puis tous ses lots).
    Draw(usize),
    /// Passage de filtre.
    Filter(super::filter::Pass),
}
/// Image prête à encoder.
#[derive(Default)]
pub(super) struct Built {
    pub layers: Vec<Layer>,
    pub steps: Vec<Step>,
    pub textures: Vec<Texture>,
    pub stops: Vec<Stop>,
    pub clips: Vec<ClipNode>,
    /// Textures d'images nouvelles dont les niveaux de mipmap restent à calculer.
    pub mipmaps: Vec<Texture>,
    /// Couches vidées d'une image précédente, dont les vecteurs sont réutilisés.
    spare: Vec<Layer>,
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
    fn layer(&mut self, origin: [i32; 2], size: [usize; 2], clear: [f64; 4]) -> Layer {
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
struct State {
    /// Repère logique courant → pixels de la couche (cible finale, ou contenu d'un
    /// groupe filtré).
    transform: Affine,
    alpha: f32,
    /// Découpe rectangulaire en pixels.
    clip: [f32; 4],
    /// Dernier maillon de la chaîne des découpes générales et masques (-1 : aucun).
    chain: i32,
    layer: usize,
}
enum Scope {
    Plain(State),
    /// Groupe rendu dans la couche `child`, recomposé à `rect` avec `opacity`.
    Group {
        saved: State,
        child: usize,
        opacity: f32,
        rect: RectF,
    },
}
pub(super) struct Builder<'a> {
    pub gpu: &'a Gpu,
    pub resources: &'a mut Resources,
    pub cg: &'a mut CgRenderer,
    built: Built,
    state: State,
    scopes: Vec<Scope>,
    /// Profondeur des groupes filtrés en cours (un `backdrop-filter` y est ignoré, comme
    /// dans la couche de filtre de Core Graphics).
    filter_depth: usize,
    /// Remplissages du texte (`background-clip: text`) et leur zone.
    text_fills: Vec<(Background, RectF)>,
    /// Index dans `built.textures` de chaque texture déjà liée (par adresse).
    bound: std::collections::HashMap<usize, usize>,
}
/// Bornes d'un groupe arrondies à 64 pixels pour réutiliser les textures.
fn bucket(n: usize) -> usize {
    n.div_ceil(64) * 64
}
fn intersect(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
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
                    let end = super::super::graphics::scope_end(commands, index - 1);
                    if *opacity <= 1. / 512. {
                        index = end + 1;
                        continue;
                    }
                    let layer =
                        *opacity < 1. && !super::super::graphics::single(&commands[index..end]);
                    if !self.effect(*transform, *opacity, *bounds, layer) {
                        index = end + 1;
                    }
                }
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    let end = super::super::graphics::scope_end(commands, index - 1);
                    self.filter(*id, &commands[index..end], *bounds, *filters)?;
                    index = end + 1;
                }
                other => self.draw(other)?,
            }
        }
        Ok(())
    }
    fn draw(&mut self, command: &DrawCommand) -> crate::Result<()> {
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
                let (rect, radii) = super::super::graphics::normalized(*rect, *radii, None);
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
    pub(super) fn bind(&mut self, texture: &Texture) -> Bind {
        let key = objc2::rc::Retained::as_ptr(texture) as *const () as usize;
        let index = *self.bound.entry(key).or_insert_with(|| {
            self.built.textures.push(texture.clone());
            self.built.textures.len() - 1
        });
        Bind::Texture(index)
    }
    pub(super) fn stops(
        &mut self,
        stops: &[crate::ui::rendering::brush::GradientStop],
    ) -> [u32; 2] {
        let first = self.built.stops.len() as u32;
        self.built.stops.extend(stops.iter().map(|s| Stop {
            color: super::instance::color(s.color),
            position: [s.position, 0., 0., 0.],
        }));
        [first, stops.len() as u32]
    }
    pub(super) fn mipmaps(&mut self, texture: Texture) {
        self.built.mipmaps.push(texture);
    }
    pub(super) fn transform(&self) -> Affine {
        self.state.transform
    }
    /// Ajoute `instance` à la couche courante avec la découpe et l'opacité en cours.
    pub(super) fn emit(&mut self, mut instance: Instance, bind: Bind) {
        let state = self.state;
        let layer = &mut self.built.layers[state.layer];
        let area = layer.area();
        let visible = intersect(
            intersect(super::instance::rect(instance.device_bounds()), state.clip),
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
