//! États transitoires des composants, conservés entre deux reconstructions de la vue.
//!
//! Chaque état implémente [`ComponentState`] : le runtime appelle ces crochets sur
//! tous les états sans les nommer. Un nouveau composant complexe ajoute son état à
//! [`ComponentStates`], l'inscrit dans `each` / `each_mut` et implémente les crochets utiles.
use super::*;

/// Glissement du séparateur d'un split.
#[derive(Clone, Copy)]
pub(super) struct Drag {
    pub(super) id: ElementId,
    pub(super) axis: Axis,
    pub(super) original: f32,
}
/// Glissement du pouce d'une barre de défilement.
#[derive(Clone, Copy)]
pub(super) struct ScrollDrag {
    pub(super) id: ElementId,
    pub(super) grab: f32,
    pub(super) original: f32,
}

/// Données de l'arbre arrangé transmises à la réconciliation avant peinture.
pub(super) struct SyncContext<'a> {
    pub(super) root: &'a Element,
    pub(super) focused: Option<ElementId>,
    pub(super) text: &'a mut dyn TextMeasurer,
    pub(super) rebuilt: bool,
    /// Zone disponible pour les popups, marge de fenêtre déduite.
    pub(super) viewport: RectF,
}
#[derive(Default)]
pub(super) struct SyncOutcome {
    /// Une capture pointeur a été perdue : l'hôte doit la relâcher.
    pub(super) release: bool,
    /// Une information exposée à l'accessibilité a changé.
    pub(super) semantics: bool,
}

/// Crochets de cycle de vie appelés par le runtime sur chaque état de composant.
/// Tous sont neutres par défaut.
pub(super) trait ComponentState {
    /// Une interaction pointeur en cours retient la souris.
    fn captured(&self) -> bool {
        false
    }
    /// Après reconstruction de la vue : écarter les éléments disparus ou devenus
    /// incompatibles. `true` si une capture pointeur a été perdue.
    fn retain_rebuilt(&mut self, _root: &Element, _live: &HashSet<ElementId>) -> bool {
        false
    }
    /// Après retrait d'une couche orpheline : ne garder que les éléments visités.
    fn retain_live(&mut self, _live: &HashSet<ElementId>) {}
    /// Après layout : mémoriser l'état porté par l'arbre pour la prochaine reconstruction.
    fn save(&mut self, _root: &Element) {}
    /// Après matérialisation des lignes virtuelles : écarter les nœuds incompatibles.
    fn retain_refreshed(&mut self, _root: &Element) {}
    /// Avant peinture : réconcilier l'état avec l'arbre arrangé et le focus.
    fn reconcile(&mut self, _cx: &mut SyncContext<'_>) -> crate::Result<SyncOutcome> {
        Ok(SyncOutcome::default())
    }
}

/// Fractions des splits redimensionnés et glissement de séparateur en cours.
#[derive(Default)]
pub(super) struct SplitState {
    pub(super) fractions: HashMap<ElementId, f32>,
    pub(super) drag: Option<Drag>,
}
/// Offsets de défilement et glissement de pouce en cours.
#[derive(Default)]
pub(super) struct ScrollState {
    pub(super) offsets: HashMap<ElementId, f32>,
    pub(super) drag: Option<ScrollDrag>,
}
/// Ligne active de chaque liste virtuelle.
#[derive(Default)]
pub(super) struct ListState {
    pub(super) positions: HashMap<ElementId, usize>,
}

#[derive(Default)]
pub(super) struct ComponentStates {
    pub(super) inputs: TextInputs,
    /// Menu contextuel intégré des champs de saisie.
    pub(super) edit_menu: EditMenu,
    pub(super) sliders: Sliders,
    pub(super) selects: Selects,
    pub(super) tooltips: Tooltips,
    pub(super) splits: SplitState,
    pub(super) scrolls: ScrollState,
    pub(super) lists: ListState,
    /// Glisser-déposer d'éléments : abandonné à chaque reconstruction.
    pub(super) item_drag: Option<drag_drop::State>,
}
impl ComponentStates {
    /// États soumis aux crochets, dans l'ordre de réconciliation.
    fn each(&self) -> [&dyn ComponentState; 7] {
        [
            &self.inputs,
            &self.edit_menu,
            &self.sliders,
            &self.selects,
            &self.splits,
            &self.scrolls,
            &self.lists,
        ]
    }

    fn each_mut(&mut self) -> [&mut dyn ComponentState; 7] {
        [
            &mut self.inputs,
            &mut self.edit_menu,
            &mut self.sliders,
            &mut self.selects,
            &mut self.splits,
            &mut self.scrolls,
            &mut self.lists,
        ]
    }

    pub(super) fn captured(&self) -> bool {
        self.item_drag.is_some() || self.each().iter().any(|state| state.captured())
    }

    pub(super) fn retain_rebuilt(&mut self, root: &Element, live: &HashSet<ElementId>) -> bool {
        let mut release = false;
        for state in self.each_mut() {
            release |= state.retain_rebuilt(root, live);
        }
        release
    }

    pub(super) fn retain_live(&mut self, live: &HashSet<ElementId>) {
        for state in self.each_mut() {
            state.retain_live(live);
        }
    }

    pub(super) fn save(&mut self, root: &Element) {
        for state in self.each_mut() {
            state.save(root);
        }
    }

