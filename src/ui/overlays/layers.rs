//! Anchored placement, layer validation and the focus restoration stack.
use crate::ui::{element::*, layout, modal, popover, scene::RectF};

/// Side of the anchor on which an [`AnchoredLayer`] is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Under the anchor.
    Below,
    /// Over the anchor.
    Above,
}
/// Result of placing a temporary layer (popover, select list, tooltip) next to
/// an anchor, in logical window coordinates.
#[derive(Clone, Copy)]
pub struct AnchoredLayer {
    /// Final layer bounds.
    pub bounds: RectF,
    /// Viewport the layer was constrained to.
    pub viewport: RectF,
    /// Side of the anchor that was chosen.
    pub placement: Placement,
}
impl AnchoredLayer {
    /// Choose the available side (below if the layer fits or if there is more room
    /// there), limit the size and keep the layer inside the viewport.
    pub fn place(anchor: RectF, desired: SizeF, viewport: RectF) -> Self {
        let gap = 4.;
        let below = (viewport.bottom - anchor.bottom - gap).max(0.);
        let above = (anchor.top - viewport.top - gap).max(0.);
        let placement = if below >= desired.height || below >= above {
            Placement::Below
        } else {
            Placement::Above
        };
        let available = if placement == Placement::Below {
            below
        } else {
            above
        };
        let width = desired.width.max(0.).min(layout::width(viewport));
        let height = desired
            .height
            .max(0.)
            .min(available)
            .min(layout::height(viewport));
        let left = anchor
            .left
            .clamp(viewport.left, (viewport.right - width).max(viewport.left));
        let top = if placement == Placement::Below {
            anchor.bottom + gap
        } else {
            anchor.top - gap - height
        };
        let top = top.clamp(viewport.top, (viewport.bottom - height).max(viewport.top));
        Self {
            bounds: RectF::new(left, top, width, height),
            viewport,
            placement,
        }
    }
}
impl AnchoredLayer {
    /// Place a popover on `placement.side` of `anchor` (flipping to the opposite side when
    /// it has more room and the preferred side is too small), aligned along that side,
    /// then shifted to stay inside `viewport`.
    pub fn place_with(
        anchor: RectF,
        desired: SizeF,
        viewport: RectF,
        placement: popover::PopoverPlacement,
    ) -> Self {
        use popover::{Side, SideAlign};
        let gap = placement.offset.max(0.);
        let vertical = matches!(placement.side, Side::Top | Side::Bottom);
        let (before, after) = if vertical {
            (
                (anchor.top - viewport.top - gap).max(0.),
                (viewport.bottom - anchor.bottom - gap).max(0.),
            )
        } else {
            (
                (anchor.left - viewport.left - gap).max(0.),
                (viewport.right - anchor.right - gap).max(0.),
            )
        };
        let need = if vertical {
            desired.height
        } else {
            desired.width
        };
        let preferred_after = matches!(placement.side, Side::Bottom | Side::Right);
        let (room, other) = if preferred_after {
            (after, before)
        } else {
            (before, after)
        };
        let use_after = if room >= need || room >= other {
            preferred_after
        } else {
            !preferred_after
        };
        let available = if use_after { after } else { before };
        let width = if vertical {
            desired.width.max(0.).min(layout::width(viewport))
        } else {
            desired.width.max(0.).min(available)
        };
        let height = if vertical {
            desired.height.max(0.).min(available)
        } else {
            desired.height.max(0.).min(layout::height(viewport))
        };
        let align = |start: f32, end: f32, size: f32| match placement.align {
            SideAlign::Start => start,
            SideAlign::Center => (start + end - size) / 2.,
            SideAlign::End => end - size,
        };
        let shift = if placement.cross_offset.is_finite() {
            placement.cross_offset
        } else {
            0.
        };
        let (left, top) = if vertical {
            (
                align(anchor.left, anchor.right, width) + shift,
                if use_after {
                    anchor.bottom + gap
                } else {
                    anchor.top - gap - height
                },
            )
        } else {
            (
                if use_after {
                    anchor.right + gap
                } else {
                    anchor.left - gap - width
                },
                align(anchor.top, anchor.bottom, height) + shift,
            )
        };
        let left = left.clamp(viewport.left, (viewport.right - width).max(viewport.left));
        let top = top.clamp(viewport.top, (viewport.bottom - height).max(viewport.top));
        Self {
            bounds: RectF::new(left, top, width, height),
            viewport,
            placement: if top + height <= anchor.top + 0.5 {
                Placement::Above
            } else {
                Placement::Below
            },
        }
    }
}

