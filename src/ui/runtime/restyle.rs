//! Résolution des styles par le runtime : cascade de la vue pour l'état d'interaction,
//! faits du pointeur qui permettent de la sauter, et ce qu'une vue reconstruite reprend
//! de l'ancienne.
use super::*;
pub(super) fn has_dynamic_styles(node: &Element) -> bool {
    node.styles.as_ref().is_some_and(|s| !s.states.is_empty())
        || node.children.iter().any(has_dynamic_styles)
}
/// Reporte sur la vue reconstruite ce que l'ancienne avait déjà calculé pour un même
/// élément : ombres rastérisées (réutilisées tant que leur forme ne change pas) et
/// taille vue des conteneurs de requêtes (évite une seconde passe styles + layout).
pub(super) fn carry_caches(
    old: &mut Element,
    new: &mut Element,
    live: &std::collections::HashSet<ElementId>,
) {
    type Carried = (crate::ui::rendering::component_shadow::Cache, Option<SizeF>);
    fn take(
        node: &mut Element,
        live: &std::collections::HashSet<ElementId>,
        out: &mut std::collections::HashMap<ElementId, Carried>,
    ) {
        if live.contains(&node.id)
            && let Some(styles) = node.styles.as_deref_mut()
        {
            out.insert(
                node.id,
                (
                    std::mem::take(&mut styles.shadow_cache),
                    styles.container_seen,
                ),
            );
        }
        for child in &mut node.children {
            take(child, live, out);
        }
    }
    fn give(node: &mut Element, carried: &mut std::collections::HashMap<ElementId, Carried>) {
        if let Some(styles) = node.styles.as_deref_mut()
            && let Some((shadows, seen)) = carried.remove(&node.id)
        {
            styles.shadow_cache = shadows;
            styles.container_seen = seen;
        }
        for child in &mut node.children {
            give(child, carried);
        }
    }
    let mut carried = std::collections::HashMap::new();
    take(old, live, &mut carried);
    give(new, &mut carried);
}
/// Un conteneur de requêtes a changé de taille depuis la dernière résolution des styles.
pub(super) fn containers_changed(node: &Element) -> bool {
    if let Some(styles) = node.styles.as_ref()
        && styles.container_type.is_some()
    {
        let content = layout::content_rect(node);
        let changed = styles.container_seen.is_none_or(|seen| {
            (seen.width - layout::width(content)).abs() > 0.01
                || (seen.height - layout::height(content)).abs() > 0.01
        });
        if changed {
            return true;
        }
    }
    node.children.iter().any(containers_changed)
}
impl UiRuntime {
    /// Faits que le pointeur donne aux éléments de la vue (voir `pointer_facts`).
    fn pointer_facts(&self) -> Option<crate::ui::styling::resolve::PointerFacts> {
        let root = self.root.as_ref()?;
        let project = |id, p| self.motions.point(id, p);
        let environment = crate::ui::styling::StyleEnvironment {
            pointer: self.interaction.pointer,
            project: Some(&project),
            ..Default::default()
        };
        Some(crate::ui::styling::resolve::pointer_facts(
            root,
            &environment,
        ))
    }
    /// Seul le pointeur a changé depuis la dernière résolution des styles, et il donne à
    /// chaque élément les mêmes faits (survol, découpes) : la résolution rendrait les
    /// mêmes styles. Sans mise en page en attente, sans transition de couleurs ou de
    /// longueurs (échantillonnées à chaque résolution) ni conteneur de requêtes dont la
    /// taille vue a changé.
    pub(super) fn pointer_moved_only(&self, signature: StyleSignature) -> bool {
        let Some(previous) = self.invalidation.style_signature else {
            return false;
        };
        self.invalidation.incremental
            && !self.invalidation.relayout
            && previous != signature
            && previous.same_except_pointer(signature)
            && !self.motions.animating_styles()
            && self
                .root
                .as_ref()
                .is_some_and(|root| !containers_changed(root))
            && self.invalidation.pointer_facts.is_some()
            && self.invalidation.pointer_facts == self.pointer_facts()
    }
    /// Résout les styles de la vue pour l'état courant (pointeur, focus, transitions).
    pub(super) fn resolve_styles(
        &mut self,
        context: ViewContext,
        now: Instant,
    ) -> crate::Result<crate::ui::styling::StyleChanges> {
        // Faits du pointeur lus par cette résolution, comparés à la suivante.
        self.invalidation.pointer_facts = self
            .invalidation
            .incremental
            .then(|| self.pointer_facts())
            .flatten();
        let Some(root) = self.root.as_mut() else {
            return Ok(Default::default());
        };
        let project = |id, p| self.motions.point(id, p);
        let input_empty = |id| self.components.inputs.value_empty(id);
        let reduced = self.motions.reduced;
        let transition_color = |id, property, color, timing| {
            self.motions
                .colors
                .borrow_mut()
                .sample(id, property, color, timing, now, reduced)
        };
        let transition_length = |id, property, length, timing| {
            self.motions
                .lengths
                .borrow_mut()
                .sample(id, property, length, timing, now, reduced)
        };
        let motions = &self.motions;
        let animate_colors =
            |id, animation, colors| motions.animate_colors(id, animation, colors, now);
        self.motions.colors.borrow_mut().begin();
        self.motions.lengths.borrow_mut().begin();
        let changes = crate::ui::styling::resolve(
            root,
            crate::ui::styling::StyleEnvironment {
                root_font: None,
                viewport: SizeF {
                    width: context.width,
                    height: context.height,
                },
                pointer: self.interaction.pointer,
                pressed: self.interaction.active_press(),
                focused: self.interaction.focused,
                focus_visible: self.interaction.focus_visible,
                input_empty: Some(&input_empty),
                project: Some(&project),
                transition_color: Some(&transition_color),
                transition_length: Some(&transition_length),
                animate_colors: Some(&animate_colors),
                pixel_scale: context.scale,
                reduced_motion: self.motions.reduced,
            },
        );
        self.motions.colors.borrow_mut().end();
        self.motions.lengths.borrow_mut().end();
        changes
    }
}
