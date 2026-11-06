//! Dessins SVG compilés : une géométrie de chemin Direct2D par forme, en unités du
//! viewBox, conservée tant que la forme reste dans la scène ; styles de trait partagés.
use super::*;
use crate::ui::rendering::svg::{FillRule, LineCap, LineJoin, PathOp, SvgImage, SvgShape};
use std::collections::HashSet;

type StrokeKey = (LineCap, LineJoin, u32);
#[derive(Default)]
pub(super) struct Cache {
    /// Clé : adresse de la forme statique.
    paths: HashMap<usize, ID2D1PathGeometry>,
    strokes: HashMap<StrokeKey, ID2D1StrokeStyle>,
}
fn key(shape: &'static SvgShape) -> usize {
    shape as *const SvgShape as usize
}
fn stroke_key(shape: &SvgShape) -> StrokeKey {
    (shape.line_cap, shape.line_join, shape.miter_limit.to_bits())
}
impl Cache {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.paths.len()
    }
    pub fn prepare(&mut self, factory: &ID2D1Factory, scene: &Scene) -> crate::Result<()> {
        let mut used = HashSet::new();
        let mut strokes = HashSet::new();
        for command in &scene.commands {
            let DrawCommand::Svg { image, .. } = command else {
                continue;
            };
            for shape in image.shapes {
                let key = key(shape);
                used.insert(key);
                if let std::collections::hash_map::Entry::Vacant(entry) = self.paths.entry(key) {
                    entry.insert(create(factory, shape)?);
                }
                if shape.stroke != crate::ui::rendering::svg::SvgPaint::None {
                    let key = stroke_key(shape);
                    strokes.insert(key);
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        self.strokes.entry(key)
                    {
                        entry.insert(stroke_style(factory, shape)?);
                    }
                }
            }
        }
        self.paths.retain(|k, _| used.contains(k));
        self.strokes.retain(|k, _| strokes.contains(k));
        Ok(())
    }
    /// Peint `image` dans `rect` ; `color` donne `currentColor`.
    pub fn draw(
        &self,
        target: &ID2D1DeviceContext,
        brush: &ID2D1SolidColorBrush,
        rect: RectF,
        image: &'static SvgImage,
        color: Color,
    ) {
        let mut native = matrix(Affine::IDENTITY);
        // SAFETY: passe de dessin ouverte ; matrice lue pendant l'appel seulement.
        let previous = unsafe {
            target.GetTransform(&mut native);
            affine(native)
        };
        // SAFETY: passe de dessin ouverte ; matrice copiée pendant l'appel.
        unsafe { target.SetTransform(&matrix(image.placement(rect).then(previous))) };
        for shape in image.shapes {
            let Some(geometry) = self.paths.get(&key(shape)) else {
                continue;
            };
            if let Some(fill) = shape.fill.resolve(color, shape.fill_opacity) {
                // SAFETY: passe de dessin ouverte ; pinceau et géométrie du cache vivants.
                unsafe {
                    brush.SetColor(&color_f(fill));
                    target.FillGeometry(geometry, brush, None);
                }
            }
            if let Some(stroke) = shape.stroke.resolve(color, shape.stroke_opacity)
                && shape.stroke_width > 0.
            {
                let style = self.strokes.get(&stroke_key(shape));
                // SAFETY: passe de dessin ouverte ; pinceau, géométrie et style du cache vivants.
                unsafe {
                    brush.SetColor(&color_f(stroke));
                    target.DrawGeometry(geometry, brush, shape.stroke_width, style);
                }
            }
        }
        // SAFETY: rétablit la matrice lue ci-dessus.
        unsafe { target.SetTransform(&matrix(previous)) };
    }
}
fn stroke_style(factory: &ID2D1Factory, shape: &SvgShape) -> crate::Result<ID2D1StrokeStyle> {
    let cap = match shape.line_cap {
        LineCap::Butt => D2D1_CAP_STYLE_FLAT,
        LineCap::Round => D2D1_CAP_STYLE_ROUND,
        LineCap::Square => D2D1_CAP_STYLE_SQUARE,
    };
    let join = match shape.line_join {
        // SVG : au-delà de la limite, l'onglet devient un biseau.
        LineJoin::Miter => D2D1_LINE_JOIN_MITER_OR_BEVEL,
        LineJoin::Round => D2D1_LINE_JOIN_ROUND,
        LineJoin::Bevel => D2D1_LINE_JOIN_BEVEL,
    };
    let properties = D2D1_STROKE_STYLE_PROPERTIES {
        startCap: cap,
        endCap: cap,
        dashCap: cap,
        lineJoin: join,
        // Direct2D rapporte l'onglet à la demi-épaisseur, SVG à l'épaisseur.
        miterLimit: shape.miter_limit * 2.,
        dashStyle: D2D1_DASH_STYLE_SOLID,
        dashOffset: 0.,
    };
    // SAFETY: fabrique valide ; propriétés lues pendant l'appel, sans tirets.
    unsafe { factory.CreateStrokeStyle(&properties, None) }.check("CreateStrokeStyle (svg)")
}
fn create(factory: &ID2D1Factory, shape: &SvgShape) -> crate::Result<ID2D1PathGeometry> {
    let (geometry, sink) = open_path(factory, "CreatePathGeometry (svg)", "Open (svg)")?;
    let point = |x: f32, y: f32| Vector2 { X: x, Y: y };
    // SAFETY: puits ouvert et non encore fermé ; points copiés pendant chaque appel.
    unsafe {
        sink.SetFillMode(match shape.fill_rule {
            FillRule::NonZero => D2D1_FILL_MODE_WINDING,
            FillRule::EvenOdd => D2D1_FILL_MODE_ALTERNATE,
        });
        // Figure ouverte, et point de départ d'une figure à ouvrir au prochain segment
        // (après M, ou après Z : le tracé repart du début du sous-chemin).
        let mut open = false;
        let mut pending: Option<Vector2> = None;
        let mut start = point(0., 0.);
        for op in shape.ops {
            let segment = !matches!(op, PathOp::MoveTo(..) | PathOp::Close);
            if segment && let Some(at) = pending.take() {
                sink.BeginFigure(at, D2D1_FIGURE_BEGIN_FILLED);
                open = true;
            }
            match *op {
                PathOp::MoveTo(x, y) => {
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_OPEN);
                        open = false;
                    }
                    start = point(x, y);
                    pending = Some(start);
                }
                PathOp::LineTo(x, y) if open => sink.AddLine(point(x, y)),
                PathOp::CubicTo(x1, y1, x2, y2, x, y) if open => {
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: point(x1, y1),
                        point2: point(x2, y2),
                        point3: point(x, y),
                    })
                }
                PathOp::Close => {
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                        open = false;
                    }
                    pending = Some(start);
                }
                _ => {}
            }
        }
        if open {
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
        }
    }
    // SAFETY: ferme le puits une seule fois.
    unsafe { sink.Close() }.check("GeometrySink Close (svg)")?;
    Ok(geometry)
}
