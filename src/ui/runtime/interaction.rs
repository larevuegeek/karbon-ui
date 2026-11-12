//! État d'interaction : pointeur, survol, appui, focus clavier et ordre de tabulation.
use super::*;

pub(super) struct Interaction {
    pub(super) hovered: Option<Hit>,
    pub(super) pointer: Option<PointF>,
    pub(super) pressed: Option<ElementId>,
    pub(super) focused: Option<ElementId>,
    pub(super) keyboard_pressed: Option<ElementId>,
    pub(super) focus_visible: bool,
    /// Capture perdue hors d'un événement : signalée au prochain `EventResult`.
    pub(super) release_pending: bool,
    pub(super) focus_order: Vec<ElementId>,
    /// Nombre d'éléments de la dernière indexation (capacité de la suivante).
    live_hint: usize,
    /// Tabulation différée jusqu'à la reconstruction qui ferme un popover.
    pub(super) pending_tab: Option<bool>,
    /// Échap déjà traité : ses répétitions et son relâchement ne remontent pas à l'hôte.
    pub(super) escape_consumed: bool,
    /// Appui hors d'une modale ou d'un popover, résolu au relâchement.
    pub(super) backdrop_capture: Option<ElementId>,
    /// Appui hors d'un popover qui l'a fermé : rejoué sous lui une fois la vue
    /// reconstruite (voir `prepare_at`).
    pub(super) replay: Option<UiEvent>,
    /// Rejeu en cours : un nouvel appui hors d'un popover encore ouvert est consommé.
    pub(super) replaying: bool,
    /// La capture prise pour un appui rejoué est rendue au relâchement.
    pub(super) replay_release: bool,
    pub(super) layer_focus: LayerFocus,
    pub(super) window_active: bool,
    /// Les bulles suivent le focus clavier plutôt que le pointeur.
    pub(super) tooltip_keyboard: bool,
    /// Survol figé pendant un défilement sous un pointeur immobile, jusqu'à cet instant
    /// (voir `UiRuntime::refresh_hover`).
    pub(super) hover_frozen: Option<Instant>,
}
impl Default for Interaction {
    fn default() -> Self {
        Self {
            hovered: None,
            pointer: None,
            pressed: None,
            focused: None,
            keyboard_pressed: None,
            focus_visible: false,
            release_pending: false,
            focus_order: Vec::new(),
            live_hint: 0,
            pending_tab: None,
            escape_consumed: false,
            backdrop_capture: None,
            replay: None,
            replaying: false,
            replay_release: false,
            layer_focus: LayerFocus::default(),
            window_active: true,
            tooltip_keyboard: false,
            hover_frozen: None,
        }
    }
}
impl Interaction {
    /// Capture détenue par l'interaction elle-même (appui ou fond de couche).
    pub(super) fn captured(&self) -> bool {
        self.backdrop_capture.is_some()
            || self.pressed.is_some()
            || self.replay.is_some()
            || self.replay_release
    }

    /// Élément enfoncé, au pointeur ou au clavier.
    pub(super) fn active_press(&self) -> Option<ElementId> {
        self.pressed.or(self.keyboard_pressed)
    }

    /// Focus déplacé au clavier : anneau visible et appui clavier abandonné.
    pub(super) fn focus_keyboard(&mut self, id: ElementId) {
        self.focused = Some(id);
        self.focus_visible = true;
        self.keyboard_pressed = None;
    }

    pub(super) fn paint_state(&self) -> PaintState {
        PaintState {
            hovered: self.hovered.map(Hit::id),
            pressed: self.pressed,
            focused: self.focused,
            keyboard_pressed: self.keyboard_pressed,
            focus_visible: self.focus_visible,
        }
    }

    pub(super) fn style_signature(&self, input_revision: u64) -> StyleSignature {
        StyleSignature {
            input_revision,
            focus_visible: self.focus_visible,
            pointer: self.pointer.map(|p| (p.x.to_bits(), p.y.to_bits())),
            pressed: self.active_press(),
            focused: self.focused,
        }
    }

    /// Ancre des bulles : le pointeur, ou le focus visible en mode clavier.
    pub(super) fn tooltip_anchor(&self) -> (Option<PointF>, Option<ElementId>) {
        (
            self.pointer.filter(|_| !self.tooltip_keyboard),
            self.focused
                .filter(|_| self.focus_visible && self.tooltip_keyboard),
        )
    }

    /// Avaler les répétitions et le relâchement d'un Échap déjà consommé.
    pub(super) fn swallow_escape(&mut self, event: &UiEvent) -> bool {
        match event {
            UiEvent::KeyDown {
                key: Key::Escape,
                repeat: true,
                ..
            } if self.escape_consumed => return true,
            UiEvent::KeyDown {
                key: Key::Escape,
                repeat: false,
                ..
            } => self.escape_consumed = false,
            UiEvent::KeyUp(Key::Escape) if self.escape_consumed => {
                self.escape_consumed = false;
                return true;
            }
            UiEvent::FocusChanged(false) => self.escape_consumed = false,
            _ => {}
        }
        false
    }

