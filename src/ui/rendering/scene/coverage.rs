//! Couverture d'une scène, à la place du test alpha des fenêtres layered.
//!
//! Indépendant de l'OS : tout backend qui présente une fenêtre transparente sans test
//! alpha natif peut s'en servir (Windows : WM_NCHITTEST des fenêtres composées).
//!
//! Une fenêtre layered ignore les clics sur ses pixels d'alpha nul. Une fenêtre
//! composée reçoit tous les clics : WM_NCHITTEST interroge donc la scène. L'estimation
//! utilise les contours des primitives visibles (rectangles arrondis, traits, boîtes du
//! texte et des images), avec transformations, clips et opacité nulle. Elle est
//! légèrement plus généreuse que l'alpha réel (entre les glyphes, dans une image).
use crate::ui::{
    layout::{contains, contains_rounded},
    rendering::{brush::Background, transform::Affine},
    scene::{DrawCommand, PointF, RectF, Scene},
};

enum Scope {
    Effect {
        transform: Affine,
        local: Option<PointF>,
        excluded: bool,
    },
    Clip {
        excluded: bool,
    },
}

/// Vrai si une primitive visible peut couvrir `point` (coordonnées logiques du client).
// Seul le backend Windows s'en sert aujourd'hui (fenêtres composées transparentes).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn covers(scene: &Scene, point: PointF) -> bool {
    if scene.clear.alpha > 0 {
        return true;
    }
    let mut transform = Affine::IDENTITY;
    let mut local = Some(point);
    let mut excluded = 0usize;
    let mut scopes = Vec::new();
    for command in &scene.commands {
        match command {
            DrawCommand::PushEffect {
                transform: effect,
                opacity,
                ..
            } => {
                scopes.push(Scope::Effect {
                    transform,
                    local,
                    excluded: false,
                });
                transform = effect.then(transform);
                local = transform.inverse().map(|inverse| inverse.point(point));
                let out = *opacity <= 0. || local.is_none();
                if let Some(Scope::Effect { excluded, .. }) = scopes.last_mut() {
                    *excluded = out;
                }
                excluded += usize::from(out);
            }
            DrawCommand::PushClip(rect) => {
                let out = !local.is_some_and(|p| contains(*rect, p.x, p.y));
                excluded += usize::from(out);
                scopes.push(Scope::Clip { excluded: out });
            }
            DrawCommand::PushShapeClip { rect, radii } => {
                let out = !local.is_some_and(|p| contains_rounded(*rect, *radii, p));
                excluded += usize::from(out);
                scopes.push(Scope::Clip { excluded: out });
            }
            // Les filtres dessinent le contenu qu'ils encadrent : seul leur débordement
            // (flou, ombre portée) n'est pas compté.
            DrawCommand::PushFilter { .. }
            | DrawCommand::PushMask { .. }
            | DrawCommand::PushTextFill { .. } => scopes.push(Scope::Clip { excluded: false }),
            DrawCommand::PopEffect
            | DrawCommand::PopClip
            | DrawCommand::PopShapeClip
            | DrawCommand::PopFilter
            | DrawCommand::PopMask
            | DrawCommand::PopTextFill => match scopes.pop() {
                Some(Scope::Effect {
                    transform: previous,
                    local: previous_local,
                    excluded: out,
                }) => {
                    transform = previous;
                    local = previous_local;
                    excluded -= usize::from(out);
                }
                Some(Scope::Clip { excluded: out }) => excluded -= usize::from(out),
                None => {}
            },
            _ => {
                if excluded == 0
                    && let Some(p) = local
                    && primitive(command, p)
                {
                    return true;
                }
            }
        }
    }
    false
}

fn primitive(command: &DrawCommand, p: PointF) -> bool {
    match command {
        DrawCommand::Fill {
            rect,
            radius,
            color,
        } => color.alpha > 0 && contains_rounded(*rect, [*radius; 4], p),
        DrawCommand::Shape {
            rect,
            radii,
            fill,
            stroke,
        } => {
            let filled = match fill {
                Some(Background::Solid(color)) => color.alpha > 0,
                Some(_) => true,
                None => false,
            };
            (filled && contains_rounded(*rect, *radii, p))
                || stroke.is_some_and(|(width, color)| color.alpha > 0 && ring(*rect, width, p))
        }
        DrawCommand::Gradient { rect, radius, .. }
        | DrawCommand::RadialGradient { rect, radius, .. } => {
            contains_rounded(*rect, [*radius; 4], p)
        }
        DrawCommand::Stroke {
            rect, width, color, ..
        } => color.alpha > 0 && ring(*rect, *width, p),
        DrawCommand::Line {
            start,
            end,
            width,
            color,
        } => color.alpha > 0 && near_segment(p, *start, *end, *width / 2.),
        DrawCommand::Path {
            points,
            closed,
            color,
        } => {
            color.alpha > 0
                && if *closed {
                    bounds(points).is_some_and(|r| contains(r, p.x, p.y))
                } else {
                    // Les chemins ouverts sont tracés avec une épaisseur de 2.
                    points
                        .windows(2)
                        .any(|pair| near_segment(p, pair[0], pair[1], 1.))
                }
        }
        DrawCommand::Text { rect, color, .. }
        | DrawCommand::FontText { rect, color, .. }
        | DrawCommand::FormattedText { rect, color, .. }
        | DrawCommand::RichText { rect, color, .. }
        | DrawCommand::Vector { rect, color, .. } => color.alpha > 0 && contains(*rect, p.x, p.y),
        DrawCommand::Svg { rect, .. } => contains(*rect, p.x, p.y),
        DrawCommand::Image { rect, .. } => contains(*rect, p.x, p.y),
        _ => false,
    }
}

