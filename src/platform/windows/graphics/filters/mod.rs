//! Cached command lists and native effects. No intermediate GPU-to-CPU readback.
use super::*;
use crate::ui::{
    element::ElementId,
    rendering::filter::{Filter, Filters},
};
use std::collections::HashSet;
mod effect;
use crate::ui::rendering::scene::signature;
#[cfg(test)]
pub(super) mod tests;
struct Entry {
    key: signature::Signature,
    crop: Option<RectF>,
    image: ID2D1Image,
    _source: Scene,
}
/// Taille (pixels physiques) jusqu'à laquelle un groupe dont seule la partie visible
/// change est enregistré entier plutôt que rogné de nouveau à chaque image (4 Mio).
const MOVING_CROP_PIXELS: f32 = 1_048_576.;
#[derive(Default)]
pub(super) struct Cache {
    entries: HashMap<ElementId, Entry>,
    #[cfg(test)]
    pub records: usize,
}
pub(super) fn visible(scene: &Scene) -> Scene {
    let mut depth = 0usize;
    let mut result = Scene::new(scene.clear);
    for c in &scene.commands {
        match c {
            DrawCommand::PushFilter { .. } => {
                if depth == 0 {
                    result.commands.push(c.clone());
                }
                depth += 1;
            }
            DrawCommand::PopFilter => {
                depth -= 1;
                if depth == 0 {
                    result.commands.push(c.clone());
                }
            }
            _ if depth == 0 => result.commands.push(c.clone()),
            _ => {}
        }
    }
    result
}
/// Chaîne d'effets natifs appliquée à `image` (réutilisée par `backdrop-filter`).
pub(super) fn effect_chain(
    context: &ID2D1DeviceContext,
    image: ID2D1Image,
    filters: Filters,
) -> crate::Result<ID2D1Image> {
    effect::chain(context, image, filters, None)
}
impl Cache {
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn prepare(
        &mut self,
        device: &ID2D1Device,
        factory: &ID2D1Factory,
        scene: &Scene,
        formats: &HashMap<(u32, Font), IDWriteTextFormat>,
        typography: &typography::Cache,
        scale: f32,
    ) -> crate::Result<()> {
        let mut stack = Vec::new();
        let mut groups = Vec::new();
        let mut used = HashSet::new();
        // Clips (in window coordinates) and transforms that enclose each group: only the
        // visible part of its output is computed and cached.
        let mut transforms = vec![Affine::IDENTITY];
        let mut clips: Vec<Option<RectF>> = vec![None];
        for (index, c) in scene.commands.iter().enumerate() {
            let transform = *transforms.last().unwrap();
            let clip = *clips.last().unwrap();
            match c {
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    let visible = clip.and_then(|clip| Some(transform.inverse()?.bounds(clip)));
                    stack.push((*id, index, *bounds, *filters, visible));
                }
                DrawCommand::PopFilter => {
                    let (id, start, bounds, filters, visible) = stack
                        .pop()
                        .ok_or_else(|| crate::Error::invalid_input("unbalanced native filter"))?;
                    groups.push((id, start, index, bounds, filters, visible));
                }
                DrawCommand::PushClip(rect) | DrawCommand::PushShapeClip { rect, .. } => {
                    let world = transform.bounds(*rect);
                    clips.push(Some(
                        clip.map_or(world, |c| crate::ui::layout::intersect(c, world)),
                    ));
                }
                DrawCommand::PopClip | DrawCommand::PopShapeClip if clips.len() > 1 => {
                    clips.pop();
                }
                DrawCommand::PushEffect { transform: t, .. } => transforms.push(t.then(transform)),
                DrawCommand::PopEffect if transforms.len() > 1 => {
                    transforms.pop();
                }
                _ => {}
            }
        }
        used.extend(groups.iter().map(|g| g.0));
        // Prune before recording, so repeated failures cannot retain obsolete groups.
        self.entries.retain(|id, _| used.contains(id));
        for (id, start, end, bounds, filters, shown) in groups {
            let output_bounds = filters.bounds(bounds);
            // Only the visible part of the output (a blurred background under
            // `overflow: hidden` is cropped by its card) is computed and cached.
            let crop = shown
                .map(|v| crate::ui::layout::intersect(output_bounds, v))
                .filter(|c| *c != output_bounds);
            let pixels = (output_bounds.right - output_bounds.left).max(0.)
                * scale
                * (output_bounds.bottom - output_bounds.top).max(0.)
                * scale;
            if !pixels.is_finite() || pixels > 16_777_216. {
                return Err(crate::Error::invalid_input(
                    "filter group exceeds its 16M physical pixel limit",
                ));
            }
            let commands = &scene.commands[start + 1..end];
            let key = signature::Signature::new(commands, bounds, filters, scale);
            let crop = match self.entries.get(&id) {
                Some(e) if e.key == key && e.crop == crop => continue,
                // Seule la partie visible a changé : contenu déplacé sous un ancêtre qui le
                // rogne (diapo qui glisse dans un carrousel), à chaque image. Une sortie
                // entière déjà enregistrée la couvre ; une sortie modeste est enregistrée
                // entière, une seule fois, au lieu d'une liste et d'effets par image.
                Some(e) if e.key == key && e.crop.is_none() => continue,
                Some(e) if e.key == key && pixels <= MOVING_CROP_PIXELS => None,
                _ => crop,
            };
            let source = Scene {
                clear: Color::rgba(0, 0, 0, 0),
                commands: commands.to_vec(),
            };
            let visible = visible(&source);
            let mut canvas = Canvas::new(device::context(device)?)?;
            canvas.prepare(factory, &visible)?;
            let context = &canvas.target;
            // SAFETY: contexte Direct2D valide, hors passe de dessin.
            let list = unsafe { context.CreateCommandList() }.check("CreateCommandList")?;
            let mut scopes = Vec::new();
            // SAFETY: la liste devient la cible le temps de l'enregistrement ; EndDraw ci-dessous
            // referme toujours la passe ouverte ici.
            unsafe {
                context.SetTarget(&list);
                context.SetDpi(96. * scale, 96. * scale);
                context.BeginDraw();
            }
            let result = canvas.draw(&visible, factory, formats, typography, &mut scopes, self);
            while let Some(scope) = scopes.pop() {
                effects::pop(context, scope);
            }
            // SAFETY: referme la passe ouverte ci-dessus, puis détache la liste du contexte.
            let status = unsafe {
                let status = context.EndDraw(None, None);
                context.SetTarget(None);
                status
            };
            result?;
            status.check("Finish filter command list")?;
            // SAFETY: liste enregistrée et détachée, fermée une seule fois.
            unsafe { list.Close() }.check("CloseCommandList")?;
            let image = effect::chain(context, list.into(), filters, crop)?;
            self.entries.insert(
                id,
                Entry {
                    key,
                    crop,
                    image,
                    _source: source,
                },
            );
            #[cfg(test)]
            {
                self.records += 1;
            }
        }
        self.entries.retain(|id, _| used.contains(id));
        Ok(())
    }
    pub fn draw(&self, target: &ID2D1DeviceContext, id: ElementId) {
        let image = &self.entries[&id].image;
        // SAFETY: passe de dessin ouverte ; image du cache vivante.
        unsafe {
            target.DrawImage(
                image,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );
        }
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
