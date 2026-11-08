use super::super::*;
impl UiRuntime {
    /// Opaque version of the accessibility tree; when it is unchanged, the previous
    /// [`accessibility`](Self::accessibility) snapshot is still valid.
    pub fn accessibility_version(&self) -> (u64, u64, u64, u64, Option<ElementId>) {
        (
            self.invalidation.stats.views,
            self.invalidation.stats.layouts,
            self.invalidation.stats.scrolls,
            self.invalidation
                .accessibility_revision
                .wrapping_add(self.components.inputs.revision())
                .wrapping_add(self.motions.revision()),
            self.interaction.focused,
        )
    }

    /// Snapshot of the accessibility tree of the active layer, parents before children.
    pub fn accessibility(&self) -> Vec<AccessibleElement> {
        fn visit(
            node: &Element,
            parent: Option<ElementId>,
            selection_group: Option<ElementId>,
            runtime: &UiRuntime,
            order: &HashSet<ElementId>,
            result: &mut Vec<AccessibleElement>,
        ) {
            if crate::ui::layout::hidden(node) || node.semantics_hidden {
                return;
            }
            let focused = runtime.interaction.focused;
            let mut descriptor = None;
            if let Some(widget) = &node.widget {
                let s = widget.semantics();
                let input = widget.text_input();
                let choice = choice(node);
                let bounds = runtime.motions.bounds(node.id, layout::content_rect(node));
                let clipped = layout::intersect(bounds, runtime.motions.clip(node));
                descriptor = Some(AccessibleElement {
                    id: node.id,
                    parent,
                    role: s.role,
                    name: s.name.to_owned(),
                    enabled: s.enabled,
                    focused: focused == Some(node.id)
                        && runtime.components.selects.active(node.id).is_none(),
                    bounds,
                    clipped_bounds: clipped,
                    focusable: order.contains(&node.id),
                    offscreen: layout::width(clipped) <= 0. || layout::height(clipped) <= 0.,
                    invokable: choice.is_none()
                        && (matches!(s.role, Role::Button | Role::MenuItem)
                            || widget.action().is_some()),
                    choice,
                    selection_container: matches!(choice, Some(ChoiceState::Radio(_)))
                        .then_some(selection_group)
                        .flatten(),
                    value: input.and_then(|i| {
                        runtime.components.inputs.snapshot(node.id, i).map(|s| {
                            if i.masked {
                                s.value.chars().map(|_| '\u{2022}').collect()
                            } else {
                                s.value
                            }
                        })
                    }),
                    text: input
                        .filter(|i| !i.masked)
                        .and_then(|_| runtime.components.inputs.accessible_text(node.id)),
                    range: widget
                        .slider()
                        .map(|s| s.info(runtime.components.sliders.value(node.id, s.value))),
                    select: widget
                        .select()
                        .map(|s| runtime.components.selects.snapshot(node.id, s)),
                    read_only: input.is_some_and(|i| i.read_only)
                        || widget.slider().is_some_and(|s| s.read_only),
                    help: input.map_or_else(
                        || {
                            widget
                                .select()
                                .map_or_else(String::new, |s| s.placeholder.to_string())
                        },
                        |i| i.placeholder.to_string(),
                    ),
                    expanded: node.expanded,
                    current: node.aria.current,
                });
            }
            if let Layout::Split(split) = node.layout
                && split.resizable
            {
                let bounds = node.divider.unwrap_or(node.rect);
                let clipped = layout::intersect(bounds, node.clip);
                descriptor = Some(AccessibleElement {
                    id: node.id,
                    parent,
                    role: Role::Separator,
                    name: "Séparateur redimensionnable".into(),
                    enabled: true,
                    focused: focused == Some(node.id),
                    bounds,
                    clipped_bounds: clipped,
                    focusable: true,
                    offscreen: layout::width(clipped) <= 0. || layout::height(clipped) <= 0.,
                    invokable: false,
                    value: None,
                    choice: None,
                    selection_container: None,
                    text: None,
                    range: None,
                    select: None,
                    read_only: true,
                    help: String::new(),
                    expanded: None,
                    current: None,
                });
            }
            if node.scroll.is_some() {
                descriptor = Some(AccessibleElement {
                    id: node.id,
                    parent,
                    role: if node.virtual_list.is_some() {
                        Role::List
                    } else {
                        Role::ScrollArea
                    },
                    name: node.virtual_list.as_ref().map_or_else(
                        || "Zone de défilement".into(),
                        |list| format!("Liste de {} éléments", list.metrics().item_count),
                    ),
                    enabled: true,
                    focused: focused == Some(node.id),
                    bounds: node.rect,
                    clipped_bounds: node.clip,
                    focusable: order.contains(&node.id),
                    offscreen: layout::width(node.clip) <= 0. || layout::height(node.clip) <= 0.,
                    invokable: false,
                    value: None,
                    choice: None,
                    selection_container: None,
                    text: None,
                    range: None,
                    select: None,
                    read_only: true,
                    help: String::new(),
                    expanded: None,
                    current: None,
                });
            }
            // Conteneur de sélection unique : groupe radio ou liste d'onglets.
            let selection_group = if node
                .widget
                .as_ref()
                .is_some_and(|w| matches!(w.semantics().role, Role::RadioGroup | Role::TabList))
            {
                Some(node.id)
            } else {
                selection_group
            };
            if let Some(descriptor) = &mut descriptor
                && let Some(tooltip) = &node.tooltip
            {
                if !descriptor.help.is_empty() {
                    descriptor.help.push('\n');
                }
                descriptor.help.push_str(&tooltip.text);
            }
            let parent = if let Some(descriptor) = descriptor {
                result.push(descriptor);
                if matches!(node.layout, Layout::Split(_)) {
                    parent
                } else {
                    Some(node.id)
                }
            } else {
                parent
            };
            for child in &node.children {
                visit(child, parent, selection_group, runtime, order, result);
            }
        }
        let mut result = Vec::new();
        if let Some(root) = &self.root {
            visit(
                layers::active(root),
                None,
                None,
                self,
                &self.interaction.focus_order.iter().copied().collect(),
                &mut result,
            );
        }
        result
    }
}
/// État de choix d'un élément : celui de son widget (case, interrupteur, radio natifs),
/// sinon celui déclaré par `aria-checked`, `aria-pressed` ou `aria-selected`.
pub(super) fn choice(node: &Element) -> Option<ChoiceState> {
    node.widget
        .as_ref()
        .and_then(|w| w.choice())
        .or_else(|| node.aria.choice())
}
