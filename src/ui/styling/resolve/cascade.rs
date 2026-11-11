//! Parcours de l'arbre : cascade des blocs actifs, déclarations dynamiques, héritage de la
//! typographie et calcul du style de chaque élément.
use super::{
    environment::{StyleChanges, StyleEnvironment},
    matching::{Frame, clipped, facts, matches},
    node::{apply_computed, keyframe_sizes, record_container, reveal, sample_sizes},
    scope::declare,
};
use crate::ui::{
    element::Element,
    layout,
    styling::{
        ColorTarget, ComputedStyle, CssVariables, LetterSpacing, Properties, Typography,
        VariableValue, tokens,
    },
};

/// Resolves the styles of `root` and its subtree for `environment`.
///
/// Returns which passes must run again, or an [`Error::InvalidInput`](crate::Error::InvalidInput)
/// error when a declared or computed value is invalid.
pub fn resolve(root: &mut Element, environment: StyleEnvironment) -> crate::Result<StyleChanges> {
    resolve_inherited(root, Typography::default(), environment)
}
pub(crate) fn resolve_inherited(
    root: &mut Element,
    inherited: Typography,
    environment: StyleEnvironment,
) -> crate::Result<StyleChanges> {
    struct TreePosition {
        root_font: f32,
        root: bool,
        index: usize,
        siblings: usize,
        pointer_clipped: bool,
    }
    #[allow(clippy::too_many_arguments)]
    fn visit(
        node: &mut Element,
        inherited: Typography,
        declared: Option<&CssVariables>,
        tree: TreePosition,
        parent: Option<&Frame<'_>>,
        (previous, following): (&[Element], &[Element]),
        env: StyleEnvironment,
        changes: &mut StyleChanges,
    ) -> crate::Result<(bool, bool)> {
        let TreePosition {
            root_font,
            root,
            index,
            siblings,
            pointer_clipped,
        } = tree;
        // Résolution partielle : un élément touché est résolu avec tout son sous-arbre.
        let env = match env.partial {
            Some(partial) if partial.full(node.id) => StyleEnvironment {
                partial: None,
                ..env
            },
            _ => env,
        };
        // Sous-arbre mémorisé remis en place par la reconstruction : ses styles restent
        // ceux de la dernière résolution tant que rien de ce qu'il lit n'a changé. Son rang
        // et ceux de ses ancêtres n'en font partie que si une règle du sous-arbre lit une
        // position : une ligne d'une liste qui perd ses premières lignes (chargement de la
        // page suivante), ou dont la liste gagne un frère (indicateur de chargement), garde
        // ses styles et, par eux, ses mesures.
        let memo_inputs = node.memo.as_ref().map(|_| {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            (root_font.to_bits(), root, pointer_clipped).hash(&mut h);
            // Règle `[.a+&]` de l'élément : les faits de son frère précédent, pas son rang
            // (une liste qui lâche ses premières lignes garde les styles des suivantes).
            if super::partial::reads_previous_facts(node) {
                match previous.last() {
                    Some(sibling) => {
                        1u8.hash(&mut h);
                        super::matching::Frame::sibling_fingerprint(
                            sibling,
                            &env,
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
        });
        if let (Some(inputs), Some(memo)) = (memo_inputs, node.memo.as_deref_mut()) {
            let transplanted = std::mem::take(&mut memo.transplanted);
            let same_scope = match (&memo.style_scope, declared) {
                (Some(a), Some(b)) => a.same(b),
                (None, None) => true,
                _ => false,
            };
            if transplanted
                && env.partial.is_none()
                && !std::mem::take(&mut memo.style_dirty)
                && memo.style_inputs == Some(inputs)
                && memo.style_inherited == Some(inherited)
                && same_scope
                && !super::partial::reads_siblings_beyond_previous(node)
            {
                return Ok((false, false));
            }
        }
        // Layout incrémental : un changement de métrique dans le sous-arbre d'un conteneur
        // annule la reprise de ses mesures et de son placement (`LayoutCache::settled`).
        let layout_above = node
            .layout_cache
            .is_some()
            .then(|| std::mem::take(&mut changes.layout));
        let parent_clipped = pointer_clipped;
        let pointer_clipped = clipped(node, &env, parent_clipped);
        record_container(node);
        let frame = Frame {
            facts: facts(node, &env, index, siblings, pointer_clipped),
            parent,
            previous,
            following,
            parent_clipped,
            children: &[],
            corners: [0.; 4],
        };
        let (contains_focus, contains_press) =
            (frame.facts.contains_focus, frame.facts.contains_press);
        let mut local = Properties::default();
        // Propriétés personnalisées héritées, puis celles de la classe, des états actifs
        // et enfin du template (style:--h={…}) ; portée enfant seulement si l'élément en
        // déclare.
        let mut scope = declared.cloned();
        // États actifs (indices), relus par l'évaluation des déclarations dynamiques.
        let mut active = Vec::new();
        let mut dynamic = false;
        if let Some(styles) = &node.styles {
            local = styles.normal;
            dynamic |= styles.normal.dynamic.is_some();
            let mut own: Vec<(&'static str, VariableValue)> = styles.class_custom.clone();
            for (i, (state, p, _)) in styles.states.iter().enumerate() {
                if matches(*state, &frame, &env) {
                    local = p.over(local);
                    dynamic |= p.dynamic.is_some();
                    own.extend_from_slice(p.custom.unwrap_or_default());
                    active.push(i);
                }
            }
            own.extend_from_slice(&styles.custom);
            if !own.is_empty() {
                scope = Some(declare(declared, &own, styles.globals));
            }
        }
        // Règles du parent pour ses enfants (`space-y-*`) : sous les déclarations propres.
        let mut child_rules = None;
        if let Some(parent) = parent
            && !parent.children.is_empty()
        {
            let mut rules = Properties::default();
            for (state, p, _) in parent.children {
                if matches(*state, &frame, &env) {
                    rules = p.over(rules);
                }
            }
            local = local.over(rules);
            child_rules = Some(rules);
        }
        // Faits détachés du nœud : ses classes sont prêtées aux descendants plus bas.
        let detached = frame.facts.with_classes(&[], &[]);
        local.validate()?;
        if let Some(styles) = &mut node.styles {
            // Portée de l'élément : propriétés héritées au-dessus des variables du contexte.
            let app = styles
                .app_variables
                .get_or_insert_with(|| styles.variables.clone());
            let next = match &scope {
                Some(declared) => Some(CssVariables::scope(declared, app.as_ref())),
                None => app.clone(),
            };
            let same = match (&styles.variables, &next) {
                (Some(a), Some(b)) => a.same(b),
                (None, None) => true,
                _ => false,
            };
            if !same {
                changes.layout = true;
                changes.paint = true;
            }
            styles.variables = next;
            styles.pixel_scale = env.pixel_scale();
        }
        let black = crate::ui::theme::Color::rgb(0, 0, 0);
        // Déclarations dépendant de propriétés personnalisées (feuilles de frameworks) :
        // évaluées dans la portée calculée, à leur rang parmi les blocs actifs.
        if dynamic && let Some(styles) = node.styles.as_deref() {
            let mut eval = tokens::Evaluation {
                variables: styles.variables.as_ref(),
                globals: styles.globals,
                font: local.text.size.unwrap_or(inherited.size.unwrap_or(16.)),
                parent_font: inherited.size.unwrap_or(16.),
                root_font,
                viewport: (env.viewport.width, env.viewport.height),
                scale: if styles.scale > 0. { styles.scale } else { 1. },
                current: local.text.color.or(inherited.color).unwrap_or(black),
                parent_corners: parent.map_or([0.; 4], |p| p.corners),
            };
            let run = |eval: &tokens::Evaluation| {
                let mut out = styles.normal;
                if let Some(d) = out.dynamic {
                    tokens::evaluate(d, eval, &mut out);
                    if out.corner_edges.iter().any(Option::is_some) {
                        // Coins isolés de la base : posés sur ses propres rayons.
                        out = out.over(Properties::default());
                    }
                }
                for &i in &active {
                    let mut block = styles.states[i].1;
                    if let Some(d) = block.dynamic {
                        tokens::evaluate(d, eval, &mut block);
                    }
                    out = block.over(out);
                }
                match child_rules {
                    Some(rules) => out.over(rules),
                    None => out,
                }
            };
            local = run(&eval);
            // `em` suit la taille de police finale de l'élément.
            let font = local.text.size.unwrap_or(eval.parent_font);
            let current = local.text.color.or(inherited.color).unwrap_or(black);
            if font != eval.font || current != eval.current {
                eval.font = font;
                eval.current = current;
                local = run(&eval);
            }
            local.validate()?;
        }
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
        let variables = node.styles.as_ref().and_then(|s| s.variables.as_ref());
        if let Some(e) = local.color_expressions[ColorTarget::Text as usize] {
            local.text.color = Some(e.evaluate(inherited.color.unwrap_or(black), variables));
        }
        let current = local.text.color.or(inherited.color).unwrap_or(black);
        if let Some(e) = local.color_expressions[ColorTarget::Background as usize] {
            local.background = Some(e.evaluate(current, variables));
        }
        if let Some(e) = local.color_expressions[ColorTarget::Border as usize] {
            local.border = Some(e.evaluate(current, variables));
        }
        if let Some(e) = local.color_expressions[ColorTarget::Accent as usize] {
            local.accent = Some(e.evaluate(current, variables));
        }
        // Transitions de couleurs : valeur interpolée par le runtime.
        if let (Some(sample), Some(timing)) = (
            env.transition_color,
            node.motion
                .as_ref()
                .and_then(|m| m.transition)
                .filter(|t| t.animates_colors()),
        ) {
            if let Some(c) = local.text.color {
                local.text.color = Some(sample(node.id, 0, c, timing));
            }
            if let Some(c) = local.background {
                local.background = Some(sample(node.id, 1, c, timing));
            }
            if let Some(c) = local.border {
                local.border = Some(sample(node.id, 2, c, timing));
            }
        }
        // Couleurs animées par `@keyframes` : elles remplacent la valeur calculée (une
        // étape extrême absente reprend cette valeur).
        if let (Some(animate), Some(animation)) = (
            env.animate_colors,
            node.motion
                .as_ref()
                .and_then(|m| m.animation)
                .filter(|a| a.animates_colors()),
        ) {
            let underlying = [
                local.text.color.or(inherited.color),
                local.background,
                local.border,
            ];
            let [text, background, border] = animate(node.id, animation, underlying);
            if text != underlying[0] {
                local.text.color = text;
            }
            local.background = background;
            local.border = border;
        }
        let mut typography = local.text.over(inherited);
        if let Some(expression) = local.font_size_expression {
            let context = crate::ui::layout::UnitContext {
                font: inherited.size.unwrap_or(16.),
                root_font,
                viewport_width: env.viewport.width,
                viewport_height: env.viewport.height,
                scale: 1.,
                pixel_scale: env.pixel_scale(),
            };
            typography.size = Some(expression.evaluate(
                context,
                context.font,
                node.styles.as_ref().and_then(|s| s.variables.as_ref()),
            ));
        }
        // `em` se calcule sur la police de l'élément déclarant, puis s'hérite en px.
        if let Some(LetterSpacing::Em(n)) = local.text.letter_spacing {
            typography.letter_spacing = Some(LetterSpacing::Px(n * typography.size.unwrap_or(16.)));
        }
        typography.validate()?;
        let root_font = if root {
            typography.size.unwrap_or(root_font)
        } else {
            root_font
        };
        let context = crate::ui::layout::UnitContext {
            font: typography.size.unwrap_or(16.),
            root_font,
            viewport_width: env.viewport.width,
            viewport_height: env.viewport.height,
            scale: 1.,
            pixel_scale: env.pixel_scale(),
        };
        // Espacements en expression : `em` suit la police de l'élément ; écrits en px
        // comme des espacements déclarés en px.
        local.layout.resolve_spacing(
            context,
            node.styles.as_ref().and_then(|s| s.variables.as_ref()),
        )?;
        let computed = ComputedStyle {
            overflow_visible: node.rules.as_ref().is_some_and(|r| {
                r.overflow_x == crate::ui::element::Overflow::Visible
                    || r.overflow_y == crate::ui::element::Overflow::Visible
            }),
            text: typography,
            local,
            element_background: local.background.is_some()
                || node.gradient.is_some()
                || node.backgrounds.is_some(),
            element_border: local.border.is_some()
                || local.border_width.is_some()
                || local.border_sides(),
            element_padding: local.padding.is_some()
                || local.layout.padding_edges.iter().any(Option::is_some),
        };
        // Transition de dimensions : le style de base doit être retenu, sans quoi la
        // taille interpolée de l'image précédente passerait pour la valeur déclarée.
        let animates_sizes = node.motion.as_ref().is_some_and(|m| {
            m.transition.is_some_and(|t| t.animates_sizes())
                || m.animation.is_some_and(|a| a.animates_sizes())
        });
        if (animates_sizes || matches!(node.layout, crate::ui::element::Layout::VirtualList))
            && node.styles.is_none()
        {
            node.styles = Some(Default::default());
        }
        let base_border = node.border();
        let laid_out = [node.style.width, node.style.height];
        apply_computed(node, &local, computed, base_border, &env, changes);
        local.layout.apply(node);
        sample_sizes(node, &local, laid_out, &env, changes);
        keyframe_sizes(node, &local, laid_out, &env, changes);
        reveal(node, &env, changes);
        if computed.element_padding {
            node.layout_rules();
        }
        let computed_lengths = matches!(node.style.width, crate::ui::element::Length::Computed(_))
            || matches!(node.style.height, crate::ui::element::Length::Computed(_));
        if computed_lengths || matches!(node.layout, crate::ui::element::Layout::VirtualList) {
            node.layout_rules();
        }
        if let Some(r) = &mut node.rules {
            r.unit_context = context;
        }
        if let Some(widget) = node.widget.as_mut() {
            changes.layout |= widget.apply_style(&computed);
        }
        // Classes prêtées au cadre des descendants (sélecteurs combinés), puis rendues.
        let classes = node
            .styles
            .as_mut()
            .map(|s| std::mem::take(&mut s.classes))
            .unwrap_or_default();
        let child_rules = node
            .styles
            .as_mut()
            .map(|s| std::mem::take(&mut s.child_states))
            .unwrap_or_default();
        let attributes = node
            .styles
            .as_mut()
            .map(|s| std::mem::take(&mut s.attributes))
            .unwrap_or_default();
        let own = Frame {
            facts: detached.with_classes(&classes, &attributes),
            parent,
            previous,
            following,
            parent_clipped,
            children: &child_rules,
            corners: {
                let mut c = local.corner_radii.unwrap_or([node.style.radius; 4]);
                for (corner, edge) in c.iter_mut().zip(local.corner_edges) {
                    if let Some(edge) = edge {
                        *corner = edge;
                    }
                }
                c
            },
        };
        // `content-visibility` : contenu sauté, résolu quand il approche de la vue
        // (`UiRuntime::resolve_revealed`) ; Chromium ne calcule pas non plus ses styles.
        let count = if layout::containment::skipped(node) {
            0
        } else {
            node.children.len()
        };
        let mut result = Ok(());
        // Résolution partielle : seuls les enfants touchés, sur le chemin d'un élément
        // touché, ou qui lisent un frère précédent touché, sont visités.
        let mut sibling_touched = false;
        for index in 0..count {
            let (before, rest) = node.children.split_at_mut(index);
            let (child, after) = rest.split_first_mut().unwrap();
            let after: &[Element] = after;
            let mut env = env;
            if let Some(partial) = env.partial {
                let touched = partial.full(child.id);
                let wanted = partial.wanted(child.id);
                let by_sibling = sibling_touched && super::partial::reads_previous_siblings(child);
                sibling_touched |= touched;
                if !wanted && !by_sibling {
                    continue;
                }
                if touched || by_sibling {
                    env.partial = None;
                }
            }
            let child_clipped = pointer_clipped && !layout::fixed(child);
            result = visit(
                child,
                typography,
                scope.as_ref(),
                TreePosition {
                    root_font,
                    root: false,
                    index,
                    siblings: count,
                    pointer_clipped: child_clipped,
                },
                Some(&own),
                (before, after),
                env,
                changes,
            )
            .map(drop);
            if result.is_err() {
                break;
            }
        }
        let _ = own;
        if let Some(styles) = node.styles.as_mut() {
            styles.classes = classes;
            styles.child_states = child_rules;
            styles.attributes = attributes;
        }
        result?;
        if let (Some(inputs), Some(memo)) = (memo_inputs, node.memo.as_deref_mut()) {
            memo.style_inputs = Some(inputs);
            memo.style_inherited = Some(inherited);
            memo.style_scope = declared.cloned();
            memo.style_dirty = false;
        }
        if let Some(above) = layout_above {
            if changes.layout
                && let Some(cache) = node.layout_cache.as_deref_mut()
            {
                cache.settled = false;
            }
            changes.layout |= above;
        }
        Ok((contains_focus, contains_press))
    }
    let mut changes = StyleChanges::default();
    visit(
        root,
        inherited,
        None,
        TreePosition {
            root_font: environment
                .root_font
                .unwrap_or_else(|| inherited.size.unwrap_or(16.)),
            root: environment.root_font.is_none(),
            index: 0,
            siblings: 1,
            pointer_clipped: false,
        },
        None,
        (&[], &[]),
        environment,
        &mut changes,
    )?;
    Ok(changes)
}
