//! Résolution partielle des styles : quand seuls le pointeur et des animations ont changé
//! depuis la dernière résolution, la cascade ne refait que les éléments touchés (faits du
//! pointeur changés, transitions et animations de style en cours), leurs descendants et
//! les frères suivants qui lisent un frère précédent (`peer-*`, `.a ~ .b`), en passant par
//! leurs ancêtres pour reconstruire le contexte (héritage, variables, faits des ancêtres).
//! Le reste de l'arbre garde ses styles calculés.
use crate::ui::core::hash::FastMap;
use crate::ui::element::{Element, ElementId};
use crate::ui::styling::StyleState;

/// Éléments à résoudre de nouveau.
pub struct Partial {
    /// Éléments touchés : eux et tout leur sous-arbre.
    full: FastMap<ElementId, ()>,
    /// Ancêtres d'un élément touché : résolus pour leurs descendants.
    path: FastMap<ElementId, ()>,
}
impl Partial {
    /// Ensemble des éléments `touched` de l'arbre `root`.
    pub fn new(root: &Element, touched: impl IntoIterator<Item = ElementId>) -> Self {
        let full: FastMap<ElementId, ()> = touched.into_iter().map(|id| (id, ())).collect();
        let mut path = FastMap::default();
        fn mark(
            node: &Element,
            full: &FastMap<ElementId, ()>,
            path: &mut FastMap<ElementId, ()>,
        ) -> bool {
            let mut any = full.contains_key(&node.id);
            // Contenu sauté (`content-visibility`) : la cascade n'y descend pas.
            if !node.content_skipped {
                for child in &node.children {
                    any |= mark(child, full, path);
                }
            }
            if any {
                path.insert(node.id, ());
            }
            any
        }
        mark(root, &full, &mut path);
        Self { full, path }
    }
    /// Rien à résoudre.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
    /// `id` et son sous-arbre sont résolus en entier.
    pub(super) fn full(&self, id: ElementId) -> bool {
        self.full.contains_key(&id)
    }
    /// `id` est résolu (touché, ou ancêtre d'un élément touché).
    pub(super) fn wanted(&self, id: ElementId) -> bool {
        self.path.contains_key(&id)
    }
}

/// `node` a une règle qui lit un frère précédent (`peer-*`, `.a + .b`, `.a ~ .b`).
pub(super) fn reads_previous_siblings(node: &Element) -> bool {
    reads_state(node, |state| {
        matches!(
            state,
            StyleState::PreviousSibling(_) | StyleState::AnyPreviousSibling(_)
        )
    })
}
/// `node` a une règle qui lit ses frères au-delà de son rang (frères précédents, `*-of-type`).
pub(super) fn reads_siblings(node: &Element) -> bool {
    reads_state(node, |state| {
        matches!(
            state,
            StyleState::PreviousSibling(_)
                | StyleState::AnyPreviousSibling(_)
                | StyleState::FirstOfType
                | StyleState::LastOfType
                | StyleState::OnlyOfType
                | StyleState::NthOfType(..)
                | StyleState::NthLastOfType(..)
        )
    })
}
/// Une règle de `node` ou d'un descendant lit un rang parmi des frères (`:first-child`,
/// `:nth-child`, `*-of-type`, frères précédents), le sien ou, par un ancêtre
/// (`group-first:`), celui de `node`. Sans elle, le rang de `node` ne change pas ses
/// styles ni ceux de son sous-arbre.
pub(super) fn reads_position(node: &Element) -> bool {
    reads_state(node, positional) || node.children.iter().any(reads_position)
}
/// Règle portée par un parent pour ses enfants (`[&>*:first-child]`) qui lit le rang de
/// l'enfant.
pub(super) fn child_rule_reads_position(state: &StyleState) -> bool {
    reads(state, positional)
}
fn positional(state: &StyleState) -> bool {
    matches!(
        state,
        StyleState::FirstChild
            | StyleState::LastChild
            | StyleState::OnlyChild
            | StyleState::NthChild(..)
            | StyleState::NthLastChild(..)
            | StyleState::FirstOfType
            | StyleState::LastOfType
            | StyleState::OnlyOfType
            | StyleState::NthOfType(..)
            | StyleState::NthLastOfType(..)
            | StyleState::PreviousSibling(_)
            | StyleState::AnyPreviousSibling(_)
    )
}
fn reads(state: &StyleState, leaf: fn(&StyleState) -> bool) -> bool {
    if leaf(state) {
        return true;
    }
    match state {
        StyleState::Any(states) | StyleState::All(states) => states.iter().any(|s| reads(s, leaf)),
        StyleState::Not(s)
        | StyleState::Where(s)
        | StyleState::Ancestor(s)
        | StyleState::Parent(s) => reads(s, leaf),
        _ => false,
    }
}
fn reads_state(node: &Element, leaf: fn(&StyleState) -> bool) -> bool {
    node.styles
        .as_ref()
        .is_some_and(|s| s.states.iter().any(|(state, ..)| reads(state, leaf)))
}