/// Couches modales au-dessus de la page (dialogues, popovers), sans une liste de
/// suggestions (combobox) qui ne prend ni le focus ni la main.
pub(crate) fn stack(root: &Element) -> Vec<ElementId> {
    if matches!(root.layout, Layout::Layers) {
        root.children
            .iter()
            .skip(1)
            .filter(|n| !popover::is_combobox(n))
            .map(|n| n.id)
            .collect()
    } else {
        Vec::new()
    }
}
/// Couche qui reçoit les interactions : la dernière couche modale, sinon toute la racine
/// (page et liste de suggestions éventuelle).
pub(crate) fn active(root: &Element) -> &Element {
    if matches!(root.layout, Layout::Layers)
        && let Some(layer) = root
            .children
            .iter()
            .skip(1)
            .rev()
            .find(|n| !popover::is_combobox(n))
    {
        return layer;
    }
    root
}
pub(crate) fn validate(root: &Element) -> crate::Result<()> {
    fn invalid() -> crate::Error {
        crate::Error::invalid_input("invalid layers or modal")
    }
    fn walk(node: &Element, root: &Element, modal_slot: bool) -> crate::Result<()> {
        if !modal_slot && node.checked_subtree().is_some_and(|c| !c.overlay) {
            return Ok(());
        }
        if let Some(m) = modal::config(node) {
            if !modal_slot
                || !matches!(node.layout, Layout::Column)
                || m.label.trim().is_empty()
                || m.initial_focus.is_some_and(|id| node.find(id).is_none())
            {
                return Err(invalid());
            }
        } else if let Some(p) = popover::config(node) {
            let last = root.children.last().map(|n| n.id);
            if !modal_slot
                || last != Some(node.id)
                || !matches!(node.layout, Layout::Column)
                || p.label.trim().is_empty()
                || p.initial_focus.is_some_and(|id| node.find(id).is_none())
                || matches!(p.anchor, popover::Anchor::Point(p) if !p.x.is_finite() || !p.y.is_finite())
                || matches!(p.anchor, popover::Anchor::Element(id) if node.find(id).is_some())
                || (p.kind == popover::PopoverKind::Combobox
                    && !matches!(p.anchor, popover::Anchor::Element(_)))
            {
                return Err(invalid());
            }
            if p.kind != popover::PopoverKind::Panel {
                fn menu(node: &Element, kind: popover::PopoverKind) -> bool {
                    !matches!(node.layout, Layout::VirtualList | Layout::Split(_))
                        && node
                            .widget
                            .as_ref()
                            .is_none_or(|w| !w.focusable() || kind.allows(w.semantics().role))
                        && node.children.iter().all(|c| menu(c, kind))
                }
                if !menu(node, p.kind) {
                    return Err(crate::Error::invalid_input(
                        "every focusable element of a menu must be a menu item (role=\"menuitem\"), and of a list box an option (role=\"option\")",
                    ));
                }
            }
        } else if modal_slot {
            return Err(invalid());
        }
        let layers = matches!(node.layout, Layout::Layers);
        if layers
            && (!std::ptr::eq(node, root)
                || node.children.is_empty()
                || node
                    .children
                    .iter()
                    .filter(|n| modal::config(n).is_some())
                    .count()
                    > 8
                || node.children.len() > 10)
        {
            return Err(invalid());
        }
        for (i, child) in node.children.iter().enumerate() {
            walk(child, root, layers && i > 0)?;
        }
        Ok(())
    }
    walk(root, root, false)
}

