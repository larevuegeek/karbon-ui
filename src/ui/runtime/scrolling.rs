use super::*;
/// Pendant un défilement, le survol suit le contenu au plus à cet intervalle (faux
/// mouvements de souris de Chromium pendant un défilement) : les lignes qui passent sous
/// un pointeur immobile ne s'allument pas l'une après l'autre.
const HOVER_DURING_SCROLL: std::time::Duration = std::time::Duration::from_millis(100);
impl UiRuntime {
    /// Survol après un déplacement du contenu sous un pointeur immobile (mise en page,
    /// animation) ; figé pendant un défilement, voir [`HOVER_DURING_SCROLL`].
    pub(super) fn refresh_hover(&mut self) {
        if self.interaction.hover_frozen.is_none() {
            self.interaction.hovered = self.interaction.pointer.and_then(|p| self.hit(p));
        }
    }

    /// Élément survolé (tests).
    #[cfg(test)]
    pub(crate) fn hovered_id(&self) -> Option<ElementId> {
        self.interaction.hovered.map(Hit::id)
    }

    /// Fin du gel du survol : il rejoint le contenu sous le pointeur.
    pub(super) fn thaw_hover(&mut self, now: Instant) -> bool {
        if self.interaction.hover_frozen.is_none_or(|t| t > now) {
            return false;
        }
        self.interaction.hover_frozen = None;
        let before = self.interaction.hovered.map(Hit::id);
        self.refresh_hover();
        self.interaction.hovered.map(Hit::id) != before
    }

    /// Un défilement est en cours : glissement de la molette, contenu qui vient de défiler
    /// sous le pointeur, pouce tiré (l'accessibilité Windows attend qu'il s'arrête).
    /// Une reconstruction de la vue attend la prochaine préparation.
    #[cfg(target_os = "windows")]
    pub(crate) fn rebuild_pending(&self) -> bool {
        self.invalidation.rebuild
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn is_scrolling(&self) -> bool {
        self.components.scrolls.smooth.is_some()
            || self.components.scrolls.drag.is_some()
            || self.interaction.hover_frozen.is_some()
    }

    pub(super) fn scroll_to(&mut self, id: ElementId, offset: f32) -> f32 {
        let Some(node) = self.root.as_mut().and_then(|r| r.find_mut(id)) else {
            return 0.;
        };
        let delta = scroll::set_offset(node, offset);
        if delta != 0. {
            // Événement `scroll` : la géométrie à jour, émise après le déplacement.
            let notify = node.on_scroll.as_deref().cloned().zip(node.scroll);
            self.components
                .scrolls
                .offsets
                .insert(id, node.scroll_offset);
            if self.interaction.pointer.is_some() && self.interaction.hover_frozen.is_none() {
                self.interaction.hover_frozen = Some(Instant::now() + HOVER_DURING_SCROLL);
            }
            self.invalidation.repaint = true;
            self.invalidation.containment_check |= self.invalidation.containment.present;
            self.invalidation.stats.scrolls += 1;
            if let Some((handler, metrics)) = notify {
                self.send(handler.emit(metrics));
            }
        }
        delta
    }

    /// Contenu défilé : les éléments `content-visibility: auto` qui s'approchent de la vue
    /// sont disposés sur place (leur chemin au layout suivant si leur taille change) ;
    /// ceux qui s'en éloignent cessent d'être peints. `window` : zone de la fenêtre.
    /// `true` si un contenu vient d'être rendu.
    pub(super) fn reveal_contained(
        &mut self,
        context: ViewContext,
        chrome: ChromeState,
        text: &mut dyn TextMeasurer,
        now: Instant,
    ) -> crate::Result<bool> {
        let window = super::prepare::window_area(context, chrome);
        let Some(root) = self.root.as_mut() else {
            return Ok(false);
        };
        let mut state = std::mem::take(&mut self.invalidation.containment);
        let near = layout::containment::refresh(&mut state, root, window, self.interaction.focused);
        layout::containment::enter(state);
        // Styles du contenu rendu (sautés jusque-là), puis sa disposition.
        let styled = self.resolve_revealed(&near, context, now);
        if let Err(error) = styled {
            self.invalidation.containment = layout::containment::leave();
            return Err(error);
        }
        let Some(root) = self.root.as_mut() else {
            self.invalidation.containment = layout::containment::leave();
            return Ok(false);
        };
        let pass = layout::LayoutPass::begin(root, false);
        let mut moved = Vec::new();
        let mut result = Ok(());
        for &id in &near {
            let Some(node) = root.find_mut(id) else {
                continue;
            };
            match layout::containment::arrange_near(node, text) {
                Ok(true) => {}
                Ok(false) => moved.push(id),
                Err(error) => {
                    result = Err(error);
                    break;
                }
            }
        }
        drop(pass);
        self.invalidation.containment = layout::containment::leave();
        result?;
        let Some(root) = self.root.as_mut() else {
            return Ok(false);
        };
        if layout::containment::unsettle(root, &moved) {
            self.invalidation.reflow = true;
        }
        if !near.is_empty() {
            self.refresh_hover();
        }
        self.invalidation.repaint = true;
        Ok(!near.is_empty())
    }

    pub(super) fn scroll_ancestors(&self, target: ElementId) -> Vec<ElementId> {
        fn walk(node: &Element, target: ElementId, ids: &mut Vec<ElementId>) -> Option<bool> {
            let stop = if node.id == target {
                false
            } else {
                node.children.iter().find_map(|c| walk(c, target, ids))?
            };
            if !stop && node.scroll.is_some() {
                ids.push(node.id);
            }
            Some(stop || layout::fixed(node))
        }
        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            walk(root, target, &mut ids);
        }
        ids
    }

