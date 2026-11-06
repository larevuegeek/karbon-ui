//! Géométries de chemin Direct2D : polylignes (`Path`) et icônes vectorielles (`Vector`).
use super::*;

/// Crée une géométrie de chemin et ouvre son puits ; libellés d'erreur fournis par l'appelant.
pub(super) fn open_path(
    factory: &ID2D1Factory,
    create: &'static str,
    open: &'static str,
) -> crate::Result<(ID2D1PathGeometry, ID2D1GeometrySink)> {
    // SAFETY: fabrique Direct2D valide, utilisée sur son thread de création.
    let geometry = unsafe { factory.CreatePathGeometry() }.check(create)?;
    // SAFETY: géométrie fraîchement créée, ouverte une seule fois.
    let sink = unsafe { geometry.Open() }.check(open)?;
    Ok((geometry, sink))
}

pub(super) fn create_path(
    factory: &ID2D1Factory,
    points: &[PointF],
    closed: bool,
) -> crate::Result<ID2D1PathGeometry> {
    let (geometry, sink) = open_path(factory, "CreatePathGeometry", "PathGeometry Open")?;
    let lines = points[1..].iter().copied().map(vector).collect::<Vec<_>>();
    // SAFETY: puits ouvert et non encore fermé ; points copiés pendant chaque appel.
    unsafe {
        sink.BeginFigure(
            vector(points[0]),
            if closed {
                D2D1_FIGURE_BEGIN_FILLED
            } else {
                D2D1_FIGURE_BEGIN_HOLLOW
            },
        );
        sink.AddLines(&lines);
        sink.EndFigure(if closed {
            D2D1_FIGURE_END_CLOSED
        } else {
            D2D1_FIGURE_END_OPEN
        });
    }
    // SAFETY: ferme le puits une seule fois.
    unsafe { sink.Close() }.check("GeometrySink Close")?;
    Ok(geometry)
}

impl Canvas {
    /// Icône vectorielle : contours mis à l'échelle de `rect`, remplis puis tracés si
    /// `outline` est positif.
    pub(super) fn icon(
        &self,
        factory: &ID2D1Factory,
        rect: RectF,
        icon: &crate::ui::vector::VectorIcon,
        color: Color,
        outline: f32,
    ) -> crate::Result<()> {
        use crate::ui::vector::VectorSegment;
        let t = &self.target;
        let b = &self.brush;
        self.color(color);
        let (geometry, sink) = open_path(
            factory,
            "CreatePathGeometry (icon)",
            "PathGeometry Open (icon)",
        )?;
        if !icon.width.is_finite()
            || icon.width <= 0.
            || !icon.height.is_finite()
            || icon.height <= 0.
        {
            return Err(crate::Error::invalid_input("invalid icon dimensions"));
        }
        let transform = |p: PointF| Vector2 {
            X: rect.left + p.x / icon.width * (rect.right - rect.left),
            Y: rect.top + p.y / icon.height * (rect.bottom - rect.top),
        };
        for contour in &icon.contours {
            if contour.segments.len() < 2 {
                continue;
            }
            // SAFETY: puits ouvert et non encore fermé ; segments copiés pendant chaque appel.
            unsafe {
                sink.BeginFigure(transform(contour.start), D2D1_FIGURE_BEGIN_FILLED);
                for segment in &contour.segments {
                    match segment {
                        VectorSegment::Line(end) => sink.AddLines(&[transform(*end)]),
                        VectorSegment::Curve {
                            control1,
                            control2,
                            end,
                        } => sink.AddBeziers(&[D2D1_BEZIER_SEGMENT {
                            point1: transform(*control1),
                            point2: transform(*control2),
                            point3: transform(*end),
                        }]),
                    }
                }
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            }
        }
        // SAFETY: ferme le puits une seule fois.
        unsafe { sink.Close() }.check("GeometrySink Close (icon)")?;
        // SAFETY: passe de dessin ouverte ; géométrie fermée et pinceau vivants.
        unsafe {
            t.FillGeometry(&geometry, b, None);
            if outline > 0. {
                t.DrawGeometry(
                    &geometry,
                    b,
                    outline * (rect.right - rect.left) / icon.width,
                    None,
                );
            }
        }
        Ok(())
    }
}
