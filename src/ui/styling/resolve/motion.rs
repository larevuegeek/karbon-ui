//! Surcharges d'animation des règles de la feuille : `transition` évaluée, réglages
//! d'animation d'une règle d'état, opacité et transformation des états actifs.
use super::environment::StyleChanges;
use crate::ui::{element::Element, styling::Properties};

/// Applique au mouvement de `node` ce que les propriétés calculées `local` en déclarent.
pub(super) fn apply(node: &mut Element, local: &Properties, changes: &mut StyleChanges) {
    // `transition` évaluée par la feuille (composée, à travers `var()`) : remplace celle
    // du template tant qu'une règle la déclare.
    if let Some(styles) = node.styles.as_deref_mut()
        && (local.transition.is_some() || styles.base_transition.is_some())
    {
        let motion = node.motion.get_or_insert_with(Default::default);
        let base = *styles.base_transition.get_or_insert(motion.transition);
        let transition = local.transition.or(base);
        if motion.transition != transition {
            motion.transition = transition;
            changes.paint = true;
        }
    }
    // Réglages d'animation d'une règle d'état (`animation-direction`, `-delay`,
    // `-duration`, `-play-state`) : appliqués à l'animation du template tant qu'elle
    // s'applique ; une pause ou une reprise ne redémarre pas l'animation.
    if let Some(styles) = node.styles.as_deref_mut()
        && let Some(motion) = node.motion.as_deref_mut()
        && (!local.animation.is_empty() || styles.base_animation.is_some())
    {
        let base = match styles.base_animation {
            Some((base, shown)) if shown == motion.animation => base,
            _ => motion.animation,
        };
        let shown = base.and_then(|a| local.animation.apply(a));
        styles.base_animation = Some((base, shown));
        if motion.animation != shown {
            motion.animation = shown;
            changes.paint = true;
        }
    }
    // Opacité/transformation des règles d'état combinées (déclarations dynamiques
    // comprises), animées par la transition.
    let state_effect = local.effect.map(|e| {
        (
            e.effect,
            crate::ui::animation::ReplacedProperties {
                opacity: e.opacity,
                transform: e.transform,
            },
        )
    });
    if node.motion.as_ref().and_then(|m| m.state) != state_effect
        && (state_effect.is_some() || node.motion.is_some())
    {
        node.motion.get_or_insert_with(Default::default).state = state_effect;
        changes.paint = true;
    }
}
