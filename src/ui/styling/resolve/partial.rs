//! Résolution partielle des styles : quand seuls le pointeur et des animations ont changé
//! depuis la dernière résolution, la cascade ne refait que les éléments touchés (faits du
//! pointeur changés, transitions et animations de style en cours), leurs descendants et
//! les frères suivants qui lisent un frère précédent (`peer-*`, `.a ~ .b`), en passant par
//! leurs ancêtres pour reconstruire le contexte (héritage, variables, faits des ancêtres).
//! Le reste de l'arbre garde ses styles calculés.
use crate::ui::core::hash::FastMap;
use crate::ui::element::{Element, ElementId};
use crate::ui::styling::StyleState;

/// Elements to resolve again in a partial pass ([`StyleEnvironment::partial`](super::StyleEnvironment::partial)):
/// the touched elements with their subtrees, and the path from the root to them.
pub struct Partial {
    /// Éléments touchés : eux et tout leur sous-arbre.
    full: FastMap<ElementId, ()>,
    /// Ancêtres d'un élément touché : résolus pour leurs descendants.
    path: FastMap<ElementId, ()>,
}
impl Partial {
    /// Set of the `touched` elements of the tree `root`.
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
    /// Returns `true` when nothing is to be resolved.
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
/// Règle posée sur l'élément lui-même qui ne lit que des faits de son frère précédent,
/// sans rang (`[.ligne+&]` : classes, balise, attributs, états) : un sous-arbre mémorisé
/// la reprend si ce frère n'a pas changé (son empreinte entre dans celle des entrées).
fn previous_facts(state: &StyleState) -> bool {
    matches!(state, StyleState::PreviousSibling(inner) if !reads(inner, positional))
}
/// Règle d'un enfant direct qui lit de même le frère précédent de son parent
/// (`[.ligne+&]:after:` : la boîte `::after` de la ligne).
fn parent_previous_facts(state: &StyleState) -> bool {
    matches!(state, StyleState::Parent(inner) if previous_facts(inner))
}
fn own_states(node: &Element, keep: impl Fn(&StyleState) -> bool) -> bool {
    node.styles
        .as_ref()
        .is_some_and(|s| s.states.iter().any(|(state, ..)| keep(state)))
}
/// `node`, ou l'un de ses enfants directs, a une règle qui lit le frère précédent de
/// `node` sans rang ([`previous_facts`], [`parent_previous_facts`]).
pub(super) fn reads_previous_facts(node: &Element) -> bool {
    own_states(node, previous_facts)
        || node
            .children
            .iter()
            .any(|child| own_states(child, parent_previous_facts))
}
/// `node` a une règle qui lit ses frères au-delà de son rang (frères précédents,
/// `*-of-type`), hors règles propres de frère précédent sans rang ([`previous_facts`]) :
/// un sous-arbre mémorisé qui en a une est toujours résolu de nouveau.
pub(super) fn reads_siblings_beyond_previous(node: &Element) -> bool {
    node.styles.as_ref().is_some_and(|s| {
        s.states.iter().any(|(state, ..)| {
            !previous_facts(state)
                && reads(state, |state| {
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
        })
    })
}
/// Comme [`reads_position`], sans les règles qui lisent le frère précédent de `node`
/// sans rang ([`reads_previous_facts`]) ; les autres règles du sous-arbre comptent.
pub(super) fn reads_position_beyond_previous(node: &Element) -> bool {
    own_states(node, |state| {
        !previous_facts(state) && reads(state, positional)
    }) || node.children.iter().any(|child| {
        own_states(child, |state| {
            !parent_previous_facts(state) && reads(state, positional)
        }) || child.children.iter().any(reads_position)
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
