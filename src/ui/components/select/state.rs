use super::*;
impl Selects {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn captured(&self) -> bool {
        self.captured || self.dismiss_capture
    }

    pub fn expanded(&self, id: ElementId) -> bool {
        self.open.as_ref().is_some_and(|o| o.id == id)
    }

    pub fn active(&self, id: ElementId) -> Option<u64> {
        self.open
            .as_ref()
            .filter(|o| o.id == id)
            .and_then(|o| o.active)
    }

    pub fn cursor(&self) -> Option<Cursor> {
        self.open
            .as_ref()
            .map(|o| {
                if o.hover.is_some() {
                    Cursor::Hand
                } else {
                    Cursor::Arrow
                }
            })
            .or(self.dismiss_capture.then_some(Cursor::Arrow))
    }

    pub fn close(&mut self) -> SelectResponse {
        let response = SelectResponse {
            handled: true,
            repaint: self.open.is_some() || self.captured(),
            release: self.captured(),
            ..SelectResponse::default()
        };
        self.open = None;
        self.captured = false;
        self.dismiss_capture = false;
        response
    }

    pub fn sync(
        &mut self,
        root: &Element,
        focused: Option<ElementId>,
        viewport: RectF,
    ) -> SelectResponse {
        let valid = self.open.as_ref().is_none_or(|o| {
            control(root, o.id).is_some_and(|(s, r, clip)| {
                s.enabled
                    && focused == Some(o.id)
                    && same_rect(r, o.anchor)
                    && same_rect(clip, o.clip)
                    && same_rect(viewport, o.viewport)
                    && s.selected == o.selected
                    && s.options == o.options
                    && s.visible_items == o.visible_items
                    && (s.row_height() - o.row_height).abs() < 0.01
            })
        });
        if valid {
            SelectResponse::default()
        } else {
            self.close()
        }
    }

    pub fn expand(&mut self, id: ElementId, root: &Element, viewport: RectF) -> SelectResponse {
        if self.expanded(id) {
            return SelectResponse {
                handled: true,
                ..SelectResponse::default()
            };
        }
        let Some((s, anchor, clip)) = control(root, id) else {
            return SelectResponse::default();
        };
        if !s.enabled
            || layout::width(layout::intersect(anchor, clip)) <= 0.
            || layout::height(layout::intersect(anchor, clip)) <= 0.
        {
            return SelectResponse::default();
        }
        let layer = AnchoredLayer::place(
            anchor,
            SizeF {
                width: layout::width(anchor).max(180.),
                height: s.options.len().max(1).min(s.visible_items) as f32 * s.row_height() + 8.,
            },
            viewport,
        );
        if layout::height(layer.bounds) < s.row_height() + 8. || layout::width(layer.bounds) < 40. {
            return SelectResponse {
                handled: true,
                ..SelectResponse::default()
            };
        }
        let release = self.captured();
        self.captured = false;
        self.dismiss_capture = false;
        let active = s
            .selected
            .filter(|id| s.options.iter().any(|o| o.id == *id && o.enabled))
            .or_else(|| s.options.iter().find(|o| o.enabled).map(|o| o.id));
        let mut open = OpenSelect {
            row_height: s.row_height(),
            id,
            options: Arc::clone(&s.options),
            selected: s.selected,
            anchor,
            clip,
            viewport,
            layer,
            active,
            hover: None,
            pressed: None,
            first: 0,
            drag_scroll: None,
            query: String::new(),
            typed: None,
            visible_items: s.visible_items,
        };
        open.reveal();
        self.open = Some(open);
        SelectResponse {
            handled: true,
            repaint: true,
            release,
            focus: Some(id),
            ..SelectResponse::default()
        }
    }

    pub fn toggle(&mut self, id: ElementId, root: &Element, viewport: RectF) -> SelectResponse {
        if self.expanded(id) {
            self.close()
        } else {
            self.expand(id, root, viewport)
        }
    }

    pub fn highlight(&mut self, id: ElementId, key: u64) -> Option<SelectResponse> {
        let o = self.open.as_mut().filter(|o| o.id == id)?;
        if !o
            .options
            .iter()
            .any(|option| option.id == key && option.enabled)
        {
            return None;
        }
        o.active = Some(key);
        o.reveal();
        Some(SelectResponse {
            handled: true,
            repaint: true,
            focus: Some(id),
            ..SelectResponse::default()
        })
    }

    pub fn choose(&mut self, id: ElementId, key: u64, root: &Element) -> Option<SelectResponse> {
        let (s, _, _) = control(root, id)?;
        if !s.enabled || !s.options.iter().any(|o| o.id == key && o.enabled) {
            return None;
        }
        let mut response = if self.expanded(id) {
            self.close()
        } else {
            SelectResponse {
                handled: true,
                repaint: true,
                ..SelectResponse::default()
            }
        };
        if s.selected != Some(key) {
            response.change = Some(s.on_change.emit(key));
        }
        Some(response)
    }

    pub fn snapshot(&self, id: ElementId, s: &Select) -> SelectInfo {
        let open = self.open.as_ref().filter(|o| o.id == id);
        let zero = RectF::new(0., 0., 0., 0.);
        let options = s
            .options
            .iter()
            .enumerate()
            .filter(|(_, o)| open.is_some() || Some(o.id) == s.selected)
            .map(|(index, o)| {
                let bounds = open.map_or(zero, |p| p.row(index));
                SelectOptionInfo {
                    id: o.id,
                    label: Arc::clone(&o.label),
                    enabled: s.enabled && o.enabled,
                    selected: Some(o.id) == s.selected,
                    focused: open.is_some_and(|p| p.active == Some(o.id)),
                    bounds,
                    clip: open.map_or(zero, |p| layout::intersect(bounds, p.inner())),
                }
            })
            .collect();
        SelectInfo {
            expanded: open.is_some(),
            popup: open.map(|o| o.layer.bounds),
            options,
        }
    }
}
