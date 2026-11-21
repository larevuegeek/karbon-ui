//! Le trou d'une surface partagée (`ui::shared_surface`) : là où un élément peint le
//! gabarit d'une surface que la fenêtre montre sous son interface, l'image de
//! l'interface est rendue transparente, et la surface se voit au travers.
use super::super::present::surface_visual;
use super::*;
use crate::ui::shared_surface::SurfacePlacement;

impl Canvas {
    /// Si `image` est le gabarit d'une surface partagée que la fenêtre en cours de
    /// dessin peut montrer : lui donne la place de `rect` et troue l'image à cet endroit.
    /// Faux pour toute autre image, et pour un dessin hors écran : l'image se peint.
    pub(super) fn punch_shared_surface(&self, rect: RectF, image: &Arc<Image>) -> bool {
        let Some(surface) = crate::ui::shared_surface::surface_of(image) else {
            return false;
        };
        let target = &self.target;
        let mut transform = matrix(Affine::IDENTITY);
        let (mut dpi_x, mut dpi_y) = (96f32, 96f32);
        // SAFETY : contexte Direct2D valide ; sorties locales écrites pendant l'appel.
        unsafe {
            target.GetTransform(&mut transform);
            target.GetDpi(&mut dpi_x, &mut dpi_y);
        }
        let placement = pixels(rect, affine(transform), (dpi_x / 96., dpi_y / 96.));
        if !surface_visual::place(&surface, placement) {
            return false;
        }
        // Découpe sans lissage : chaque pixel est à l'interface ou à la surface, et ses
        // bords tombent sur ceux de `placement`.
        // SAFETY : passe de dessin ouverte ; la découpe est retirée aussitôt.
        unsafe {
            target.PushAxisAlignedClip(&rect_f(rect), D2D1_ANTIALIAS_MODE_ALIASED);
            target.Clear(Some(&D2D1_COLOR_F {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 0.,
            }));
            target.PopAxisAlignedClip();
        }
        true
    }
}

/// Le rectangle de pixels que couvre `rect` (unités de mise en page) sous la
/// transformation `transform`, à `scale` pixels par unité : la boîte de ses quatre coins.
fn pixels(rect: RectF, transform: Affine, scale: (f32, f32)) -> SurfacePlacement {
    let corner = |x: f32, y: f32| {
        (
            (x * transform.m11 + y * transform.m21 + transform.dx) * scale.0,
            (x * transform.m12 + y * transform.m22 + transform.dy) * scale.1,
        )
    };
    let corners = [
        corner(rect.left, rect.top),
        corner(rect.right, rect.top),
        corner(rect.left, rect.bottom),
        corner(rect.right, rect.bottom),
    ];
    let bound = |pick: fn((f32, f32)) -> f32, least: bool| {
        let values = corners.iter().map(|&c| pick(c));
        let value = match least {
            true => values.fold(f32::INFINITY, f32::min),
            false => values.fold(f32::NEG_INFINITY, f32::max),
        };
        value.round() as i32
    };
    let (left, top) = (bound(|c| c.0, true), bound(|c| c.1, true));
    let (right, bottom) = (bound(|c| c.0, false), bound(|c| c.1, false));
    SurfacePlacement {
        x: left,
        y: top,
        width: (right - left).max(0) as u32,
        height: (bottom - top).max(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_element_covers_whole_pixels_at_the_scale_of_the_screen() {
        let rect = RectF::new(10., 20., 100.5, 50.);
        let placed = pixels(rect, Affine::IDENTITY, (1.5, 1.5));
        assert_eq!(
            placed,
            SurfacePlacement {
                x: 15,
                y: 30,
                width: 151,
                height: 75
            }
        );
        // Un décalage (défilement) déplace le rectangle sans changer sa taille.
        let moved = Affine {
            dx: 4.,
            dy: -20.,
            ..Affine::IDENTITY
        };
        let placed = pixels(rect, moved, (1., 1.));
        assert_eq!((placed.x, placed.y), (14, 0));
        assert_eq!((placed.width, placed.height), (101, 50));
    }
}
