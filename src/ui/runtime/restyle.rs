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
    live: &crate::ui::core::hash::FastSet<ElementId>,
) {
    type Carried = (
        crate::ui::rendering::component_shadow::Cache,
        Option<SizeF>,
        RectF,
        RectF,
    );
    fn take(
        node: &mut Element,
        live: &crate::ui::core::hash::FastSet<ElementId>,
        out: &mut crate::ui::core::hash::FastMap<ElementId, Carried>,
    ) {
        // Élément vide laissé par un sous-arbre mémorisé repris : rien à reporter (le
        // sous-arbre a gardé son état).
        let placeholder = node.memo.as_deref().is_some_and(|m| m.placeholder);
        if live.contains(&node.id) && !placeholder {
            let (shadows, seen) = node
                .styles
                .as_deref_mut()
                .map_or((Default::default(), None), |s| {
                    (std::mem::take(&mut s.shadow_cache), s.container_seen)
                });
            out.insert(node.id, (shadows, seen, node.rect, node.clip));
        }
        for child in &mut node.children {
            take(child, live, out);
        }
    }
    fn give(node: &mut Element, carried: &mut crate::ui::core::hash::FastMap<ElementId, Carried>) {
        if let Some((shadows, seen, rect, clip)) = carried.remove(&node.id) {
            if let Some(styles) = node.styles.as_deref_mut() {
                styles.shadow_cache = shadows;
                styles.container_seen = seen;
            }
            // Élément neuf (sans layout) : la position de l'ancien, pour que les styles
            // résolus avant le layout voient le même survol qu'après (`:hover`, `group-*`).
            if node.rect == RectF::new(0., 0., 0., 0.) {
                node.rect = rect;
                node.clip = clip;
            }
        }
        for child in &mut node.children {
            give(child, carried);
        }
    }
    let mut carried = crate::ui::core::hash::FastMap::default();
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
        let hovered = self.hover_chain();
        let environment = crate::ui::styling::StyleEnvironment {
            pointer: self.interaction.pointer,
            hovered: Some(&hovered),
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
    /// Le dernier layout a changé ce que le pointeur touche depuis la résolution des
    /// styles (positions des éléments d'une vue reconstruite, contenu déplacé).
    pub(super) fn hover_moved_by_layout(&self) -> bool {
        self.interaction.pointer.is_some()
            && self.invalidation.pointer_facts.is_some()
            && self.pointer_facts() != self.invalidation.pointer_facts
    }
    /// Résout les styles de la vue pour l'état courant (pointeur, focus, transitions).
    /// `partial` : depuis la dernière résolution, seuls le pointeur et des animations ont
    /// pu changer ; la cascade ne refait alors que les éléments touchés (voir
    /// `styling::resolve::Partial`).
    pub(super) fn resolve_styles(
        &mut self,
        context: ViewContext,
        now: Instant,
        partial: bool,
    ) -> crate::Result<crate::ui::styling::StyleChanges> {
        // Les styles peuvent créer ou retirer des animations (`hover:scale-105`).
        self.motions.invalidate_paths();
        // Faits du pointeur lus par cette résolution, comparés à la suivante.
        let previous = self.invalidation.pointer_facts.take();
        self.invalidation.pointer_facts = self.pointer_facts();
        let partial = match (&previous, &self.invalidation.pointer_facts, &self.root) {
            (Some(previous), Some(facts), Some(root)) if partial => {
                facts.changed(previous).map(|mut touched| {
                    touched.extend(self.motions.colors.borrow().unsettled(now));
                    touched.extend(self.motions.lengths.borrow().unsettled(now));
                    style_animated(root, &mut touched);
                    crate::ui::styling::resolve::Partial::new(root, touched)
                })
            }
            _ => None,
        };
        if partial.as_ref().is_some_and(|p| p.is_empty()) {
            return Ok(Default::default());
        }
        // Résolution complète : un sous-arbre mémorisé repris garde ses styles sauf si un
        // de ses éléments a changé d'état (pointeur, focus, appui, animation).
        if partial.is_none()
            && let (Some(previous), Some(facts), Some(root)) =
                (&previous, &self.invalidation.pointer_facts, &mut self.root)
        {
            let mut touched = facts.changed_by_id(previous);
            touched.extend(self.motions.colors.borrow().unsettled(now));
            touched.extend(self.motions.lengths.borrow().unsettled(now));
            style_animated(root, &mut touched);
            touched.extend(self.invalidation.style_targets.into_iter().flatten());
            touched.extend(
                [self.interaction.focused, self.interaction.active_press()]
                    .into_iter()
                    .flatten(),
            );
            mark_memo_dirty(root, &touched.into_iter().map(|id| (id, ())).collect());
        }
        self.invalidation.style_targets =
            [self.interaction.focused, self.interaction.active_press()];
        if partial.is_some() {
            self.invalidation.stats.partial_styles += 1;
        }
        self.cascade(context, now, partial.as_ref())
    }

    /// Styles du contenu `content-visibility` qui vient d'approcher de la vue (sauté par
    /// les résolutions précédentes, ou résolu avant de s'éloigner) : `ids` et leurs
    /// sous-arbres en entier, leurs ancêtres sur le chemin.
    pub(super) fn resolve_revealed(
        &mut self,
        ids: &[ElementId],
        context: ViewContext,
        now: Instant,
    ) -> crate::Result<crate::ui::styling::StyleChanges> {
        let Some(root) = self.root.as_ref().filter(|_| !ids.is_empty()) else {
            return Ok(Default::default());
        };
        let partial = crate::ui::styling::resolve::Partial::new(root, ids.iter().copied());
        self.cascade(context, now, Some(&partial))
    }

    /// Cascade de l'arbre pour l'état d'interaction ; `partial` : seuls ces éléments.
    /// La proximité des éléments `content-visibility: auto` lui est prêtée (contenu sauté).
    fn cascade(
        &mut self,
        context: ViewContext,
        now: Instant,
        partial: Option<&crate::ui::styling::resolve::Partial>,
    ) -> crate::Result<crate::ui::styling::StyleChanges> {
        let lent = !layout::containment::entered();
        if lent {
            layout::containment::enter(std::mem::take(&mut self.invalidation.containment));
        }
        let result = self.cascade_inner(context, now, partial);
        if lent {
            self.invalidation.containment = layout::containment::leave();
        }
        result
    }

    fn cascade_inner(
        &mut self,
        context: ViewContext,
        now: Instant,
        partial: Option<&crate::ui::styling::resolve::Partial>,
    ) -> crate::Result<crate::ui::styling::StyleChanges> {
        let hovered = self.hover_chain();
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
        let animate_sizes = |id, animation, sizes| motions.animate_sizes(id, animation, sizes, now);
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
                hovered: Some(&hovered),
                pressed: self.interaction.active_press(),
                focused: self.interaction.focused,
                focus_visible: self.interaction.focus_visible,
                input_empty: Some(&input_empty),
                project: Some(&project),
                transition_color: Some(&transition_color),
                transition_length: Some(&transition_length),
                animate_colors: Some(&animate_colors),
                animate_sizes: Some(&animate_sizes),
                pixel_scale: context.scale,
                reduced_motion: self.motions.reduced,
                dark: self.invalidation.colors.0,
                high_contrast: self.invalidation.colors.1,
                partial,
            },
        );
        // Une résolution partielle ne revoit pas toutes les transitions : celles des
        // éléments laissés de côté sont gardées.
        if partial.is_none() {
            self.motions.colors.borrow_mut().end();
            self.motions.lengths.borrow_mut().end();
        }
        changes
    }
}

/// Éléments dont une animation `@keyframes` anime des couleurs ou des dimensions,
/// échantillonnées pendant la résolution des styles.
fn style_animated(node: &Element, out: &mut Vec<ElementId>) {
    if node
        .motion
        .as_ref()
        .and_then(|m| m.animation)
        .is_some_and(|a| a.animates_colors() || a.animates_sizes())
    {
        out.push(node.id);
    }
    for child in &node.children {
        style_animated(child, out);
    }
}

/// Marque les sous-arbres mémorisés repris qui contiennent un élément de `touched` ;
/// `true` si `node` en contient un.
fn mark_memo_dirty(
    node: &mut Element,
    touched: &crate::ui::core::hash::FastMap<ElementId, ()>,
) -> bool {
    let mut any = touched.contains_key(&node.id);
    for child in &mut node.children {
        any |= mark_memo_dirty(child, touched);
    }
    if any && let Some(memo) = node.memo.as_deref_mut() {
        memo.style_dirty = true;
    }
    any
}
