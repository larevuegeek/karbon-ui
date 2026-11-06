//! Bordures par côté : contour arrondi uniforme, jonctions en onglet ou bandes rognées.
use super::painted;
use crate::ui::{
    element::Element,
    layout,
    scene::{DrawCommand, PointF, RectF, Scene},
};
/// Bordures dont un côté a sa propre largeur ou couleur ; couleur du côté, puis
/// `border-color`, puis texte. Coins arrondis exacts quand les côtés présents partagent
/// largeur et couleur, sinon bandes droites rognées à la forme extérieure.
pub(super) fn border_sides(node: &Element, scene: &mut Scene, theme: crate::ui::theme::Theme) {
    let Some(styles) = &node.styles else {
        return;
    };
    let local = styles.computed.local;
    let fallback = local
        .border
        .or_else(|| {
            node.surface_style
                .as_deref()
                .and_then(|s| s.border)
                .map(|c| c.resolve(theme))
        })
        .or(styles.computed.text.color)
        .unwrap_or(theme.text);
    let b = node.border();
    let r = painted(node);
    let (w, h) = (layout::width(r), layout::height(r));
    let widths = [b.top, b.right, b.bottom, b.left];
    let colors = local.border_colors.map(|c| c.unwrap_or(fallback));
    let radii = local
        .corner_radii
        .unwrap_or([node.style.radius; 4])
        .map(|n| n.max(0.));
    let rounded = radii.iter().any(|n| *n > 0.);
    // Coins arrondis, côtés présents de même largeur et de même couleur (`border-b-0`…) :
    // contour arrondi uniforme sur une boîte prolongée au-delà des côtés absents, rogné à
    // la boîte de bordure.
    let present: Vec<usize> = (0..4).filter(|i| widths[*i] > 0.).collect();
    if rounded
        && let Some(&first) = present.first()
        && present
            .iter()
            .all(|i| widths[*i] == widths[first] && colors[*i] == colors[first])
    {
        let width = widths[first];
        let grow = |i: usize| if widths[i] > 0. { 0. } else { width };
        let rect = RectF {
            left: r.left - grow(3),
            top: r.top - grow(0),
            right: r.right + grow(1),
            bottom: r.bottom + grow(2),
        };
        scene.commands.push(DrawCommand::PushClip(r));
        scene.commands.push(DrawCommand::Shape {
            rect,
            radii,
            fill: None,
            stroke: Some((width, colors[first])),
        });
        scene.commands.push(DrawCommand::PopClip);
        return;
    }
    // Côté épais (2 px et plus) voisin d'un côté transparent : jonctions en onglet comme
    // les navigateurs (triangle de `border-top` avec côtés transparents, arc de
    // `.spinner-border`), là où les bandes déborderaient sur le côté invisible. Les autres
    // bordures par côté gardent les bandes ci-dessous.
    let visible = |i: usize| widths[i] > 0. && colors[i].alpha > 0;
    let mitred = (0..4).any(|i| {
        let j = (i + 1) % 4;
        widths[i] > 0.
            && widths[j] > 0.
            && widths[i].max(widths[j]) >= 2.
            && visible(i) != visible(j)
    });
    if mitred {
        // Côtés voisins de même couleur peints d'un seul tenant (pas de couture
        // d'anticrénelage entre eux).
        let same = |a: usize, b: usize| visible(a) && visible(b) && colors[a] == colors[b];
        let first = (0..4).find(|&i| !same((i + 3) % 4, i)).unwrap_or(0);
        let mut side = first;
        loop {
            let mut count = 1;
            while count < 4 && same((side + count - 1) % 4, (side + count) % 4) {
                count += 1;
            }
            if visible(side) {
                let points = if rounded {
                    rounded_sides(r, widths, radii, side, count)
                } else if count == 1 {
                    mitred_side(r, widths, side)
                } else {
                    mitred_run(r, widths, side, count)
                };
                if points.len() >= 3 {
                    scene.commands.push(DrawCommand::Path {
                        points,
                        closed: true,
                        color: colors[side],
                    });
                }
            }
            side = (side + count) % 4;
            if side == first {
                break;
            }
        }
        return;
    }
    // Sinon une bande par côté, rognée à la forme arrondie extérieure.
    if rounded {
        scene
            .commands
            .push(DrawCommand::PushShapeClip { rect: r, radii });
    }
    let sides = [
        RectF::new(r.left, r.top, w, b.top),
        RectF::new(r.right - b.right, r.top, b.right, h),
        RectF::new(r.left, r.bottom - b.bottom, w, b.bottom),
        RectF::new(r.left, r.top, b.left, h),
    ];
    for (index, side) in sides.into_iter().enumerate() {
        if layout::width(side) > 0. && layout::height(side) > 0. {
            scene.fill(side, 0., colors[index]);
        }
    }
    if rounded {
        scene.commands.push(DrawCommand::PopShapeClip);
    }
}
/// Trapèze d'un côté de bordure d'une boîte à angles droits : des coins extérieurs aux
/// coins de la boîte de remplissage (côtés haut, droite, bas, gauche).
fn mitred_side(r: RectF, widths: [f32; 4], side: usize) -> Vec<PointF> {
    let [t, rt, b, l] = widths;
    let p = |x: f32, y: f32| PointF { x, y };
    // Boîte intérieure, réduite à un segment quand les bordures se rejoignent.
    let (il, ir) = {
        let (a, z) = (r.left + l, r.right - rt);
        if a <= z {
            (a, z)
        } else {
            let m = r.left + (r.right - r.left) * l / (l + rt).max(f32::EPSILON);
            (m, m)
        }
    };
    let (it, ib) = {
        let (a, z) = (r.top + t, r.bottom - b);
        if a <= z {
            (a, z)
        } else {
            let m = r.top + (r.bottom - r.top) * t / (t + b).max(f32::EPSILON);
            (m, m)
        }
    };
    let mut points = match side {
        0 => vec![p(r.left, r.top), p(r.right, r.top), p(ir, it), p(il, it)],
        1 => vec![
            p(r.right, r.top),
            p(r.right, r.bottom),
            p(ir, ib),
            p(ir, it),
        ],
        2 => vec![
            p(r.right, r.bottom),
            p(r.left, r.bottom),
            p(il, ib),
            p(ir, ib),
        ],
        _ => vec![p(r.left, r.bottom), p(r.left, r.top), p(il, it), p(il, ib)],
    };
    // Sommets confondus (triangle) : un seul.
    points.dedup_by(|a, b| (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4);
    points
}
/// Côtés consécutifs d'une boîte à angles droits réunis : coins extérieurs, puis coins
/// de la boîte de remplissage en sens inverse.
fn mitred_run(r: RectF, widths: [f32; 4], side: usize, count: usize) -> Vec<PointF> {
    let outer = [
        PointF {
            x: r.left,
            y: r.top,
        },
        PointF {
            x: r.right,
            y: r.top,
        },
        PointF {
            x: r.right,
            y: r.bottom,
        },
        PointF {
            x: r.left,
            y: r.bottom,
        },
    ];
    let [t, rt, b, l] = widths;
    let inner = [
        PointF {
            x: r.left + l,
            y: r.top + t,
        },
        PointF {
            x: r.right - rt,
            y: r.top + t,
        },
        PointF {
            x: r.right - rt,
            y: r.bottom - b,
        },
        PointF {
            x: r.left + l,
            y: r.bottom - b,
        },
    ];
    let mut points = (0..=count)
        .map(|k| outer[(side + k) % 4])
        .collect::<Vec<_>>();
    points.extend((0..=count).rev().map(|k| inner[(side + k) % 4]));
    points
}
/// Distance du centre au bord d'un rectangle arrondi dans la direction `angle`.
fn radial(r: RectF, radii: [f32; 4], center: PointF, angle: f32) -> f32 {
    let (dx, dy) = (angle.cos(), angle.sin());
    let inside = |d: f32| {
        let (x, y) = (center.x + dx * d, center.y + dy * d);
        if x < r.left || x > r.right || y < r.top || y > r.bottom {
            return false;
        }
        // Coin concerné : haut-gauche, haut-droit, bas-droit, bas-gauche.
        let corners = [
            (r.left + radii[0], r.top + radii[0], radii[0]),
            (r.right - radii[1], r.top + radii[1], radii[1]),
            (r.right - radii[2], r.bottom - radii[2], radii[2]),
            (r.left + radii[3], r.bottom - radii[3], radii[3]),
        ];
        corners.iter().enumerate().all(|(i, &(cx, cy, radius))| {
            let outside_x = if i == 0 || i == 3 { x < cx } else { x > cx };
            let outside_y = if i < 2 { y < cy } else { y > cy };
            !(outside_x && outside_y) || (x - cx).powi(2) + (y - cy).powi(2) <= radius * radius
        })
    };
    let (mut low, mut high) = (0., (r.right - r.left).hypot(r.bottom - r.top));
    if !inside(low) {
        return 0.;
    }
    for _ in 0..24 {
        let mid = (low + high) / 2.;
        if inside(mid) {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}
/// Secteur d'un côté de bordure d'une boîte arrondie : entre les rayons issus du centre
/// vers les coins extérieurs, du bord extérieur au bord de la boîte de remplissage.
fn rounded_sides(
    r: RectF,
    widths: [f32; 4],
    radii: [f32; 4],
    side: usize,
    count: usize,
) -> Vec<PointF> {
    let [t, rt, b, l] = widths;
    // Rayons réduits comme en CSS : deux coins voisins ne dépassent pas leur côté.
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let f = [
        w / (radii[0] + radii[1]),
        h / (radii[1] + radii[2]),
        w / (radii[2] + radii[3]),
        h / (radii[3] + radii[0]),
    ]
    .into_iter()
    .filter(|f| f.is_finite())
    .fold(1f32, f32::min);
    let radii = radii.map(|n| n * f);
    let center = PointF {
        x: (r.left + r.right) / 2.,
        y: (r.top + r.bottom) / 2.,
    };
    let inner = RectF {
        left: r.left + l,
        top: r.top + t,
        right: r.right - rt,
        bottom: r.bottom - b,
    };
    // Rayons intérieurs : rayon extérieur moins la plus grande des deux largeurs.
    let inner_radii = [
        (radii[0] - t.max(l)).max(0.),
        (radii[1] - t.max(rt)).max(0.),
        (radii[2] - b.max(rt)).max(0.),
        (radii[3] - b.max(l)).max(0.),
    ];
    let corner = |x: f32, y: f32| (y - center.y).atan2(x - center.x);
    let corners = [
        corner(r.left, r.top),
        corner(r.right, r.top),
        corner(r.right, r.bottom),
        corner(r.left, r.bottom),
    ];
    // Côté haut : du coin haut-gauche au coin haut-droit (sens horaire à l'écran) ;
    // `count` côtés consécutifs à partir de `side`.
    let start = corners[side % 4];
    let mut end = corners[(side + count) % 4];
    while end <= start {
        end += std::f32::consts::TAU;
    }
    let steps = 64 * count;
    let empty = inner.right <= inner.left || inner.bottom <= inner.top;
    let mut outer = Vec::with_capacity(steps + 1);
    let mut inside = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let a = start + (end - start) * k as f32 / steps as f32;
        let (c, s) = (a.cos(), a.sin());
        let d = radial(r, radii, center, a);
        outer.push(PointF {
            x: center.x + c * d,
            y: center.y + s * d,
        });
        if !empty {
            let d = radial(inner, inner_radii, center, a);
            inside.push(PointF {
                x: center.x + c * d,
                y: center.y + s * d,
            });
        }
    }
    if empty {
        outer.push(center);
    } else {
        outer.extend(inside.into_iter().rev());
    }
    outer
}
