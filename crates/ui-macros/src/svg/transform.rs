//! Matrices affines des attributs `transform` : lecture des listes
//! (`matrix`, `translate`, `scale`, `rotate`, `skewX`, `skewY`) et composition.
use super::scanner::numbers;
/// Matrice SVG `[a, b, c, d, e, f]` : x' = a·x + c·y + e, y' = b·x + d·y + f.
pub(super) type Matrix = [f64; 6];
pub(super) const IDENTITY: Matrix = [1., 0., 0., 1., 0., 0.];
pub(super) fn multiply(m: Matrix, n: Matrix) -> Matrix {
    // m ∘ n : applique n puis m.
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}
pub(super) fn apply(m: Matrix, (x, y): (f64, f64)) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}
pub(super) fn transform(text: &str) -> std::result::Result<Matrix, String> {
    let mut m = IDENTITY;
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest
            .find('(')
            .ok_or_else(|| format!("Invalid transform \"{text}\""))?;
        let close = rest[open..]
            .find(')')
            .map(|i| i + open)
            .ok_or_else(|| format!("Unclosed transform function in \"{text}\""))?;
        let name = rest[..open].trim();
        let args = numbers(&rest[open + 1..close])?;
        let count = |n: &[usize]| {
            if n.contains(&args.len()) {
                Ok(())
            } else {
                Err(format!(
                    "{name}() takes {} arguments in \"{text}\"",
                    n.iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(" or ")
                ))
            }
        };
        let next = match name {
            "matrix" => {
                count(&[6])?;
                [args[0], args[1], args[2], args[3], args[4], args[5]]
            }
            "translate" => {
                count(&[1, 2])?;
                [1., 0., 0., 1., args[0], *args.get(1).unwrap_or(&0.)]
            }
            "scale" => {
                count(&[1, 2])?;
                [args[0], 0., 0., *args.get(1).unwrap_or(&args[0]), 0., 0.]
            }
            "rotate" => {
                count(&[1, 3])?;
                let (s, c) = args[0].to_radians().sin_cos();
                let r = [c, s, -s, c, 0., 0.];
                if args.len() == 3 {
                    let (x, y) = (args[1], args[2]);
                    multiply(
                        multiply([1., 0., 0., 1., x, y], r),
                        [1., 0., 0., 1., -x, -y],
                    )
                } else {
                    r
                }
            }
            "skewX" => {
                count(&[1])?;
                [1., 0., args[0].to_radians().tan(), 1., 0., 0.]
            }
            "skewY" => {
                count(&[1])?;
                [1., args[0].to_radians().tan(), 0., 1., 0., 0.]
            }
            _ => return Err(format!("Unknown SVG transform function '{name}'")),
        };
        m = multiply(m, next);
        rest = rest[close + 1..].trim_start_matches([' ', ',', '\t', '\n', '\r']);
    }
    if m.iter().all(|n| n.is_finite()) {
        Ok(m)
    } else {
        Err(format!("Transform \"{text}\" is not finite"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transforms_compose_in_svg_order() {
        let m = transform("translate(10 20) scale(2)").unwrap();
        assert_eq!(apply(m, (1., 1.)), (12., 22.));
        let r = transform("rotate(90 12 12)").unwrap();
        let (x, y) = apply(r, (12., 0.));
        assert!((x - 24.).abs() < 1e-9 && (y - 12.).abs() < 1e-9);
        assert!(transform("perspective(1)").is_err());
    }
}