/// Remonte à la racine les dialogues et popovers déclarés n'importe où dans la vue
/// (`<dialog>` d'un template) : la racine devient une pile de couches dont la page est
/// la première, comme si l'application l'avait écrite avec [`Element::layers`].
pub(crate) fn hoist(mut root: Element) -> Element {
    fn overlay(node: &Element) -> bool {
        modal::config(node).is_some() || popover::config(node).is_some()
    }
    /// Renvoie `true` si une couche a quitté le sous-arbre de `node`.
    fn extract(node: &mut Element, found: &mut Vec<Element>) -> bool {
        if matches!(node.layout, Layout::VirtualList) {
            return false;
        }
        let mut moved = false;
        let mut index = 0;
        while index < node.children.len() {
            if overlay(&node.children[index]) {
                let mut layer = node.children.remove(index);
                // Un dialogue peut lui-même en ouvrir un autre.
                for child in &mut layer.children {
                    extract(child, found);
                }
                if let Some(memo) = layer.memo.as_deref_mut() {
                    memo.hoisted = true;
                }
                found.push(layer);
                moved = true;
            } else {
                moved |= extract(&mut node.children[index], found);
                index += 1;
            }
        }
        if moved && let Some(memo) = node.memo.as_deref_mut() {
            memo.hoisted = true;
        }
        moved
    }
    let mut found = Vec::new();
    let layered = matches!(root.layout, Layout::Layers);
    if layered {
        if let Some(page) = root.children.first_mut() {
            extract(page, &mut found);
        }
    } else if !overlay(&root) {
        extract(&mut root, &mut found);
    }
    if found.is_empty() {
        return root;
    }
    // Dialogues dans l'ordre du document, popover en dernier.
    found.sort_by_key(|n| popover::config(n).is_some());
    if layered {
        let at = if root
            .children
            .last()
            .is_some_and(|n| popover::config(n).is_some())
        {
            root.children.len() - 1
        } else {
            root.children.len()
        };
        root.children.splice(at..at, found);
        root
    } else {
        Element::layers(root.id.child("layers").0, root, found)
    }
}

struct Frame {
    id: ElementId,
    restore: Option<ElementId>,
}
#[derive(Default)]
pub(crate) struct LayerFocus {
    frames: Vec<Frame>,
}
impl LayerFocus {
    pub fn changed(&self, root: &Element) -> bool {
        !self.frames.iter().map(|f| f.id).eq(stack(root))
    }
    pub fn sync(
        &mut self,
        root: &Element,
        order: &mut Vec<ElementId>,
        focused: Option<ElementId>,
    ) -> Option<ElementId> {
        let changed = self.changed(root);
        let ids = stack(root);
        let common = self
            .frames
            .iter()
            .zip(&ids)
            .take_while(|(a, b)| a.id == **b)
            .count();
        let mut next = focused;
        while self.frames.len() > common {
            next = self.frames.pop().unwrap().restore;
        }
        for id in ids.iter().skip(common) {
            self.frames.push(Frame {
                id: *id,
                restore: next,
            });
            let node = root.find(*id).unwrap();
            next = modal::config(node)
                .and_then(|m| m.initial_focus)
                .or_else(|| popover::config(node).and_then(|p| p.initial_focus))
                .filter(|id| order.contains(id) || *id == node.id)
                .or_else(|| {
                    order
                        .iter()
                        .copied()
                        .find(|id| *id != node.id && node.find(*id).is_some())
                })
                .or(Some(node.id));
        }
        let scope = active(root);
        if !ids.is_empty() {
            fn collect(node: &Element, ids: &mut std::collections::HashSet<ElementId>) {
                ids.insert(node.id);
                for child in &node.children {
                    collect(child, ids);
                }
            }
            let mut live = std::collections::HashSet::new();
            collect(scope, &mut live);
            order.retain(|id| live.contains(id));
        }
        if (modal::config(scope).is_some() || popover::config(scope).is_some()) && order.len() > 1 {
            order.retain(|id| *id != scope.id);
        }
        if let Some(kind) = popover::config(scope)
            .map(|p| p.kind)
            .filter(|k| *k != popover::PopoverKind::Panel)
        {
            order.retain(|id| {
                scope
                    .find(*id)
                    .and_then(|n| n.widget.as_ref())
                    .is_some_and(|w| kind.allows(w.semantics().role))
            });
            if order.is_empty() {
                order.push(scope.id);
            }
        }
        next.filter(|id| order.contains(id)).or_else(|| {
            (!ids.is_empty() || changed)
                .then(|| order.first().copied())
                .flatten()
        })
    }
}
