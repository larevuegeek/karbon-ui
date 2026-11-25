//! Redessin partiel de la chaîne d'échange (KUI-055) : seules les zones qui changent
//! sont redessinées, et seules les commandes qui les touchent sont rejouées ; la
//! présentation déclare ces zones à DWM (`Present1`).
//!
//! La chaîne a deux tampons, en modèle flip séquentiel (le contenu d'un tampon est
//! gardé) : le tampon où l'on dessine l'image N contient l'image N − 2. Les zones à y
//! redessiner sont donc celles qui ont changé de N − 2 à N − 1 et de N − 1 à N. Après une
//! création, un redimensionnement ou une image dessinée en entier par l'autre chemin,
//! les deux images suivantes sont dessinées en entier.
use super::frame::Pass;
use super::*;
pub(super) use crate::platform::shared::damage::Pixels;
use crate::platform::shared::damage::{self, Change, Damage};

/// Ce que le redessin partiel retient d'une image à l'autre.
#[derive(Default)]
pub(crate) struct State {
    /// Inventaire de la dernière image présentée.
    previous: Option<Damage>,
    /// Zones qui ont changé à la dernière image (`None` : toute la surface, ou inconnu).
    prior: Option<Vec<Pixels>>,
    /// Zones à déclarer à la présentation qui suit (`None` : toute l'image).
    pending: Option<Vec<Pixels>>,
    /// Pixels touchés par chaque commande de la scène (réutilisé d'une image à l'autre).
    places: Vec<Option<Pixels>>,
    /// Pixels redessinés depuis le début, et pixels de toutes les images (mesure).
    pub(crate) painted: u64,
    pub(crate) offered: u64,
}

impl State {
    /// Le contenu des tampons n'est plus connu : les deux images suivantes sont entières.
    pub(super) fn forget(&mut self) {
        self.previous = None;
        self.prior = None;
        self.pending = None;
    }
}

/// Rectangle en unités de mise en page (DIP) d'une zone en pixels, à `scale`.
pub(super) fn dips(zone: Pixels, scale: f32) -> RectF {
    RectF {
        left: zone[0] as f32 / scale,
        top: zone[1] as f32 / scale,
        right: zone[2] as f32 / scale,
        bottom: zone[3] as f32 / scale,
    }
}

fn overlaps(a: Pixels, b: Pixels) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn area(zones: &[Pixels]) -> u64 {
    zones
        .iter()
        .map(|z| (z[2] - z[0]).max(0) as u64 * (z[3] - z[1]).max(0) as u64)
        .sum()
}

/// Zones à redessiner dans le tampon (qui contient l'image N − 2) : celles de l'image
/// et celles de la précédente ; `None` : tout.
fn to_redraw(this: Option<&Vec<Pixels>>, prior: Option<&Vec<Pixels>>) -> Option<Vec<Pixels>> {
    let mut zones = this?.clone();
    zones.extend_from_slice(prior?);
    damage::merge(&mut zones);
    Some(zones)
}

/// Les pixels des commandes de la scène dessinée (`visible`, où un groupe filtré n'est
/// plus que ses deux bornes) d'après ceux de la scène entière.
fn visible_places(scene: &Scene, places: &[Option<Pixels>]) -> Vec<Option<Pixels>> {
    let mut depth = 0usize;
    let mut result = Vec::new();
    for (command, place) in scene.commands.iter().zip(places) {
        match command {
            DrawCommand::PushFilter { .. } => {
                if depth == 0 {
                    result.push(*place);
                }
                depth += 1;
            }
            DrawCommand::PopFilter => {
                depth -= 1;
                if depth == 0 {
                    result.push(None);
                }
            }
            _ if depth == 0 => result.push(*place),
            _ => {}
        }
    }
    result
}

/// Pour chaque zone, les commandes à rejouer : celles qui la touchent, et celles dont on
/// ne sait rien (portées, gabarits de surface partagée, qui doivent se placer).
fn masks(zones: &[Pixels], places: &[Option<Pixels>]) -> Vec<(Pixels, Vec<bool>)> {
    zones
        .iter()
        .map(|&zone| {
            let keep = places
                .iter()
                .map(|place| place.is_none_or(|p| overlaps(p, zone)))
                .collect();
            (zone, keep)
        })
        .collect()
}

impl Renderer {
    /// Comme [`render_to_swap_chain`](Self::render_to_swap_chain), en ne redessinant que
    /// ce qui a changé ; la [`present`](Self::present) qui suit déclare ces zones.
    pub(in crate::platform::windows) fn render_changes_to_swap_chain(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
        premultiplied: bool,
    ) -> crate::Result<()> {
        let kind = Kind::SwapChain { premultiplied };
        let visible = self.prepare_scene(scene, width, height, scale, kind)?;
        let mut places = std::mem::take(&mut self.partial.places);
        let size = (width as usize, height as usize);
        let ink = &mut typography::Ink(&self.typography);
        let current = Damage::inventory(scene, scale, size, ink, Some(&mut places))?;
        let mut templates = false;
        for (command, place) in scene.commands.iter().zip(&mut places) {
            if let DrawCommand::Image { image, .. } = command
                && crate::ui::shared_surface::surface_of(image).is_some()
            {
                *place = None;
                templates = true;
            }
        }
        let this = match self.partial.previous.as_ref().map(|p| current.change(p)) {
            Some(Change::None) => Some(Vec::new()),
            Some(Change::Region(zones)) => Some(zones),
            Some(Change::Full) | None => None,
        };
        let whole = u64::from(width) * u64::from(height);
        let mut redraw = to_redraw(this.as_ref(), self.partial.prior.as_ref());
        // Rien à redessiner, mais des gabarits de surface partagée : ils sont rejoués sous
        // une découpe vide, pour que leurs visuels restent à leur place (sans quoi la
        // fin de la peinture les croirait disparus).
        if templates && redraw.as_ref().is_some_and(Vec::is_empty) {
            redraw = Some(vec![[0, 0, 0, 0]]);
        }
        let result = match &redraw {
            None => {
                self.partial.painted += whole;
                self.paint_scene(visible.as_ref().unwrap_or(scene), scene, Pass::Whole)
            }
            Some(zones) if zones.is_empty() => Ok(()),
            Some(zones) => {
                self.partial.painted += area(zones);
                let drawn = visible.as_ref().unwrap_or(scene);
                let places = match &visible {
                    Some(visible) => visible_places(scene, &places)
                        .into_iter()
                        .take(visible.commands.len())
                        .collect(),
                    None => places.clone(),
                };
                let zones = masks(zones, &places);
                self.paint_scene(drawn, scene, Pass::Zones(&zones))
            }
        };
        self.partial.offered += whole;
        self.partial.places = places;
        if result.is_err() {
            self.partial.forget();
            return result;
        }
        self.partial.previous = Some(current);
        self.partial.prior = this.clone();
        self.partial.pending = this;
        Ok(())
    }
    /// Zones que la prochaine présentation déclare (`None` : toute l'image), remises à
    /// zéro : une image dessinée en entier par l'autre chemin rend les tampons inconnus.
    pub(super) fn take_pending_region(&mut self) -> Option<Vec<Pixels>> {
        self.partial.pending.take()
    }
    /// Part des pixels redessinés par le redessin partiel depuis le début (mesure).
    pub(in crate::platform::windows) fn partial_share(&self) -> Option<f64> {
        (self.partial.offered > 0)
            .then(|| self.partial.painted as f64 / self.partial.offered as f64)
    }
}
