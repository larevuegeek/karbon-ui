//! Images d'icône pour le bus et pour X11, tirées d'une image encodée : décodée par
//! gdk-pixbuf (BGRA prémultiplié), réduite aux tailles usuelles, puis convertie en ARGB à
//! alpha droit (`IconPixmap` de StatusNotifierItem : octets A, R, G, B ; `_NET_WM_ICON` :
//! mots `0xAARRGGBB` de 32 bits).
use crate::platform::linux::imaging;
use crate::ui::scene::Image;
use std::sync::Arc;

/// Tailles proposées à l'hôte de la zone de notification, qui choisit la plus proche.
const TRAY_SIZES: [u32; 6] = [16, 22, 24, 32, 48, 64];
/// Tailles de `_NET_WM_ICON` (barre des tâches, Alt+Tab, titre).
const WINDOW_SIZES: [u32; 5] = [16, 32, 48, 64, 128];

/// Une image de l'icône : taille et pixels ARGB, octets dans l'ordre du réseau.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pixmap {
    pub width: i32,
    pub height: i32,
    pub argb: Vec<u8>,
}

/// L'image aux tailles de la zone de notification (jamais agrandie au-delà de la source,
/// sauf si elle est plus petite que toutes).
pub(super) fn tray_pixmaps(bytes: &[u8]) -> crate::Result<Vec<Pixmap>> {
    Ok(scaled(bytes, &TRAY_SIZES)?
        .iter()
        .map(|image| Pixmap {
            width: image.width as i32,
            height: image.height as i32,
            argb: straight_pixels(image).flatten().collect(),
        })
        .collect())
}

/// Valeur de `_NET_WM_ICON` : pour chaque taille, largeur, hauteur puis les pixels
/// `0xAARRGGBB`.
pub(in crate::platform::linux) fn window_icon(bytes: &[u8]) -> crate::Result<Vec<u32>> {
    let mut words = Vec::new();
    for image in scaled(bytes, &WINDOW_SIZES)? {
        words.extend([image.width, image.height]);
        words.extend(straight_pixels(&image).map(|[a, r, g, b]| u32::from_be_bytes([a, r, g, b])));
    }
    Ok(words)
}

/// L'image décodée, réduite à chaque taille de `sizes` sur son grand côté.
fn scaled(bytes: &[u8], sizes: &[u32]) -> crate::Result<Vec<Image>> {
    let source = Arc::new(imaging::decode(bytes)?);
    let largest = source.width.max(source.height);
    let mut wanted: Vec<u32> = sizes.iter().copied().filter(|&s| s <= largest).collect();
    if wanted.is_empty() {
        wanted.push(sizes[0]);
    }
    wanted
        .into_iter()
        .map(|edge| {
            let ratio = f64::from(edge) / f64::from(largest);
            let width = (f64::from(source.width) * ratio).round().max(1.) as usize;
            let height = (f64::from(source.height) * ratio).round().max(1.) as usize;
            imaging::resize(&source, width, height)
        })
        .collect()
}

/// Pixels `[A, R, G, B]` à alpha droit d'une image BGRA prémultipliée.
pub(super) fn straight_pixels(image: &Image) -> impl Iterator<Item = [u8; 4]> + '_ {
    image.pixels.as_chunks::<4>().0.iter().map(|pixel| {
        let alpha = pixel[3];
        let straight = |channel: u8| {
            if alpha == 0 {
                0
            } else {
                ((u32::from(channel) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255)
                    as u8
            }
        };
        [
            alpha,
            straight(pixel[2]),
            straight(pixel[1]),
            straight(pixel[0]),
        ]
    })
}
