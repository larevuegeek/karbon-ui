//! Sous-arbres mémorisés ([`Element::memo`]) : empreinte des entrées de la cascade à leur
//! racine, reprise de leurs styles tant qu'elle n'a pas changé.
use super::{environment::StyleEnvironment, matching::Frame};
use crate::ui::{
    element::Element,
    styling::{CssVariables, Typography},
};

/// Place de l'élément dans l'arbre, lue par la cascade.
#[derive(Clone, Copy)]
pub(super) struct TreePosition {
    pub(super) root_font: f32,
    pub(super) root: bool,
    pub(super) index: usize,
    pub(super) siblings: usize,
    pub(super) pointer_clipped: bool,
}

/// Empreinte des entrées de la cascade à la racine d'un sous-arbre mémorisé (`None` pour
/// un autre élément).
///
/// Sous-arbre mémorisé remis en place par la reconstruction : ses styles restent ceux de
/// la dernière résolution tant que rien de ce qu'il lit n'a changé. Son rang et ceux de
/// ses ancêtres n'en font partie que si une règle du sous-arbre lit une position : une
/// ligne d'une liste qui perd ses premières lignes (chargement de la page suivante), ou
/// dont la liste gagne un frère (indicateur de chargement), garde ses styles et, par eux,
/// ses mesures.
pub(super) fn inputs(
    node: &Element,
    tree: &TreePosition,
    parent: Option<&Frame<'_>>,
    previous: &[Element],
    env: &StyleEnvironment,
) -> Option<u64> {
    let TreePosition {
        root_font,
        root,
        index,
        siblings,
        pointer_clipped,
    } = *tree;
    node.memo.as_ref().map(|_| {
        use std::hash::{Hash, Hasher};
        let mut h = std::hash::DefaultHasher::new();
        (root_font.to_bits(), root, pointer_clipped).hash(&mut h);
        // Règle `[.a+&]` de l'élément : les faits de son frère précédent, pas son rang
        // (une liste qui lâche ses premières lignes garde les styles des suivantes).
        if super::partial::reads_previous_facts(node) {
            match previous.last() {
                Some(sibling) => {
                    1u8.hash(&mut h);
                    Frame::sibling_fingerprint(
                        sibling,
                        env,
                        (index.saturating_sub(1), siblings),
                        pointer_clipped,
                        false,
                        &mut h,
                    );
                }
                None => 0u8.hash(&mut h),
            }
        }
        let positions = super::partial::reads_position_beyond_previous(node)
            || parent.is_some_and(|p| {
                p.children
                    .iter()
                    .any(|(state, ..)| super::partial::child_rule_reads_position(state))
            });
        if positions {
            (index, siblings).hash(&mut h);
        }
        (env.viewport.width.to_bits(), env.viewport.height.to_bits()).hash(&mut h);
        (
            env.focus_visible,
            env.reduced_motion,
            env.dark,
            env.high_contrast,
            env.pixel_scale.to_bits(),
        )
            .hash(&mut h);
        if let Some(parent) = parent {
            parent.fingerprint(&mut h, positions);
        }
        h.finish()
    })
}

/// Le sous-arbre mémorisé `node`, remis en place par la dernière reconstruction, garde
/// les styles de sa dernière résolution : mêmes entrées, même héritage, même portée.
pub(super) fn unchanged(
    node: &mut Element,
    inputs: Option<u64>,
    declared: Option<&CssVariables>,
    inherited: Typography,
    env: &StyleEnvironment,
) -> bool {
    let Some(inputs) = inputs else {
        return false;
    };
    let Some(memo) = node.memo.as_deref_mut() else {
        return false;
    };
    let transplanted = std::mem::take(&mut memo.transplanted);
    let same_scope = match (&memo.style_scope, declared) {
        (Some(a), Some(b)) => a.same(b),
        (None, None) => true,
        _ => false,
    };
    let reusable = transplanted
        && env.partial.is_none()
        && !std::mem::take(&mut memo.style_dirty)
        && memo.style_inputs == Some(inputs)
        && memo.style_inherited == Some(inherited)
        && same_scope;
    reusable && !super::partial::reads_siblings_beyond_previous(node)
}

/// Retient les entrées de la résolution qui vient d'avoir lieu.
pub(super) fn remember(
    node: &mut Element,
    inputs: Option<u64>,
    declared: Option<&CssVariables>,
    inherited: Typography,
) {
    if let (Some(inputs), Some(memo)) = (inputs, node.memo.as_deref_mut()) {
        memo.style_inputs = Some(inputs);
        memo.style_inherited = Some(inherited);
        memo.style_scope = declared.cloned();
        memo.style_dirty = false;
    }
}
