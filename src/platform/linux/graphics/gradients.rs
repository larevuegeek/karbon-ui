//! Dégradés gardés d'une image à l'autre : pixman calcule un dégradé pixel par pixel à
//! chaque remplissage (le premier poste du rendu Cairo d'une page à cartes en dégradé).
//! Rastérisé une fois à sa taille d'affichage, il est ensuite recopié : la source du
//! remplissage est cette image, posée sur les pixels de la cible par une translation
//! entière. Un défilement le retrouve tel quel, la clé ne portant pas la position.
use super::{
    super::{
        bitmap::Bitmap,
        ffi::cairo::{Matrix, OPERATOR_SOURCE},
    },
    canvas::Canvas,
    paint::{linear, radial},
};
use crate::ui::{
    rendering::{brush::Background, transform::Affine},
    scene::RectF,
};
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    rc::Rc,
};

/// Plus petit dégradé gardé (pixels) : en dessous, le calculer coûte moins que le garder.
const MIN_PIXELS: usize = 4096;
/// Plus grand dégradé gardé (pixels, 16 Mio en ARGB).
const MAX_PIXELS: usize = 4096 * 1024;

/// Ce qui détermine les pixels d'un dégradé rastérisé, position sur la cible exceptée :
/// taille en pixels, partie fractionnaire de l'origine, taille logique du rectangle,
/// échelle et opacité (bits des `f32`).
type Key = [u32; 9];

struct Entry {
    fill: Background,
    key: Key,
    image: Rc<Bitmap>,
}

/// Dégradés rastérisés ; chaque peinture ne garde que ceux qu'elle a utilisés.
#[derive(Default)]
pub(super) struct Gradients {
    entries: HashMap<u64, Entry>,
    used: HashSet<u64>,
}
impl Gradients {
    pub fn begin(&mut self) {
        self.used.clear();
    }
    pub fn end(&mut self) {
        self.entries.retain(|id, _| self.used.contains(id));
    }
    /// Nombre de dégradés gardés.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Image d'un dégradé gardé.
    #[cfg(test)]
    pub fn first(&self) -> Rc<Bitmap> {
        Rc::clone(
            &self
                .entries
                .values()
                .next()
                .expect("un dégradé gardé")
                .image,
        )
    }
    /// Image du dégradé `fill` pour `key`, rastérisée par `draw` si elle manque.
    fn get(
        &mut self,
        fill: &Background,
        key: Key,
        draw: impl FnOnce() -> crate::Result<Bitmap>,
    ) -> crate::Result<Rc<Bitmap>> {
        let id = digest(fill, &key);
        self.used.insert(id);
        if let Some(entry) = self
            .entries
            .get(&id)
            .filter(|e| e.key == key && e.fill == *fill)
        {
            return Ok(Rc::clone(&entry.image));
        }
        let image = Rc::new(draw()?);
        self.entries.insert(
            id,
            Entry {
                fill: fill.clone(),
                key,
                image: Rc::clone(&image),
            },
        );
        Ok(image)
    }
}

/// Empreinte du contenu d'un dégradé et de sa clé (l'égalité est vérifiée ensuite).
fn digest(fill: &Background, key: &Key) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    let stops = |h: &mut std::collections::hash_map::DefaultHasher,
                 stops: &[crate::ui::rendering::brush::GradientStop]| {
        for s in stops {
            s.position.to_bits().hash(h);
            [s.color.red, s.color.green, s.color.blue, s.color.alpha].hash(h);
        }
    };
    match fill {
        Background::Solid(c) => (0u8, [c.red, c.green, c.blue, c.alpha]).hash(&mut h),
        Background::Linear(g) => {
            (1u8, g.angle.to_bits(), g.corner).hash(&mut h);
            stops(&mut h, &g.stops);
        }
        Background::Radial(g) => {
            (2u8, g.center.map(f32::to_bits), g.radius.map(f32::to_bits)).hash(&mut h);
            (std::mem::discriminant(&g.extent), g.circle).hash(&mut h);
            stops(&mut h, &g.stops);
        }
    }
    h.finish()
}

impl Canvas<'_> {
    /// Image d'un dégradé `fill` posé sur `rect`, rastérisée une fois à sa taille
    /// d'affichage, et la matrice du motif qui la pose sur la cible ; `None` s'il est
    /// calculé directement (repère tourné ou en miroir, dégradé très petit ou très grand,
    /// couleur unie).
    pub(super) fn cached_gradient(
        &mut self,
        fill: &Background,
        rect: RectF,
    ) -> crate::Result<Option<(Rc<Bitmap>, Matrix)>> {
        let t = self.transform;
        if matches!(fill, Background::Solid(_))
            || t.m12 != 0.
            || t.m21 != 0.
            || t.m11 <= 0.
            || t.m22 <= 0.
        {
            return Ok(None);
        }
        let device = t.bounds(rect);
        let (x0, y0) = (device.left.floor(), device.top.floor());
        let (x1, y1) = (device.right.ceil(), device.bottom.ceil());
        if !(x0.is_finite() && y0.is_finite() && x1 > x0 && y1 > y0) {
            return Ok(None);
        }
        let (width, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
        if !(MIN_PIXELS..=MAX_PIXELS).contains(&(width * height)) {
            return Ok(None);
        }
        let key = [
            width as u32,
            height as u32,
            (device.left - x0).to_bits(),
            (device.top - y0).to_bits(),
            (rect.right - rect.left).to_bits(),
            (rect.bottom - rect.top).to_bits(),
            t.m11.to_bits(),
            t.m22.to_bits(),
            self.alpha.to_bits(),
        ];
        // Du repère logique vers les pixels de l'image du dégradé.
        let to_image = t.then(Affine::translation(-x0, -y0));
        let alpha = self.alpha;
        let image = self.caches.gradients.get(fill, key, || {
            let bitmap = Bitmap::new(width, height)?;
            {
                let c = bitmap.context()?;
                c.transform(Matrix::from_affine(to_image));
                let source = match fill {
                    Background::Linear(g) => linear(g, rect, alpha, false)?,
                    Background::Radial(g) => radial(g, rect, alpha)?,
                    Background::Solid(_) => unreachable!("couleur unie écartée plus haut"),
                };
                c.set_source(&source);
                c.set_operator(OPERATOR_SOURCE);
                c.paint();
            }
            Ok(bitmap)
        })?;
        Ok(Some((image, Matrix::from_affine(to_image))))
    }
}