    pub(super) fn reveal_focus(&mut self) {
        let Some(id) = self.interaction.focused else {
            return;
        };
        self.reveal_element(id);
    }

    pub(super) fn reveal_element(&mut self, id: ElementId) {
        for ancestor in self.scroll_ancestors(id) {
            if ancestor == id {
                continue;
            }
            let root = self.root.as_ref().unwrap();
            let target = root.find(id).unwrap();
            let container = root.find(ancestor).unwrap();
            let s = container.scroll.unwrap();
            // `scroll-margin` de l'élément et `scroll-padding` du conteneur.
            let margin = target
                .rules
                .as_ref()
                .map_or(Default::default(), |r| r.scroll_margin);
            let padding = container
                .rules
                .as_ref()
                .map_or(Default::default(), |r| r.scroll_padding);
            let (before, after) = match s.axis {
                Axis::Vertical => (margin.top + padding.top, margin.bottom + padding.bottom),
                Axis::Horizontal => (margin.left + padding.left, margin.right + padding.right),
            };
            let bounds = target.rect;
            let start = scroll::axis_start(bounds, s.axis) - before;
            let view_start = scroll::axis_start(s.viewport, s.axis);
            let end =
                scroll::axis_start(bounds, s.axis) + scroll::axis_size(bounds, s.axis) + after;
            let view_end = view_start + s.viewport_size();
            let delta = if start < view_start {
                start - view_start
            } else if end > view_end {
                (end - view_end).min(start - view_start)
            } else {
                0.
            };
            self.scroll_to(ancestor, s.offset + delta);
        }
    }

    pub(super) fn scroll_key(&mut self, key: Key) -> bool {
        let Some(id) = self.interaction.focused else {
            return false;
        };
        for ancestor in self.scroll_ancestors(id) {
            let s = self
                .root
                .as_ref()
                .unwrap()
                .find(ancestor)
                .unwrap()
                .scroll
                .unwrap();
            let delta = match key {
                Key::Home => -s.offset,
                Key::End => s.max_offset() - s.offset,
                Key::PageUp => -s.viewport_size() * 0.9,
                Key::PageDown => s.viewport_size() * 0.9,
                Key::Up if s.axis == Axis::Vertical => -40.,
                Key::Down if s.axis == Axis::Vertical => 40.,
                Key::Left if s.axis == Axis::Horizontal => -40.,
                Key::Right if s.axis == Axis::Horizontal => 40.,
                _ => continue,
            };
            if self.scroll_to(ancestor, s.offset + delta) != 0. {
                return true;
            }
            // Le viewport lui-même consomme la touche même à une extrémité.
            if ancestor == id {
                return true;
            }
        }
        false
    }
}
