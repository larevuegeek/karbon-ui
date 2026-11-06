//! Décorations peintes par le runtime par-dessus le contenu : barres de défilement et
//! séparateur d'un split.
use super::super::PaintState;
use crate::ui::{
    element::*,
    layout,
    scene::{DrawCommand, RectF, Scene},
    theme::Color,
};
/// Barre de défilement d'une zone défilante, et cadre du viewport quand elle a le focus
/// clavier.
pub(super) fn scroll_bar(
    node: &Element,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
    runtime: PaintState,
) {
    if let Some(s) = node.scroll {
        let bar = node
            .rules
            .as_ref()
            .map_or(Default::default(), |r| r.scrollbar);
        let active = runtime.hovered == Some(node.id) || runtime.focused == Some(node.id);
        if s.max_offset() > 0. && s.thickness > 0. {
            match bar.width {
                ScrollbarWidth::Thin => {
                    // Barre fine : piste transparente par défaut, pouce arrondi plus
                    // opaque au survol de la barre (comme Chromium).
                    if let Some(track) = bar.track.filter(|c| c.alpha > 0) {
                        scene.fill(s.track, 0., track);
                    }
                    let thumb = bar.thumb.unwrap_or(Color {
                        alpha: 110,
                        ..theme.muted
                    });
                    let thumb = if runtime.hovered == Some(node.id) {
                        Color {
                            alpha: (f32::from(thumb.alpha) * 1.55).min(255.) as u8,
                            ..thumb
                        }
                    } else {
                        thumb
                    };
                    let radius = layout::width(s.thumb).min(layout::height(s.thumb)) / 2.;
                    scene.fill(s.thumb, radius, thumb);
                }
                _ => {
                    scene.fill(s.track, 0., bar.track.unwrap_or(theme.surface));
                    scene.fill(
                        s.thumb,
                        3.,
                        match bar.thumb {
                            Some(color) => color,
                            None if active => theme.accent,
                            None => theme.muted,
                        },
                    );
                }
            }
        }
        if runtime.focus_visible && runtime.focused == Some(node.id) {
            scene.commands.push(DrawCommand::Stroke {
                rect: s.viewport,
                radius: 0.,
                width: 1.,
                color: theme.accent,
            });
        }
    }
}
/// Ligne du séparateur d'un split, accentuée au survol ou au focus.
pub(super) fn divider(
    node: &Element,
    scene: &mut Scene,
    theme: crate::ui::theme::Theme,
    runtime: PaintState,
) {
    if let (Layout::Split(split), Some(rect)) = (node.layout, node.divider) {
        let active = split.resizable
            && (runtime.hovered == Some(node.id) || runtime.focused == Some(node.id));
        let line = if split.axis == Axis::Horizontal {
            RectF::new(
                rect.left + layout::DIVIDER_SIZE / 2. - 1.,
                rect.top,
                2.,
                layout::height(rect),
            )
        } else {
            RectF::new(
                rect.left,
                rect.top + layout::DIVIDER_SIZE / 2. - 1.,
                layout::width(rect),
                2.,
            )
        };
        scene.fill(line, 1., if active { theme.accent } else { theme.border });
    }
}
