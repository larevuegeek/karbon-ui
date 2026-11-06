//! Fond et contour de la boîte : couleurs, dégradés, couches et images de fond, `outline`.
use super::painted;
use crate::ui::{
    element::Element,
    layout,
    scene::{DrawCommand, RectF, Scene},
};
pub(super) fn background(
    node: &Element,
    scene: &mut Scene,
    fill: crate::ui::rendering::brush::Background,
) {
    if let Some(radii) = node
        .styles
        .as_ref()
        .and_then(|s| s.computed.local.corner_radii)
    {
        scene.commands.push(DrawCommand::Shape {
            rect: painted(node),
            radii,
            fill: Some(fill),
            stroke: None,
        });
    } else {
        fill.paint(scene, painted(node), node.style.radius);
    }
}
/// `outline` : trait hors de la boîte de bordure, décalé de `outline-offset`, sans
/// effet sur le layout.
pub(super) fn outline(
    node: &Element,
    styles: &crate::ui::styling::Styles,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
) {
    let local = styles.computed.local;
    let Some(width) = local.outline_width.filter(|w| *w > 0.) else {
        return;
    };
    let grow = local.outline_offset.unwrap_or(0.) + width / 2.;
    let r = painted(node);
    let rect = RectF {
        left: r.left - grow,
        top: r.top - grow,
        right: r.right + grow,
        bottom: r.bottom + grow,
    };
    if layout::width(rect) <= 0. || layout::height(rect) <= 0. {
        return;
    }
    let color = local
        .outline_color
        .or(styles.computed.text.color)
        .unwrap_or(theme.text);
    let radii = local
        .corner_radii
        .unwrap_or([node.style.radius; 4])
        .map(|n| if n > 0. { (n + grow).max(0.) } else { 0. });
    scene.commands.push(DrawCommand::Shape {
        rect,
        radii,
        fill: None,
        stroke: Some((width, color)),
    });
}

/// Couches d'images SVG de `background-image` : placées dans la boîte de remplissage
/// (`background-origin: padding-box`), rognées à la boîte de bordure ; la première
/// couche est peinte au-dessus.
pub(super) fn background_images(
    node: &Element,
    styles: &crate::ui::styling::Styles,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
) {
    use crate::ui::styling::{BackgroundLength, BackgroundSize};
    let images = styles.computed.local.background_images;
    let Some(layers) = images.layers.filter(|_| !images.is_empty()) else {
        return;
    };
    let b = node.border();
    let r = painted(node);
    let area = RectF {
        left: r.left + b.left,
        top: r.top + b.top,
        right: r.right - b.right,
        bottom: r.bottom - b.bottom,
    };
    let (aw, ah) = (layout::width(area), layout::height(area));
    if aw <= 0. || ah <= 0. {
        return;
    }
    let color = styles.computed.text.color.unwrap_or(theme.text);
    scene.commands.push(DrawCommand::PushClip(r));
    for (i, image) in layers.iter().enumerate().rev() {
        let Some(image) = image else {
            continue;
        };
        let [_, _, vw, vh] = image.view_box;
        let ratio = vw / vh;
        let length = |l: BackgroundLength, full: f32| match l {
            BackgroundLength::Auto => None,
            BackgroundLength::Px(px) => Some(px),
            BackgroundLength::Percent(p) => Some(p / 100. * full),
        };
        // Sans taille intrinsèque, `auto` remplit la zone comme `contain`.
        let contain = if aw / ah > ratio {
            (ah * ratio, ah)
        } else {
            (aw, aw / ratio)
        };
        let (w, h) = match images.size.map_or(BackgroundSize::Auto, |s| s[i]) {
            BackgroundSize::Auto | BackgroundSize::Contain => contain,
            BackgroundSize::Cover => {
                if aw / ah > ratio {
                    (aw, aw / ratio)
                } else {
                    (ah * ratio, ah)
                }
            }
            BackgroundSize::Explicit(w, h) => match (length(w, aw), length(h, ah)) {
                (Some(w), Some(h)) => (w, h),
                (Some(w), None) => (w, w / ratio),
                (None, Some(h)) => (h * ratio, h),
                (None, None) => contain,
            },
        };
        if !(w > 0. && h > 0. && w.is_finite() && h.is_finite()) {
            continue;
        }
        let position = images.position.map(|p| p[i]).unwrap_or_default();
        let left = area.left + position.x.0 * (aw - w) + position.x.1;
        let top = area.top + position.y.0 * (ah - h) + position.y.1;
        let repeat = images.repeat.is_none_or(|r| r[i]);
        let (columns, rows) = if repeat {
            // Pavage de toute la boîte de bordure (au plus 64 × 64 tuiles).
            let first = |start: f32, size: f32, origin: f32| {
                origin - ((origin - start) / size).ceil().max(0.) * size
            };
            let x0 = first(r.left, w, left);
            let y0 = first(r.top, h, top);
            let n =
                |from: f32, end: f32, size: f32| (((end - from) / size).ceil() as usize).min(64);
            ((x0, n(x0, r.right, w)), (y0, n(y0, r.bottom, h)))
        } else {
            ((left, 1), (top, 1))
        };
        for row in 0..rows.1 {
            for column in 0..columns.1 {
                scene.commands.push(DrawCommand::Svg {
                    rect: RectF::new(columns.0 + column as f32 * w, rows.0 + row as f32 * h, w, h),
                    image,
                    color,
                });
            }
        }
    }
    scene.commands.push(DrawCommand::PopClip);
}
