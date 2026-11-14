//! Teinte dominante d'une image, pour teinter un fond d'après une pochette ou une photo.
use crate::ui::{scene::Image, theme::Color};

impl Image {
    /// Dominant hue of the image and how strong it is, as `(hue, strength)`: the OKLCH
    /// hue in degrees (`0..360`) and `0.0` (grey) to `1.0` (vivid). Read on a 32 × 32
    /// grid; vivid pixels weigh more, near-black and near-white ones are ignored.
    ///
    /// Feed it to a stylesheet tinted by custom properties
    /// (`style:--h={hue} style:--s={strength}`).
    ///
    /// ```
    /// use karbon_ui::ui::scene::Image;
    ///
    /// // Rouge vif (BGRA prémultiplié).
    /// let red = Image::new(2, 2, [0, 0, 220, 255].repeat(4)).unwrap();
    /// let (hue, strength) = red.dominant_hue();
    /// assert!((25. ..35.).contains(&hue) && strength > 0.9);
    /// ```
    pub fn dominant_hue(&self) -> (f32, f32) {
        const GRID: u32 = 32;
        if self.width == 0 || self.height == 0 {
            return (0., 0.);
        }
        let (mut sa, mut sb, mut weight) = (0f32, 0f32, 0f32);
        for y in 0..GRID {
            for x in 0..GRID {
                let px = (x * self.width / GRID).min(self.width - 1);
                let py = (y * self.height / GRID).min(self.height - 1);
                let i = ((py * self.width + px) * 4) as usize;
                let Some(p) = self.pixels.get(i..i + 3) else {
                    continue;
                };
                let [l, a, b] = Color::rgb(p[2], p[1], p[0]).to_oklab();
                // Noirs et blancs n'ont pas de teinte fiable.
                if !(0.18..=0.96).contains(&l) {
                    continue;
                }
                let w = a * a + b * b;
                sa += a * w;
                sb += b * w;
                weight += w;
            }
        }
        if weight <= 0. {
            return (0., 0.);
        }
        let hue = (sb.atan2(sa).to_degrees() + 360.) % 360.;
        let chroma = sa.hypot(sb) / weight;
        let coverage = ((weight / (GRID * GRID) as f32) / 0.004).min(1.);
        (hue.round() % 360., ((chroma / 0.1) * coverage).min(1.))
    }
}
