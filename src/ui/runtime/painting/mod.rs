//! Peinture de l'arbre d'éléments en commandes de scène : parcours (ordre de peinture,
//! éléments fixes, effets, filtres, masques, rognages) et chrome de la fenêtre.
use super::*;
mod border;
mod chrome;
mod decoration;
mod surface;
use border::border_sides;
pub(super) use chrome::paint_chrome;
use surface::{background, background_images, outline};
/// Boîte de bordure peinte (fond, bordure, ombres, contour, images de fond, rognage) :
/// calée sur les pixels physiques pour une feuille de framework (voir
/// `Styles::painted_rect`).
pub(super) fn painted(node: &Element) -> RectF {
    node.styles
        .as_ref()
        .map_or(node.rect, |s| s.painted_rect(node.rect))
}
#[derive(Clone, Copy)]
pub(super) struct PaintResources<'a> {
    pub inputs: &'a TextInputs,
    pub sliders: &'a Sliders,
    pub selects: &'a Selects,
    pub motions: &'a crate::ui::animation::timeline::Timeline,
}
pub(super) fn paint_node(
    node: &Element,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
    runtime: PaintState,
    resources: &PaintResources<'_>,
) {
    paint_inner(node, scene, theme, runtime, resources);
    if !matches!(node.layout, Layout::Layers) {
        paint_fixed(node, scene, theme, runtime, resources);
    }
}
fn paint_fixed(
    node: &Element,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
    runtime: PaintState,
    resources: &PaintResources<'_>,
) {
    for index in layout::paint_order(node) {
        let child = &node.children[index];
        if layout::fixed(child) {
            paint_inner(child, scene, theme, runtime, resources);
        }
        paint_fixed(child, scene, theme, runtime, resources);
    }
}
fn paint_inner(
    node: &Element,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
    runtime: PaintState,
    resources: &PaintResources<'_>,
) {
    let PaintResources {
        inputs,
        sliders,
        selects,
        motions,
    } = *resources;
    if layout::width(node.clip) <= 0. || layout::height(node.clip) <= 0. {
        return;
    }
    let effect = motions.local(node);
    let filters = node
        .styles
        .as_ref()
        .and_then(|s| s.computed.local.filters)
        .filter(|f| !f.values.is_empty());
    let bounds = node
        .styles
        .as_ref()
        .map_or(node.clip, |s| s.shadow_cache.bounds(node.rect, node.clip));
    if let Some((transform, opacity)) = effect {
        scene.commands.push(DrawCommand::PushEffect {
            transform,
            opacity,
            bounds: filters.map_or(bounds, |f| f.bounds(bounds)),
        });
    }
    if let Some(filters) = filters {
        scene.commands.push(DrawCommand::PushFilter {
            id: node.id,
            bounds,
            filters,
        });
    }
    let mask = node.styles.as_ref().and_then(|s| s.mask.clone());
    if let Some(gradient) = &mask {
        scene.commands.push(DrawCommand::PushMask {
            bounds: node.rect,
            gradient: std::sync::Arc::clone(gradient),
        });
    }
    let local = node.styles.as_ref().map(|s| s.computed.local);
    let text_clip = local.is_some_and(|l| l.text_clip == Some(true));
    if let Some(styles) = &node.styles {
        styles.shadow_cache.paint(painted(node), false, scene);
        outline(node, styles, scene, theme);
    }
    // Rognage calé comme la boîte peinte : un bord arrondi vers l'extérieur reste visible.
    let clip = node
        .styles
        .as_ref()
        .map_or(node.clip, |s| s.painted_rect(node.clip));
    scene.commands.push(DrawCommand::PushClip(clip));
    if let Some(filters) = local.and_then(|l| l.backdrop) {
        scene.commands.push(DrawCommand::Backdrop {
            rect: node.rect,
            radii: local
                .and_then(|l| l.corner_radii)
                .unwrap_or([node.style.radius; 4]),
            filters,
        });
    }
    let surface = node.surface_style.as_deref();
    // `background-clip: text` : le fond devient le remplissage du texte.
    let text_fill = text_clip.then(|| {
        node.backgrounds
            .as_ref()
            .and_then(|l| l.first().cloned())
            .or_else(|| {
                node.gradient
                    .as_ref()
                    .map(|g| crate::ui::rendering::brush::Background::Linear(g.clone()))
            })
            .or_else(|| node.style.background.map(Into::into))
    });
    if let Some(color) = node
        .style
        .background
        .or_else(|| surface.and_then(|s| s.background).map(|c| c.resolve(theme)))
        .filter(|_| !text_clip)
    {
        background(node, scene, color.into());
    } else if !text_clip
        && node.gradient.is_none()
        && surface.is_none()
        && (modal::config(node).is_some() || popover::config(node).is_some())
    {
        background(node, scene, theme.surface.into());
    }
    if let Some(gradient) = node.gradient.as_ref().filter(|_| !text_clip) {
        background(
            node,
            scene,
            crate::ui::rendering::brush::Background::Linear(std::sync::Arc::clone(gradient)),
        );
    }
    if let Some(layers) = node.backgrounds.as_ref().filter(|_| !text_clip) {
        for layer in layers.iter().rev() {
            background(node, scene, layer.clone());
        }
    }
    if !text_clip && let Some(styles) = &node.styles {
        background_images(node, styles, scene, theme);
    }
    if let Some(Some(fill)) = &text_fill {
        scene.commands.push(DrawCommand::PushTextFill {
            rect: node.rect,
            fill: fill.clone(),
        });
    }
    if let Some(styles) = &node.styles {
        styles.shadow_cache.paint(painted(node), true, scene);
    }
    let sides = node
        .styles
        .as_ref()
        .is_some_and(|s| s.computed.local.border_sides());
    if sides {
        border_sides(node, scene, theme);
    }
    if let Some(surface) = surface.filter(|_| !sides) {
        if let Some(color) = surface.border {
            if let Some(radii) = node
                .styles
                .as_ref()
                .and_then(|s| s.computed.local.corner_radii)
            {
                scene.commands.push(DrawCommand::Shape {
                    rect: painted(node),
                    radii,
                    fill: None,
                    stroke: Some((surface.border_width, color.resolve(theme))),
                });
            } else {
                crate::ui::style::border(
                    scene,
                    painted(node),
                    node.style.radius,
                    surface.border_width,
                    color.resolve(theme),
                );
            }
        }
    } else if popover::config(node).is_some() {
        scene.commands.push(DrawCommand::Stroke {
            rect: node.rect,
            radius: node.style.radius,
            width: 1.,
            color: theme.border,
        });
    }
    let rounded_clip = layout::rounded_clip(node);
    if let Some((rect, radii)) = rounded_clip {
        scene
            .commands
            .push(DrawCommand::PushShapeClip { rect, radii });
    }
    let padding = node.style.padding;
    let border = node.border();
    let p = Insets::edges(
        padding.top + border.top,
        padding.right + border.right,
        padding.bottom + border.bottom,
        padding.left + border.left,
    );
    if let Some(widget) = &node.widget {
        let context = PaintContext {
            rect: RectF::new(
                node.rect.left + p.left,
                node.rect.top + p.top,
                (layout::width(node.rect) - p.left - p.right).max(0.),
                (layout::height(node.rect) - p.top - p.bottom).max(0.),
            ),
            intrinsic: node.intrinsic,
            theme,
            state: VisualState {
                hovered: runtime.hovered == Some(node.id),
                pressed: (runtime.pressed == Some(node.id) && runtime.hovered == Some(node.id))
                    || runtime.keyboard_pressed == Some(node.id),
                focused: runtime.focus_visible && runtime.focused == Some(node.id),
            },
        };
        if let Some(input) = widget.text_input() {
            inputs.paint(node.id, input, &context, scene);
        } else if let Some(slider) = widget.slider() {
            sliders.paint(node.id, slider, &context, scene);
        } else if let Some(select) = widget.select() {
            selects.paint_control(node.id, select, &context, scene);
        } else {
            widget.paint(&context, scene);
        }
    }
    for index in layout::paint_order(node) {
        let child = &node.children[index];
        if layout::fixed(child) {
            continue;
        }
        if let Some(dialog) = modal::config(child) {
            scene.fill(node.rect, 0., dialog.backdrop);
        }
        let mut state = runtime;
        let active = node
            .virtual_list
            .as_ref()
            .is_some_and(|list| list.active == Some(list.indices[index]));
        if active
            && runtime.focused == Some(node.id)
            && let Some((id, _)) = virtual_list::first_action(child)
        {
            state.focused = Some(id);
            if runtime.keyboard_pressed == Some(node.id) {
                state.keyboard_pressed = Some(id);
            }
        }
        paint_inner(child, scene, theme, state, resources);
        if matches!(node.layout, Layout::Layers) {
            paint_fixed(child, scene, theme, state, resources);
        }
        if active && runtime.focused == Some(node.id) && layout::height(child.clip) > 0. {
            let child_effect = motions.local(child);
            if let Some((transform, opacity)) = child_effect {
                scene.commands.push(DrawCommand::PushEffect {
                    transform,
                    opacity,
                    bounds: child.clip,
                });
            }
            scene.commands.push(DrawCommand::PushClip(child.clip));
            scene.fill(
                RectF::new(
                    child.rect.left,
                    child.rect.top,
                    3.,
                    layout::height(child.rect),
                ),
                1.,
                theme.accent,
            );
            scene.commands.push(DrawCommand::PopClip);
            if child_effect.is_some() {
                scene.commands.push(DrawCommand::PopEffect);
            }
        }
    }
    decoration::scroll_bar(node, scene, theme, runtime);
    decoration::divider(node, scene, theme, runtime);
    if rounded_clip.is_some() {
        scene.commands.push(DrawCommand::PopShapeClip);
    }
    if let Some(Some(_)) = &text_fill {
        scene.commands.push(DrawCommand::PopTextFill);
    }
    scene.commands.push(DrawCommand::PopClip);
    if mask.is_some() {
        scene.commands.push(DrawCommand::PopMask);
    }
    if filters.is_some() {
        scene.commands.push(DrawCommand::PopFilter);
    }
    if effect.is_some() {
        scene.commands.push(DrawCommand::PopEffect);
    }
}
