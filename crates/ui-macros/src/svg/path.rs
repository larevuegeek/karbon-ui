//! Géométrie : données de chemin (`d`) et formes de base (cercle, ellipse, rectangle
//! arrondi, lignes, polygones) ramenées à des opérations absolues `Op` ; arcs et
//! quadratiques deviennent des cubiques.
use super::paint::{literal, number};
use super::scanner::{Scanner, numbers};
use crate::{
    Result,
    template::markup::{Attr, Element},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Op {
    Move(f64, f64),
    Line(f64, f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    Close,
}
/// Analyse un attribut `d` complet (M L H V C S Q T A Z, absolus et relatifs) en
/// commandes absolues ; les arcs et quadratiques deviennent des cubiques.
fn path_data(d: &str) -> std::result::Result<Vec<Op>, String> {
    let mut s = Scanner::new(d);
    let mut ops = Vec::new();
    let (mut x, mut y) = (0f64, 0f64);
    let (mut sx, mut sy) = (0f64, 0f64);
    // Dernier point de contrôle (réflexion de S/T) et type de la commande précédente.
    let mut control: Option<(f64, f64)> = None;
    let mut quad_control: Option<(f64, f64)> = None;
    let mut command: Option<u8> = None;
    let mut started = false;
    loop {
        s.separators();
        if s.done() {
            break;
        }
        let c = match s.peek() {
            Some(b) if b.is_ascii_alphabetic() => {
                s.at += 1;
                b
            }
            Some(_) if s.starts_number() => match command {
                // Coordonnées répétées : après M/m, la suite est une ligne.
                Some(b'M') => b'L',
                Some(b'm') => b'l',
                Some(b'Z' | b'z') | None => {
                    return Err(format!("Path data must start with M, at offset {}", s.at));
                }
                Some(other) => other,
            },
            Some(other) => {
                return Err(format!(
                    "Unexpected character '{}' in path data at offset {}",
                    other as char, s.at
                ));
            }
            None => break,
        };
        if !started && !matches!(c, b'M' | b'm') {
            return Err("Path data must start with M or m".into());
        }
        started = true;
        let relative = c.is_ascii_lowercase();
        let (ox, oy) = if relative { (x, y) } else { (0., 0.) };
        let mut n = || -> std::result::Result<f64, String> {
            s.separators();
            s.number()
        };
        let mut next_control = None;
        let mut next_quad = None;
        match c.to_ascii_uppercase() {
            b'M' => {
                x = ox + n()?;
                y = oy + n()?;
                sx = x;
                sy = y;
                ops.push(Op::Move(x, y));
            }
            b'L' => {
                x = ox + n()?;
                y = oy + n()?;
                ops.push(Op::Line(x, y));
            }
            b'H' => {
                x = ox + n()?;
                ops.push(Op::Line(x, y));
            }
            b'V' => {
                y = oy + n()?;
                ops.push(Op::Line(x, y));
            }
            b'C' => {
                let (x1, y1) = (ox + n()?, oy + n()?);
                let (x2, y2) = (ox + n()?, oy + n()?);
                x = ox + n()?;
                y = oy + n()?;
                ops.push(Op::Cubic(x1, y1, x2, y2, x, y));
                next_control = Some((x2, y2));
            }
            b'S' => {
                let (x1, y1) = match control {
                    Some((cx, cy)) => (2. * x - cx, 2. * y - cy),
                    None => (x, y),
                };
                let (x2, y2) = (ox + n()?, oy + n()?);
                x = ox + n()?;
                y = oy + n()?;
                ops.push(Op::Cubic(x1, y1, x2, y2, x, y));
                next_control = Some((x2, y2));
            }
            b'Q' => {
                let (qx, qy) = (ox + n()?, oy + n()?);
                let (ex, ey) = (ox + n()?, oy + n()?);
                ops.push(quadratic((x, y), (qx, qy), (ex, ey)));
                (x, y) = (ex, ey);
                next_quad = Some((qx, qy));
            }
            b'T' => {
                let (qx, qy) = match quad_control {
                    Some((cx, cy)) => (2. * x - cx, 2. * y - cy),
                    None => (x, y),
                };
                let (ex, ey) = (ox + n()?, oy + n()?);
                ops.push(quadratic((x, y), (qx, qy), (ex, ey)));
                (x, y) = (ex, ey);
                next_quad = Some((qx, qy));
            }
            b'A' => {
                let rx = n()?;
                let ry = n()?;
                let rotation = n()?;
                let large = s.flag()?;
                let sweep = s.flag()?;
                let ex = ox + {
                    s.separators();
                    s.number()?
                };
                let ey = oy + {
                    s.separators();
                    s.number()?
                };
                arc(&mut ops, (x, y), rx, ry, rotation, large, sweep, (ex, ey));
                (x, y) = (ex, ey);
            }
            b'Z' => {
                ops.push(Op::Close);
                x = sx;
                y = sy;
            }
            other => {
                return Err(format!(
                    "Unknown path command '{}' (expected M L H V C S Q T A Z)",
                    other as char
                ));
            }
        }
        control = next_control;
        quad_control = next_quad;
        command = Some(c);
    }
    Ok(ops)
}
fn quadratic((x0, y0): (f64, f64), (qx, qy): (f64, f64), (x, y): (f64, f64)) -> Op {
    Op::Cubic(
        x0 + 2. / 3. * (qx - x0),
        y0 + 2. / 3. * (qy - y0),
        x + 2. / 3. * (qx - x),
        y + 2. / 3. * (qy - y),
        x,
        y,
    )
}
/// Arc elliptique (paramétrage par extrémités, SVG 1.1 §F.6.5) en cubiques d'au plus 90°.
#[allow(clippy::too_many_arguments)]
pub(super) fn arc(
    ops: &mut Vec<Op>,
    (x1, y1): (f64, f64),
    rx: f64,
    ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
    (x2, y2): (f64, f64),
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if (x1 == x2 && y1 == y2) || rx == 0. || ry == 0. {
        if x1 != x2 || y1 != y2 {
            ops.push(Op::Line(x2, y2));
        }
        return;
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let (dx, dy) = ((x1 - x2) / 2., (y1 - y2) / 2.);
    let x1p = cos * dx + sin * dy;
    let y1p = -sin * dx + cos * dy;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1. {
        let k = lambda.sqrt();
        rx *= k;
        ry *= k;
    }
    let numerator = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let denominator = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coefficient = (numerator / denominator).max(0.).sqrt();
    if large == sweep {
        coefficient = -coefficient;
    }
    let cxp = coefficient * rx * y1p / ry;
    let cyp = -coefficient * ry * x1p / rx;
    let cx = cos * cxp - sin * cyp + (x1 + x2) / 2.;
    let cy = sin * cxp + cos * cyp + (y1 + y2) / 2.;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let a = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        if a.is_finite() { a } else { 0. }
    };
    let (ux, uy) = ((x1p - cxp) / rx, (y1p - cyp) / ry);
    let (vx, vy) = ((-x1p - cxp) / rx, (-y1p - cyp) / ry);
    let start = angle(1., 0., ux, uy);
    let mut delta = angle(ux, uy, vx, vy);
    if !sweep && delta > 0. {
        delta -= std::f64::consts::TAU;
    } else if sweep && delta < 0. {
        delta += std::f64::consts::TAU;
    }
    let segments = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let step = delta / segments as f64;
    let k = 4. / 3. * (step / 4.).tan();
    let point = |t: f64| {
        let (s, c) = t.sin_cos();
        let (px, py) = (rx * c, ry * s);
        (cos * px - sin * py + cx, sin * px + cos * py + cy)
    };
    let derivative = |t: f64| {
        let (s, c) = t.sin_cos();
        let (px, py) = (-rx * s, ry * c);
        (cos * px - sin * py, sin * px + cos * py)
    };
    let mut t = start;
    for i in 0..segments {
        let t2 = t + step;
        let (p1x, p1y) = point(t);
        let (d1x, d1y) = derivative(t);
        let (p2x, p2y) = if i + 1 == segments {
            (x2, y2)
        } else {
            point(t2)
        };
        let (d2x, d2y) = derivative(t2);
        ops.push(Op::Cubic(
            p1x + k * d1x,
            p1y + k * d1y,
            p2x - k * d2x,
            p2y - k * d2y,
            p2x,
            p2y,
        ));
        t = t2;
    }
}
// -- Formes --------------------------------------------------------------------
/// Quart d'ellipse en cubique (constante de Bézier).
const KAPPA: f64 = 0.552_284_749_830_793_6;
pub(super) fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<Op> {
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    vec![
        Op::Move(cx + rx, cy),
        Op::Cubic(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry),
        Op::Cubic(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy),
        Op::Cubic(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry),
        Op::Cubic(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy),
        Op::Close,
    ]
}
pub(super) fn rectangle(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64) -> Vec<Op> {
    if rx <= 0. || ry <= 0. {
        return vec![
            Op::Move(x, y),
            Op::Line(x + w, y),
            Op::Line(x + w, y + h),
            Op::Line(x, y + h),
            Op::Close,
        ];
    }
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let (r, b) = (x + w, y + h);
    vec![
        Op::Move(x + rx, y),
        Op::Line(r - rx, y),
        Op::Cubic(r - rx + kx, y, r, y + ry - ky, r, y + ry),
        Op::Line(r, b - ry),
        Op::Cubic(r, b - ry + ky, r - rx + kx, b, r - rx, b),
        Op::Line(x + rx, b),
        Op::Cubic(x + rx - kx, b, x, b - ry + ky, x, b - ry),
        Op::Line(x, y + ry),
        Op::Cubic(x, y + ry - ky, x + rx - kx, y, x + rx, y),
        Op::Close,
    ]
}
pub(super) fn points(a: &Attr) -> Result<Vec<(f64, f64)>> {
    let values = numbers(literal(a)?).map_err(|m| a.loc.error(m))?;
    if values.len() % 2 == 1 {
        return Err(a.loc.error("points needs an even number of coordinates"));
    }
    Ok(values.chunks(2).map(|p| (p[0], p[1])).collect())
}
/// Valeur numérique d'un attribut de géométrie (0 par défaut).
pub(super) fn geometry(e: &Element, name: &str) -> Result<Option<f64>> {
    e.attr(name).map(number).transpose()
}
pub(super) fn shape_ops(e: &Element) -> Result<(Vec<Op>, &'static [&'static str])> {
    let get = |name| geometry(e, name).map(|v| v.unwrap_or(0.));
    Ok(match e.name.as_str() {
        "path" => {
            let ops = match e.attr("d") {
                Some(a) => path_data(literal(a)?).map_err(|m| a.loc.error(m))?,
                None => Vec::new(),
            };
            (ops, &["d"])
        }
        "circle" => {
            let r = get("r")?;
            if r < 0. {
                return Err(e.loc.error("circle r cannot be negative"));
            }
            let ops = if r > 0. {
                ellipse(get("cx")?, get("cy")?, r, r)
            } else {
                Vec::new()
            };
            (ops, &["cx", "cy", "r"])
        }
        "ellipse" => {
            let (rx, ry) = (get("rx")?, get("ry")?);
            if rx < 0. || ry < 0. {
                return Err(e.loc.error("ellipse radii cannot be negative"));
            }
            let ops = if rx > 0. && ry > 0. {
                ellipse(get("cx")?, get("cy")?, rx, ry)
            } else {
                Vec::new()
            };
            (ops, &["cx", "cy", "rx", "ry"])
        }
        "rect" => {
            let (w, h) = (get("width")?, get("height")?);
            if w < 0. || h < 0. {
                return Err(e.loc.error("rect width and height cannot be negative"));
            }
            // rx/ry : l'un donne l'autre s'il manque, bornés à la moitié des côtés.
            let (rx, ry) = match (geometry(e, "rx")?, geometry(e, "ry")?) {
                (Some(rx), Some(ry)) => (rx, ry),
                (Some(r), None) | (None, Some(r)) => (r, r),
                (None, None) => (0., 0.),
            };
            let ops = if w > 0. && h > 0. {
                rectangle(
                    get("x")?,
                    get("y")?,
                    w,
                    h,
                    rx.clamp(0., w / 2.),
                    ry.clamp(0., h / 2.),
                )
            } else {
                Vec::new()
            };
            (ops, &["x", "y", "width", "height", "rx", "ry"])
        }
        "line" => (
            vec![
                Op::Move(get("x1")?, get("y1")?),
                Op::Line(get("x2")?, get("y2")?),
            ],
            &["x1", "y1", "x2", "y2"],
        ),
        "polyline" | "polygon" => {
            let list = match e.attr("points") {
                Some(a) => points(a)?,
                None => Vec::new(),
            };
            let mut ops = Vec::new();
            for (i, (x, y)) in list.iter().enumerate() {
                ops.push(if i == 0 {
                    Op::Move(*x, *y)
                } else {
                    Op::Line(*x, *y)
                });
            }
            if e.name == "polygon" && !ops.is_empty() {
                ops.push(Op::Close);
            }
            (ops, &["points"])
        }
        _ => unreachable!("checked by the caller"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_data_supports_every_command_and_compact_syntax() {
        let ops = path_data("M10 10h5v5H10V10zm2,2l1-1 1 1L14 14").unwrap();
        assert_eq!(ops[0], Op::Move(10., 10.));
        assert_eq!(ops[1], Op::Line(15., 10.));
        assert_eq!(ops[2], Op::Line(15., 15.));
        assert_eq!(ops[5], Op::Close);
        // Après Z, « m » est relatif au début du sous-chemin.
        assert_eq!(ops[6], Op::Move(12., 12.));
        assert_eq!(ops[7], Op::Line(13., 11.));
        assert_eq!(ops[8], Op::Line(14., 12.));
        let ops = path_data("M0 0c1 2 3 4 5 6s1 1 2 2q1 1 2 0t2 0").unwrap();
        assert_eq!(ops.len(), 5);
        assert!(matches!(ops[2], Op::Cubic(x1, y1, ..) if x1 == 7. && y1 == 8.));
        let ops = path_data("M.5.5-1-1").unwrap();
        assert_eq!(ops, vec![Op::Move(0.5, 0.5), Op::Line(-1., -1.)]);
        assert!(path_data("L1 1").is_err());
        assert!(path_data("M0 0 X").is_err());
    }
    #[test]
    fn arcs_become_cubics_ending_on_the_target_point() {
        let ops = path_data("M0 0a10 10 0 1 0 20 0").unwrap();
        assert_eq!(ops.len(), 3);
        let Op::Cubic(.., x, y) = ops[2] else {
            panic!("cubic expected")
        };
        assert!((x - 20.).abs() < 1e-9 && y.abs() < 1e-9);
        // Point milieu du demi-cercle (balayage 0, sens négatif) : sous le segment, y vers le bas.
        let Op::Cubic(.., mx, my) = ops[1] else {
            panic!("cubic expected")
        };
        assert!(
            (mx - 10.).abs() < 1e-6 && (my - 10.).abs() < 1e-6,
            "{mx} {my}"
        );
        // Drapeaux collés.
        assert_eq!(path_data("M0 0a5 5 0 0120 0").unwrap().len(), 3);
    }
}
