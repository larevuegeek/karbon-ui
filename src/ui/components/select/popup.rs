use super::*;
impl OpenSelect {
    pub(super) fn inner(&self) -> RectF {
        let r = self.layer.bounds;
        RectF::new(
            r.left + 4.,
            r.top + 4.,
            (layout::width(r)
                - 8.
                - if self.options.len() > self.rows() {
                    12.
                } else {
                    0.
                })
            .max(0.),
            (layout::height(r) - 8.).max(0.),
        )
    }

    pub(super) fn rows(&self) -> usize {
        ((layout::height(self.layer.bounds) - 8.).max(0.) / self.row_height)
            .floor()
            .max(1.) as usize
    }

    pub(super) fn max_first(&self) -> usize {
        self.options.len().saturating_sub(self.rows())
    }

    pub(super) fn row(&self, index: usize) -> RectF {
        let r = self.inner();
        RectF::new(
            r.left,
            r.top + (index as f32 - self.first as f32) * self.row_height,
            layout::width(r),
            self.row_height,
        )
    }

    pub(super) fn item(&self, p: PointF) -> Option<usize> {
        let r = self.inner();
        if !layout::contains(r, p.x, p.y) {
            return None;
        }
        let index = self.first + ((p.y - r.top) / self.row_height).floor() as usize;
        (index < self.options.len()).then_some(index)
    }

    pub(super) fn reveal(&mut self) {
        if let Some(index) = self
            .active
            .and_then(|id| self.options.iter().position(|o| o.id == id))
        {
            if index < self.first {
                self.first = index;
            } else if index >= self.first + self.rows() {
                self.first = index + 1 - self.rows();
            }
        }
        self.first = self.first.min(self.max_first());
    }

    pub(super) fn scroll_bar(&self) -> Option<(RectF, RectF)> {
        if self.max_first() == 0 {
            return None;
        }
        let r = self.layer.bounds;
        let track = RectF::new(
            r.right - 10.,
            r.top + 4.,
            6.,
            (layout::height(r) - 8.).max(0.),
        );
        let height = (layout::height(track) * self.rows() as f32 / self.options.len() as f32)
            .max(18.)
            .min(layout::height(track));
        let top = track.top
            + (layout::height(track) - height) * self.first as f32 / self.max_first() as f32;
        Some((
            track,
            RectF::new(track.left, top, track.right - track.left, height),
        ))
    }
}
