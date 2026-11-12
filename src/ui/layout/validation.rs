use super::*;
use crate::ui::core::element::memo::CheckedSubtree;

const TOO_BIG: &str = "element tree too deep (over 64 levels) or too large (over 50,000 elements)";

/// Deux éléments de l'arbre portent le même identifiant.
pub(crate) fn duplicate_id(id: ElementId) -> crate::Error {
    crate::Error::invalid_input(format!("duplicate element id {:#018x}", id.0))
}

/// Most elements in a tree. Large lists stay cheap between frames (memoized subtrees,
/// incremental styles and layout); beyond this, use a virtual list.
pub const MAX_ELEMENTS: usize = 50_000;
/// Checks a tree before layout: depth (at most 64) and size (at most [`MAX_ELEMENTS`]) limits,
/// effect/animation values, lengths, grid tracks and unsupported combinations. Returns an
/// [`Error::InvalidInput`](crate::Error::InvalidInput) error describing the first problem found.
pub fn validate(root: &Element) -> crate::Result<()> {
    const FIXED: &str = "position: fixed under an animated, transformed or translucent ancestor is not supported yet";
    const SPATIAL: &str = "lasting or repeating translate/rotate/scale effects are supported on text, buttons and visual containers; use a fade or a one-shot entrance on inputs, sliders, selects, layers and scroll areas";
    /// Sous-arbre repris tel quel de la vue précédente, déjà vérifié : seul ce qui dépend
    /// de ses ancêtres reste à regarder. Ses identifiants sont vérifiés par l'indexation
    /// de l'arbre (`Interaction::index_tree`).
    fn reused(node: &Element) -> Option<CheckedSubtree> {
        node.memo
            .as_deref()
            .filter(|m| m.transplanted)
            .and_then(|m| m.checked.get())
    }
    /// Élément refusé sous une transformation animée : son placement ou ses zones de
    /// saisie ne suivraient pas.
    fn motion_limited(node: &Element) -> bool {
        matches!(
            node.layout,
            Layout::Layers | Layout::Split(_) | Layout::Scroll(_) | Layout::VirtualList
        ) || node.drag_source.is_some()
            || node.drop_target.is_some()
            || node.widget.as_ref().is_some_and(|w| {
                w.text_input().is_some()
                    || w.slider().is_some()
                    || w.select().is_some()
                    || w.modal().is_some()
                    || w.popover().is_some()
            })
    }
    fn motion(
        node: &Element,
        inherited: bool,
        grouped: bool,
        depth: usize,
        count: &mut usize,
    ) -> crate::Result<CheckedSubtree> {
        if let Some(known) = reused(node) {
            *count += known.elements as usize;
            if depth + known.height as usize > 64 || *count > MAX_ELEMENTS {
                return Err(invalid(TOO_BIG));
            }
            if known.fixed && grouped {
                return Err(invalid(FIXED));
            }
            if known.motion_limited && inherited {
                return Err(invalid(SPATIAL));
            }
            return Ok(known);
        }
        let fixed = super::position::fixed(node);
        if fixed && grouped {
            return Err(invalid(FIXED));
        }
        *count += 1;
        if depth > 64 || *count > MAX_ELEMENTS {
            return Err(invalid(TOO_BIG));
        }
        let spatial = inherited || node.motion.as_deref().is_some_and(|m| m.displaced());
        if node.motion.as_deref().is_some_and(|m| !m.valid()) {
            return Err(invalid("invalid animation or effect"));
        }
        // Une infobulle sous une transformation (survol `scale-105`…) se place sur la
        // géométrie non transformée : écart de quelques pixels au plus.
        let limited = motion_limited(node);
        if spatial && limited {
            return Err(invalid(SPATIAL));
        }
        let filtered = !node.children.is_empty()
            && node.styles.as_ref().is_some_and(|s| {
                s.normal.filters.is_some_and(|f| !f.values.is_empty())
                    || s.states
                        .iter()
                        .any(|(_, p, _)| p.filters.is_some_and(|f| !f.values.is_empty()))
            });
        let choice = node.widget.as_ref().is_some_and(|w| {
            w.semantics().role == crate::ui::element::Role::RadioGroup
                || matches!(w.choice(), Some(crate::ui::element::ChoiceState::Radio(_)))
        });
        let mut summary = CheckedSubtree {
            elements: 1,
            height: 0,
            fixed,
            motion_limited: limited,
            text_input: node
                .widget
                .as_ref()
                .is_some_and(|w| w.text_input().is_some()),
            radio: choice,
            overlay: matches!(node.layout, Layout::Layers)
                || crate::ui::modal::config(node).is_some()
                || crate::ui::popover::config(node).is_some(),
        };
        for child in &node.children {
            let inner = motion(
                child,
                spatial,
                grouped || node.motion.is_some() || filtered,
                depth + 1,
                count,
            )?;
            summary.elements += inner.elements;
            summary.height = summary.height.max(inner.height + 1);
            summary.fixed |= inner.fixed;
            summary.motion_limited |= inner.motion_limited;
            summary.text_input |= inner.text_input;
            summary.radio |= inner.radio;
            summary.overlay |= inner.overlay;
        }
        if let Some(memo) = node.memo.as_deref() {
            memo.checked.set(Some(summary));
        }
        Ok(summary)
    }
    motion(root, false, false, 0, &mut 0)?;
    crate::ui::layers::validate(root)?;
    type Ids = crate::ui::core::hash::FastMap<ElementId, ()>;
    fn walk(node: &Element, ids: &mut Ids, depth: usize) -> crate::Result<()> {
        if depth > 64 || ids.len() >= MAX_ELEMENTS {
            return Err(invalid(TOO_BIG));
        }
        if ids.insert(node.id, ()).is_some() {
            return Err(duplicate_id(node.id));
        }
        if reused(node).is_some() {
            return Ok(());
        }
        let s = node.style;
        if let Some(r) = &node.rules {
            for offset in r.offsets.into_iter().flatten() {
                let valid = match offset {
                    Length::Px(n) | Length::Percent(n) => n.is_finite(),
                    Length::Computed(v) => v.valid(),
                    Length::Auto => true,
                    Length::Fill(_) | Length::MaxContent => false,
                };
                if !valid {
                    return Err(invalid("invalid offset"));
                }
            }
            if [r.flex_grow, r.flex_shrink]
                .into_iter()
                .flatten()
                .any(|n| !n.is_finite() || n < 0.)
                || [r.border.top, r.border.right, r.border.bottom, r.border.left]
                    .iter()
                    .any(|n| !n.is_finite() || *n < 0.)
                || r.columns.as_ref().is_some_and(|tracks| {
                    tracks.is_empty() || tracks.len() > 128 || tracks.iter().any(|t| !t.valid())
                })
            {
                return Err(invalid("invalid box, flex or grid rules"));
            }
            if let Some(basis) = r.flex_basis {
                let valid = match basis {
                    Length::Computed(value) => value.valid(),
                    Length::Auto | Length::MaxContent => true,
                    Length::Px(n) | Length::Percent(n) => n.is_finite() && n >= 0.,
                    Length::Fill(n) => n.is_finite() && n > 0.,
                };
                if !valid {
                    return Err(invalid("invalid flex-basis"));
                }
            }
        }
        if let Some(gradient) = &node.gradient {
            gradient.validate()?;
        }
        if let Some(layers) = &node.backgrounds {
            if layers.len() > 32 {
                return Err(invalid("more than 32 backgrounds on one element"));
            }
            for layer in layers.iter() {
                layer.validate()?;
            }
        }
        if let Some(widget) = &node.widget {
            widget.validate()?;
        }
        if let Some(style) = &node.surface_style {
            style.validate()?;
        }
        if let Some(tooltip) = &node.tooltip {
            tooltip.validate()?;
        }
        if let Some(input) = node.widget.as_ref().and_then(|w| w.text_input()) {
            input.validate()?;
        }
        if let Some(slider) = node.widget.as_ref().and_then(|w| w.slider()) {
            slider.validate()?;
        }
        if let Some(select) = node.widget.as_ref().and_then(|w| w.select()) {
            select.validate()?;
        }
        for length in [s.width, s.height] {
            let valid = match length {
                Length::Computed(value) => value.valid(),
                Length::Auto | Length::MaxContent => true,
                Length::Px(n) => n.is_finite() && n >= 0.,
                // CSS accepte une taille au-delà de 100 % du bloc conteneur (`h-[110%]`).
                Length::Percent(n) => n.is_finite() && n >= 0.,
                Length::Fill(n) => n.is_finite() && n > 0.,
            };
            if !valid {
                return Err(invalid("invalid layout dimension"));
            }
        }
        for n in [
            s.min_width,
            s.min_height,
            s.padding.left,
            s.padding.top,
            s.padding.right,
            s.padding.bottom,
            s.gap,
            s.row_gap.unwrap_or(0.),
            s.column_gap.unwrap_or(0.),
            s.radius,
        ] {
            if !n.is_finite() || n < 0. {
                return Err(invalid("invalid gap or minimum size"));
            }
        }
        if [
            s.margin.left,
            s.margin.top,
            s.margin.right,
            s.margin.bottom,
            s.margin.horizontal(),
            s.margin.vertical(),
        ]
        .iter()
        .any(|n| !n.is_finite())
        {
            return Err(invalid("non-finite margin"));
        }
        if s.max_width.is_nan()
            || s.max_height.is_nan()
            || s.max_width < s.min_width
            || s.max_height < s.min_height
        {
            return Err(invalid("maximum size below the minimum size"));
        }
        if let Layout::Split(split) = node.layout
            && (node.children.len() != 2
                || !split.fraction.is_finite()
                || !(0. ..=1.).contains(&split.fraction)
                || !split.min_first.is_finite()
                || !split.min_second.is_finite()
                || split.min_first < 0.
                || split.min_second < 0.)
        {
            return Err(invalid("invalid split configuration"));
        }
        if matches!(node.layout, Layout::Leaf)
            && node.children.len() != node.widget.as_ref().map_or(0, |w| w.inline_children())
        {
            return Err(invalid("a leaf widget cannot have children"));
        }
        if matches!(node.layout, Layout::Scroll(_)) && node.children.len() != 1 {
            return Err(invalid("a scroll viewport must have exactly one child"));
        }
        // Un volet ou un contenu de défilement masqué est permis (l'autre volet prend la
        // place) ; une couche masquée ne l'est pas : la suivante passerait pour la page.
        if match node.layout {
            Layout::Layers => node.children.iter().any(super::position::out_of_flow),
            Layout::Split(_) | Layout::Scroll(_) => {
                node.children.iter().any(super::position::positioned)
            }
            _ => false,
        } {
            return Err(invalid(
                "positioned elements must be placed inside the content of a scroll viewport, split pane or layer",
            ));
        }
        if super::position::fixed(node)
            && (node
                .widget
                .as_ref()
                .is_some_and(|w| w.modal().is_some() || w.popover().is_some()))
        {
            return Err(invalid(
                "modals and popovers use their own layer and cannot be position: fixed",
            ));
        }
        if matches!(node.layout, Layout::VirtualList) {
            node.virtual_list
                .as_ref()
                .ok_or_else(|| invalid("virtual list without an item builder"))?
                .validate(s.gap)?;
        }
        if let Layout::Grid { columns } = node.layout {
            placements(node, columns)?;
        }
        for child in &node.children {
            walk(child, ids, depth + 1)?;
        }
        Ok(())
    }
    walk(root, &mut Ids::default(), 0)?;
    crate::ui::choices::radio_groups(root)?;
    Ok(())
}