/// Trait centré sur le contour du rectangle.
fn ring(rect: RectF, width: f32, p: PointF) -> bool {
    let half = width.max(1.) / 2.;
    let outer = RectF {
        left: rect.left - half,
        top: rect.top - half,
        right: rect.right + half,
        bottom: rect.bottom + half,
    };
    let inner = RectF {
        left: rect.left + half,
        top: rect.top + half,
        right: rect.right - half,
        bottom: rect.bottom - half,
    };
    contains(outer, p.x, p.y) && !contains(inner, p.x, p.y)
}

fn near_segment(p: PointF, a: PointF, b: PointF, half: f32) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0. {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0., 1.)
    } else {
        0.
    };
    let (x, y) = (a.x + t * dx - p.x, a.y + t * dy - p.y);
    x * x + y * y <= half.max(0.5).powi(2)
}

fn bounds(points: &[PointF]) -> Option<RectF> {
    let first = points.first()?;
    Some(points.iter().fold(
        RectF {
            left: first.x,
            top: first.y,
            right: first.x,
            bottom: first.y,
        },
        |r, p| RectF {
            left: r.left.min(p.x),
            top: r.top.min(p.y),
            right: r.right.max(p.x),
            bottom: r.bottom.max(p.y),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Color;
    fn at(x: f32, y: f32) -> PointF {
        PointF { x, y }
    }
    #[test]
    fn rounded_window_background_leaves_its_corners_uncovered() {
        let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
        scene.fill(RectF::new(0., 0., 200., 100.), 12., Color::rgb(10, 15, 12));
        assert!(covers(&scene, at(100., 50.)));
        assert!(covers(&scene, at(12., 0.5)));
        assert!(!covers(&scene, at(0.5, 0.5)));
        assert!(!covers(&scene, at(199.5, 99.5)));
        assert!(!covers(&scene, at(250., 50.)));
        // Un fond opaque couvre tout.
        assert!(covers(&Scene::new(Color::rgb(0, 0, 0)), at(0., 0.)));
    }
    #[test]
    fn transparent_scene_only_covers_visible_primitives() {
        let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
        scene.fill(RectF::new(0., 0., 50., 50.), 0., Color::rgba(0, 0, 0, 0));
        scene.commands.push(DrawCommand::Stroke {
            rect: RectF::new(100., 0., 50., 50.),
            radius: 0.,
            width: 2.,
            color: Color::rgb(255, 0, 0),
        });
        scene.text(
            RectF::new(0., 100., 80., 20.),
            "Texte",
            12.,
            false,
            Color::rgb(255, 255, 255),
        );
        assert!(!covers(&scene, at(25., 25.)), "remplissage invisible");
        assert!(covers(&scene, at(100., 25.)), "trait du contour");
        assert!(!covers(&scene, at(125., 25.)), "intérieur d'un contour");
        assert!(covers(&scene, at(40., 110.)));
    }
    #[test]
    fn transforms_clips_and_zero_opacity_are_respected() {
        let card = |scene: &mut Scene| {
            scene.fill(RectF::new(0., 0., 40., 40.), 0., Color::rgb(1, 2, 3));
        };
        let mut scene = Scene::new(Color::rgba(0, 0, 0, 0));
        scene.commands.push(DrawCommand::PushEffect {
            transform: Affine::translation(100., 100.),
            opacity: 1.,
            bounds: RectF::new(0., 0., 40., 40.),
        });
        card(&mut scene);
        scene.commands.push(DrawCommand::PopEffect);
        assert!(covers(&scene, at(120., 120.)));
        assert!(!covers(&scene, at(20., 20.)));

        let mut hidden = Scene::new(Color::rgba(0, 0, 0, 0));
        hidden.commands.push(DrawCommand::PushEffect {
            transform: Affine::IDENTITY,
            opacity: 0.,
            bounds: RectF::new(0., 0., 40., 40.),
        });
        card(&mut hidden);
        hidden.commands.push(DrawCommand::PopEffect);
        assert!(!covers(&hidden, at(20., 20.)));

        let mut clipped = Scene::new(Color::rgba(0, 0, 0, 0));
        clipped
            .commands
            .push(DrawCommand::PushClip(RectF::new(0., 0., 10., 10.)));
        card(&mut clipped);
        clipped.commands.push(DrawCommand::PopClip);
        card(&mut clipped);
        assert!(covers(&clipped, at(5., 5.)));
        // Le second dessin, hors clip, couvre à nouveau toute la carte.
        assert!(covers(&clipped, at(20., 20.)));
        clipped.commands.pop();
        assert!(!covers(&clipped, at(20., 20.)));
    }
}