    pub(super) fn retain_refreshed(&mut self, root: &Element) {
        for state in self.each_mut() {
            state.retain_refreshed(root);
        }
    }

    /// Réconcilier dans l'ordre ; une erreur interrompt les états suivants.
    pub(super) fn reconcile(&mut self, cx: &mut SyncContext<'_>) -> crate::Result<SyncOutcome> {
        let mut outcome = SyncOutcome::default();
        for state in self.each_mut() {
            let step = state.reconcile(cx)?;
            outcome.release |= step.release;
            outcome.semantics |= step.semantics;
        }
        Ok(outcome)
    }
}

fn is_scroll(node: &Element) -> bool {
    matches!(node.layout, Layout::Scroll(_) | Layout::VirtualList)
}

impl ComponentState for TextInputs {
    fn captured(&self) -> bool {
        self.dragging.is_some()
    }
    fn reconcile(&mut self, cx: &mut SyncContext<'_>) -> crate::Result<SyncOutcome> {
        let captured = self.dragging.is_some();
        self.sync(cx.root, cx.focused, cx.text, cx.rebuilt)?;
        Ok(SyncOutcome {
            release: captured && self.dragging.is_none(),
            semantics: false,
        })
    }
}

impl ComponentState for EditMenu {
    fn captured(&self) -> bool {
        EditMenu::captured(self)
    }
    fn reconcile(&mut self, cx: &mut SyncContext<'_>) -> crate::Result<SyncOutcome> {
        let menu = self.sync(cx.root, cx.focused);
        Ok(SyncOutcome {
            release: menu.release,
            semantics: menu.repaint,
        })
    }
}

impl ComponentState for Sliders {
    fn captured(&self) -> bool {
        Sliders::captured(self)
    }
    fn reconcile(&mut self, cx: &mut SyncContext<'_>) -> crate::Result<SyncOutcome> {
        Ok(SyncOutcome {
            release: self.sync(cx.root, cx.focused),
            semantics: false,
        })
    }
}

impl ComponentState for Selects {
    fn captured(&self) -> bool {
        Selects::captured(self)
    }
    fn reconcile(&mut self, cx: &mut SyncContext<'_>) -> crate::Result<SyncOutcome> {
        let popup = self.sync(cx.root, cx.focused, cx.viewport);
        Ok(SyncOutcome {
            release: popup.release,
            semantics: popup.repaint,
        })
    }
}

impl ComponentState for SplitState {
    fn captured(&self) -> bool {
        self.drag.is_some()
    }
    fn retain_rebuilt(&mut self, root: &Element, live: &HashSet<ElementId>) -> bool {
        self.fractions.retain(|id, _| live.contains(id));
        let lost = self.drag.is_some_and(|d| {
            !root
                .find(d.id)
                .is_some_and(|n| matches!(n.layout, Layout::Split(s) if s.resizable))
        });
        if lost {
            self.drag = None;
        }
        lost
    }
    fn retain_live(&mut self, live: &HashSet<ElementId>) {
        self.fractions.retain(|id, _| live.contains(id));
    }
    fn retain_refreshed(&mut self, root: &Element) {
        self.fractions.retain(|id, _| {
            root.find(*id)
                .is_some_and(|n| matches!(n.layout, Layout::Split(_)))
        });
    }
}

impl ComponentState for ScrollState {
    fn captured(&self) -> bool {
        self.drag.is_some()
    }
    fn retain_rebuilt(&mut self, root: &Element, _live: &HashSet<ElementId>) -> bool {
        self.offsets
            .retain(|id, _| root.find(*id).is_some_and(is_scroll));
        let lost = self
            .drag
            .is_some_and(|d| !root.find(d.id).is_some_and(is_scroll));
        if lost {
            self.drag = None;
        }
        lost
    }
    fn retain_live(&mut self, live: &HashSet<ElementId>) {
        self.offsets.retain(|id, _| live.contains(id));
    }
    fn save(&mut self, root: &Element) {
        fn save(node: &Element, offsets: &mut HashMap<ElementId, f32>) {
            if let Some(s) = node.scroll {
                offsets.insert(node.id, s.offset);
            }
            for child in &node.children {
                save(child, offsets);
            }
        }
        save(root, &mut self.offsets);
    }
    fn retain_refreshed(&mut self, root: &Element) {
        self.offsets
            .retain(|id, _| root.find(*id).is_some_and(|n| n.scroll.is_some()));
    }
}

impl ComponentState for ListState {
    fn retain_rebuilt(&mut self, root: &Element, _live: &HashSet<ElementId>) -> bool {
        self.positions
            .retain(|id, _| root.find(*id).is_some_and(|n| n.virtual_list.is_some()));
        false
    }
    fn retain_live(&mut self, live: &HashSet<ElementId>) {
        self.positions.retain(|id, _| live.contains(id));
    }
    fn save(&mut self, root: &Element) {
        fn save(node: &Element, positions: &mut HashMap<ElementId, usize>) {
            if let Some(list) = &node.virtual_list {
                if let Some(index) = list.active {
                    positions.insert(node.id, index);
                } else {
                    positions.remove(&node.id);
                }
            }
            for child in &node.children {
                save(child, positions);
            }
        }
        save(root, &mut self.positions);
    }
}
