//! `backdrop-filter` : flou en boîte des pixels déjà peints sous une forme.
use super::{
    Canvas,
    shape::{Rounded, intersect},
};
use crate::ui::scene::PointF;

impl Canvas<'_> {
    /// `backdrop-filter` : flou en boîte (trois passes séparables) des pixels déjà
    /// dessinés sous la forme ; les autres filtres sont ignorés, comme `filter`.
    pub(super) fn backdrop(
        &mut self,
        shape: Rounded,
        filters: &crate::ui::rendering::filter::Filters,
    ) {
        let sigma = filters
            .values
            .iter()
            .map(|f| match f {
                crate::ui::rendering::filter::Filter::Blur(n) => *n,
                _ => 0.,
            })
            .sum::<f32>()
            * filters.scale
            * self.scale;
        let radius = sigma.round().clamp(0., 64.) as i64;
        if radius == 0 {
            return;
        }
        let area = intersect(self.device().bounds(shape.rect), self.visible());
        if !(area.left < area.right && area.top < area.bottom) {
            return;
        }
        let (w, h) = (self.width as i64, self.height as i64);
        let x0 = (area.left.floor() as i64 - radius).max(0);
        let x1 = (area.right.ceil() as i64 + radius).min(w);
        let y0 = (area.top.floor() as i64 - radius).max(0);
        let y1 = (area.bottom.ceil() as i64 + radius).min(h);
        let (bw, bh) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut buffer = vec![[0f32; 4]; bw * bh];
        for y in 0..bh {
            for x in 0..bw {
                let offset = ((y0 as usize + y) * self.width as usize + x0 as usize + x) * 4;
                for (c, value) in buffer[y * bw + x].iter_mut().enumerate() {
                    *value = f32::from(self.pixels[offset + c]);
                }
            }
        }
        // Flou en boîte horizontal puis vertical, répété trois fois (≈ gaussienne).
        let pass = |buffer: &mut Vec<[f32; 4]>, horizontal: bool| {
            let (n, m) = if horizontal { (bh, bw) } else { (bw, bh) };
            let index = |i: usize, j: usize| if horizontal { i * bw + j } else { j * bw + i };
            for i in 0..n {
                let line = (0..m).map(|j| buffer[index(i, j)]).collect::<Vec<_>>();
                for j in 0..m {
                    let (a, b) = (
                        (j as i64 - radius).max(0) as usize,
                        ((j as i64 + radius) as usize).min(m - 1),
                    );
                    let mut sum = [0f32; 4];
                    for v in &line[a..=b] {
                        for c in 0..4 {
                            sum[c] += v[c];
                        }
                    }
                    let count = (b - a + 1) as f32;
                    buffer[index(i, j)] = sum.map(|s| s / count);
                }
            }
        };
        for _ in 0..3 {
            pass(&mut buffer, true);
            pass(&mut buffer, false);
        }
        let device = self.device();
        let Some(inverse) = device.inverse() else {
            return;
        };
        for y in area.top.floor().max(0.) as i64..(area.bottom.ceil() as i64).min(h) {
            for x in area.left.floor().max(0.) as i64..(area.right.ceil() as i64).min(w) {
                let center = PointF {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                };
                if !shape.contains(inverse.point(center))
                    || !self
                        .clips
                        .iter()
                        .all(|clip| clip.shape.contains(clip.inverse.point(center)))
                {
                    continue;
                }
                let value = buffer[(y - y0) as usize * bw + (x - x0) as usize];
                let offset = (y as usize * self.width as usize + x as usize) * 4;
                for (target, v) in self.pixels[offset..offset + 4].iter_mut().zip(value) {
                    *target = v.round().clamp(0., 255.) as u8;
                }
            }
        }
    }
}
