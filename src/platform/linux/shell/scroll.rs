//! Défilement commun aux boucles Wayland et X11 : un cran de molette vaut 3 lignes de
//! 16 px (48 px) en vertical et 3 colonnes de 32 px (96 px) en horizontal, comme sous
//! Windows ; Maj fait défiler horizontalement.

/// Déplacement d'un cran de molette : vertical, horizontal.
pub(in crate::platform::linux) const NOTCH: [f32; 2] = [48., 96.];

/// Déplacement `(x, y)` à envoyer au moteur, ou `None` s'il est nul. Avec Maj, un
/// défilement vertical seul devient horizontal, à l'échelle de son cran.
pub(in crate::platform::linux) fn delta(x: f32, y: f32, shift: bool) -> Option<(f32, f32)> {
    if x == 0. && y == 0. {
        return None;
    }
    Some(if shift && x == 0. {
        (y * NOTCH[1] / NOTCH[0], 0.)
    } else {
        (x, y)
    })
}

#[cfg(test)]
mod tests {
    use super::{NOTCH, delta};

    #[test]
    fn shift_turns_a_vertical_scroll_into_a_horizontal_one() {
        assert_eq!(delta(0., 0., true), None);
        assert_eq!(delta(0., NOTCH[0], false), Some((0., NOTCH[0])));
        assert_eq!(delta(0., NOTCH[0], true), Some((NOTCH[1], 0.)));
        // Un défilement déjà horizontal n'est pas touché.
        assert_eq!(delta(10., 5., true), Some((10., 5.)));
    }
}
