use super::*;
impl Sliders {
    pub fn captured(&self) -> bool {
        self.drag.is_some()
    }

    pub fn value(&self, id: ElementId, base: f64) -> f64 {
        self.drag
            .or(self.keyboard)
            .filter(|e| e.id == id)
            .map_or(base, |e| e.value)
    }

    pub fn paint(&self, id: ElementId, s: &Slider, ctx: &PaintContext, scene: &mut Scene) {
        s.paint_value(
            self.value(id, s.value),
            self.drag.is_some_and(|e| e.id == id),
            ctx,
            scene,
        );
    }

    /// Retirer un contrôle ou changer sa configuration invalide son interaction.
    pub fn sync(&mut self, root: &Element, focused: Option<ElementId>) -> bool {
        let valid = |edit: Edit| {
            slider(root, edit.id)
                .is_some_and(|(s, _)| s.enabled && !s.read_only && s.config() == edit.config)
        };
        let release = self.drag.is_some_and(|e| !valid(e));
        if release {
            self.drag = None;
        }
        if self
            .keyboard
            .is_some_and(|e| !valid(e) || focused != Some(e.id))
        {
            self.keyboard = None;
        }
        release
    }

    pub fn begin(&mut self, id: ElementId, p: PointF, root: &Element) -> SliderResponse {
        let mut response = SliderResponse {
            handled: true,
            repaint: true,
            ..SliderResponse::default()
        };
        let Some((s, rect)) = slider(root, id) else {
            return response;
        };
        if !s.enabled || s.read_only || !p.x.is_finite() || !p.y.is_finite() {
            return response;
        }
        let original = self
            .keyboard
            .take()
            .filter(|e| e.id == id)
            .map_or(s.value, |e| e.value);
        let g = s.geometry(rect, original);
        if g.length <= 0. {
            return response;
        }
        let coordinate = s.coordinate(p);
        let cross = if s.axis == Axis::Horizontal {
            p.y - (rect.top + rect.bottom) / 2.
        } else {
            p.x - (rect.left + rect.right) / 2.
        };
        let thumb = (coordinate - g.center).abs() <= 8. && cross.abs() <= 8.;
        let offset = if thumb { coordinate - g.center } else { 0. };
        let value = if thumb {
            original
        } else {
            s.pointer_value(rect, p, offset).unwrap_or(s.value)
        };
        self.drag = Some(Edit {
            id,
            config: s.config(),
            original,
            value,
            offset,
            keys: 0,
            pointer: coordinate,
        });
        response.capture = true;
        if value != original {
            response.change = Some((id, value));
        }
        response
    }

    pub fn set_value(&mut self, id: ElementId, value: f64, root: &Element) -> SliderResponse {
        let Some((s, _)) = slider(root, id) else {
            return SliderResponse::default();
        };
        let edit = self.drag.or(self.keyboard).filter(|e| e.id == id);
        let release = self.drag.is_some_and(|e| e.id == id);
        if release {
            self.drag = None;
        }
        if self.keyboard.is_some_and(|e| e.id == id) {
            self.keyboard = None;
        }
        let value = s.snapped(value);
        let changed = value != s.value;
        let commit = if changed || edit.is_some_and(|e| e.original != value) {
            Some((id, value))
        } else {
            None
        };
        SliderResponse {
            handled: true,
            repaint: true,
            release,
            change: changed.then_some((id, value)),
            commit,
            ..SliderResponse::default()
        }
    }
}
