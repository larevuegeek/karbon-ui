//! Application du style calculé à l'élément : style de base restauré puis recouvert,
//! bordures calées sur les pixels, transitions de dimensions et révélation.
use super::environment::{StyleChanges, StyleEnvironment};
use crate::ui::{
    element::{Element, Length},
    layout,
    styling::{ComputedStyle, Properties},
};

/// Border width snapped like browsers: a positive width becomes a whole number of
/// physical pixels, at least one.
fn snap_border(width: f32, scale: f32) -> f32 {
    if width > 0. && scale != 1. {
        (width * scale + 0.001).floor().max(1.) / scale
    } else {
        width
    }
}
/// Conteneur de requêtes : taille de sa boîte de contenu au dernier layout.
pub(super) fn record_container(node: &mut Element) {
    if node
        .styles
        .as_ref()
        .is_some_and(|s| s.container_type.is_some())
    {
        let content = layout::content_rect(node);
        // Élément neuf d'une vue reconstruite : pas encore de layout, garder la taille
        // reportée de l'ancienne vue.
        if let Some(styles) = node.styles.as_deref_mut()
            && (styles.ready || styles.container_seen.is_none())
        {
            styles.container_seen = Some(crate::ui::element::SizeF {
                width: layout::width(content),
                height: layout::height(content),
            });
        }
    }
}
/// Remplace le style de base de `node` par celui recouvert du style calculé `local`
/// quand le style calculé a changé, puis cale ses bordures sur les pixels physiques.
pub(super) fn apply_computed(
    node: &mut Element,
    local: &Properties,
    computed: ComputedStyle,
    base_border: crate::ui::element::Insets,
    env: &StyleEnvironment,
    changes: &mut StyleChanges,
) {
    if let Some(styles) = &mut node.styles
        && (!styles.ready || styles.computed != computed)
    {
        let base = *styles.base.get_or_insert(node.style);
        let base_layout = *styles.base_layout.get_or_insert(node.layout);
        if local.layout.changes_model() || styles.computed.local.layout.changes_model() {
            node.layout = base_layout;
        }
        let base_rules = styles.base_rules.get_or_insert_with(|| node.rules.clone());
        if !styles.ready || local.layout != styles.computed.local.layout {
            node.rules = base_rules.clone();
        }
        styles.base_border.get_or_insert(base_border);
        let surface = styles
            .base_surface
            .get_or_insert_with(|| node.surface_style.clone())
            .clone();
        node.style = base;
        node.surface_style = surface;
        if let Some(color) = local.background {
            node.style.background = Some(color);
        }
        if let Some(radius) = local.radius {
            node.style.radius = radius;
        }
        if let Some(padding) = local.padding {
            node.style.padding = padding;
        }
        if local.border.is_some() || local.border_width.is_some() {
            let surface = node.surface_style.get_or_insert_with(|| {
                std::sync::Arc::new(crate::ui::style::SurfaceStyle {
                    background: None,
                    border: None,
                    padding: Default::default(),
                    gap: 0.,
                    ..Default::default()
                })
            });
            let surface = std::sync::Arc::make_mut(surface);
            if let Some(color) = local.border {
                surface.border = Some(color.into());
            }
            if let Some(width) = local.border_width {
                surface.border_width = snap_border(width, env.pixel_scale());
            }
        }
        changes.paint = true;
        changes.layout |= !computed.metrics_eq(styles.computed) || !styles.ready;
        styles.computed = computed;
        styles.ready = true;
    }
    if let Some(border) = node.styles.as_ref().and_then(|s| s.base_border)
        && (node.rules.is_some() || local.border_width.is_some() || local.border_sides())
    {
        let uniform = local
            .border_width
            .map_or(border, crate::ui::element::Insets::all);
        let e = local.border_edges;
        let snap = |w: f32| snap_border(w, env.pixel_scale());
        node.layout_rules().border = crate::ui::element::Insets::edges(
            snap(e[0].unwrap_or(uniform.top)),
            snap(e[1].unwrap_or(uniform.right)),
            snap(e[2].unwrap_or(uniform.bottom)),
            snap(e[3].unwrap_or(uniform.left)),
        );
    }
}
pub(super) fn sample_sizes(
    node: &mut Element,
    local: &Properties,
    laid_out: [Length; 2],
    env: &StyleEnvironment,
    changes: &mut StyleChanges,
) {
    // Transitions de dimensions : la cible est la valeur déclarée (base du style,
    // recouverte par les règles actives), la taille affichée vient du runtime.
    if let (Some(sample), Some(timing)) = (
        env.transition_length,
        node.motion
            .as_ref()
            .and_then(|m| m.transition)
            .filter(|t| t.animates_sizes()),
    ) {
        use crate::ui::element::Length;
        let base = node.styles.as_ref().and_then(|s| s.base);
        let targets = [
            local
                .layout
                .width
                .or(base.map(|b| b.width))
                .unwrap_or(node.style.width),
            local
                .layout
                .height
                .or(base.map(|b| b.height))
                .unwrap_or(node.style.height),
        ];
        for (property, target) in targets.into_iter().enumerate() {
            let Length::Px(target) = target else {
                continue;
            };
            let shown = Length::Px(sample(node.id, property as u8, target, timing).max(0.));
            let slot = if property == 0 {
                &mut node.style.width
            } else {
                &mut node.style.height
            };
            // Comparée à la taille du dernier layout, pas à la valeur déclarée que
            // les règles viennent de réécrire.
            changes.layout |= laid_out[property] != shown;
            *slot = shown;
        }
    }
}
pub(super) fn reveal(node: &mut Element, env: &StyleEnvironment, changes: &mut StyleChanges) {
    // Révélation de hauteur (`Element::reveal`) : affiché ↔ `display: none` anime une
    // fraction de la hauteur naturelle (piste `2` des longueurs), mesurée au layout.
    if let Some(timing) = node.motion.as_ref().and_then(|m| m.reveal) {
        let hidden = node.rules.as_ref().is_some_and(|r| r.hidden);
        let target = if hidden { 0. } else { 1. };
        let progress = env
            .transition_length
            .map_or(target, |sample| sample(node.id, 2, target, timing));
        let previous = node.rules.as_ref().and_then(|r| r.reveal);
        let reveal = (progress != target).then(|| crate::ui::element::Reveal {
            progress: progress.clamp(0., 1.),
            height: node.style.height,
            natural: previous.map_or(0., |p| p.natural),
        });
        if reveal.is_some() || previous.is_some() {
            changes.layout |= previous.map(|r| r.progress) != reveal.map(|r| r.progress);
            node.layout_rules().reveal = reveal;
        }
    }
}
pub(crate) fn local(node: &mut Element) {
    if let Some(styles) = &node.styles {
        let computed = ComputedStyle {
            overflow_visible: node.rules.as_ref().is_some_and(|r| {
                r.overflow_x == crate::ui::element::Overflow::Visible
                    || r.overflow_y == crate::ui::element::Overflow::Visible
            }),
            text: styles.normal.text,
            local: styles.normal,
            element_background: styles.normal.background.is_some()
                || node.gradient.is_some()
                || node.backgrounds.is_some(),
            element_border: styles.normal.border.is_some() || styles.normal.border_width.is_some(),
            element_padding: styles.normal.padding.is_some()
                || styles
                    .normal
                    .layout
                    .padding_edges
                    .iter()
                    .any(Option::is_some),
        };
        if let Some(widget) = node.widget.as_mut() {
            widget.apply_style(&computed);
        }
    }
}