    /// Réindexer l'arbre : ordre de focus, états restaurés et focus de couche.
    /// Renvoie les IDs visités.
    pub(super) fn index_tree(
        &mut self,
        root: &mut Element,
        states: &ComponentStates,
    ) -> crate::Result<FastSet<ElementId>> {
        let mut live = FastSet::with_capacity_and_hasher(self.live_hint, Default::default());
        let mut duplicate = None;
        self.focus_order.clear();
        visit_tree(
            root,
            &mut live,
            &mut duplicate,
            &mut self.focus_order,
            states,
            self.focused,
        );
        // Vérifié ici et non par `layout::validate`, qui ne descend pas dans les sous-arbres
        // repris : ce parcours-ci voit de toute façon tout l'arbre.
        if let Some(id) = duplicate {
            return Err(layout::duplicate_id(id));
        }
        self.live_hint = live.len();
        self.focused = self
            .layer_focus
            .sync(root, &mut self.focus_order, self.focused);
        Ok(live)
    }

    /// Après reconstruction : oublier le focus d'un élément devenu inaccessible et l'appui
    /// d'un élément disparu. Un bouton hors de l'ordre de tabulation (`tabindex="-1"`)
    /// garde son appui : le relâchement revérifie de toute façon la cible touchée.
    pub(super) fn retain_focusable(&mut self, live: &FastSet<ElementId>) {
        if self
            .focused
            .is_some_and(|id| !self.focus_order.contains(&id))
        {
            self.focused = None;
            self.keyboard_pressed = None;
        }
        if self.pressed.is_some_and(|id| !live.contains(&id)) {
            self.pressed = None;
            self.release_pending = true;
        }
    }

    pub(super) fn tab_order(&self, root: Option<&Element>) -> Vec<ElementId> {
        let mut order = self.focus_order.clone();
        if let Some(root) = root {
            // Sous-arbres `display: none` : hors de l'ordre de tabulation.
            fn hidden(node: &Element, out: &mut Vec<ElementId>, inside: bool) {
                let inside = inside || crate::ui::layout::hidden(node);
                if inside {
                    out.push(node.id);
                }
                for child in &node.children {
                    hidden(child, out, inside);
                }
            }
            let mut skipped = Vec::new();
            hidden(root, &mut skipped, false);
            if !skipped.is_empty() {
                order.retain(|id| !skipped.contains(id));
            }
            for group in crate::ui::choices::radio_groups(root).unwrap_or_default() {
                let enabled: Vec<_> = group
                    .members
                    .iter()
                    .filter(|m| m.enabled && self.focus_order.contains(&m.id))
                    .collect();
                let active = enabled
                    .iter()
                    .find(|m| Some(m.id) == self.focused)
                    .or_else(|| enabled.iter().find(|m| m.selected))
                    .or_else(|| enabled.first())
                    .map(|m| m.id);
                order
                    .retain(|id| !group.members.iter().any(|m| m.id == *id) || Some(*id) == active);
            }
        }
        order
    }

    /// Cible de Tab / Maj+Tab depuis le focus courant.
    pub(super) fn tab_target(&self, root: Option<&Element>, shift: bool) -> Option<ElementId> {
        let order = self.tab_order(root);
        if order.is_empty() {
            return None;
        }
        let n = order.len();
        let current = self
            .focused
            .and_then(|id| order.iter().position(|x| *x == id));
        let next = match current {
            Some(i) if shift => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
            None if shift => n - 1,
            None => 0,
        };
        Some(order[next])
    }

    /// Bouton radio voisin atteint par une flèche, Début ou Fin dans le groupe focalisé.
    pub(super) fn radio_target(&self, root: Option<&Element>, key: Key) -> Option<ElementId> {
        if !matches!(
            key,
            Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End
        ) {
            return None;
        }
        let id = self.focused?;
        let root = root?;
        if !root
            .find(id)
            .and_then(|n| n.widget.as_ref())
            .is_some_and(|w| matches!(w.choice(), Some(ChoiceState::Radio(_))))
        {
            return None;
        }
        let group = crate::ui::choices::radio_groups(root)
            .unwrap_or_default()
            .into_iter()
            .find(|g| g.members.iter().any(|m| m.id == id))?;
        let members: Vec<_> = group
            .members
            .iter()
            .filter(|m| m.enabled && self.focus_order.contains(&m.id))
            .map(|m| m.id)
            .collect();
        let index = members.iter().position(|m| *m == id)?;
        let next = match key {
            Key::Home => 0,
            Key::End => members.len() - 1,
            Key::Left | Key::Up => (index + members.len() - 1) % members.len(),
            _ => (index + 1) % members.len(),
        };
        Some(members[next])
    }
}
