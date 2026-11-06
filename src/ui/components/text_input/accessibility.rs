use super::*;
impl TextInputs {
    pub fn snapshot(&self, id: ElementId, input: &TextInput) -> Option<InputSnapshot> {
        let e = self.states.get(&id)?;
        Some(InputSnapshot {
            value: e.text.clone(),
            selection: e.range(),
            composing: e.composition.is_some(),
            read_only: input.read_only,
            caret: e.caret(),
            bounds: e.clip,
        })
    }

    pub fn geometry(&self, id: ElementId, input: &TextInput) -> Option<InputGeometry> {
        let e = self.states.get(&id)?;
        Some(InputGeometry {
            composing: e.composition.is_some(),
            read_only: input.read_only,
            caret: e.caret(),
            bounds: e.clip,
        })
    }

    pub fn accessible_text(&self, id: ElementId) -> Option<AccessibleText> {
        let e = self.states.get(&id)?;
        if e.rows.is_some() {
            return Some(e.accessible_rows());
        }
        let line = e.line.as_ref()?;
        let dx = e.rect.left + e.text_offset - e.scroll;
        let top = e.rect.top + (layout::height(e.rect) - line.height).max(0.) / 2.;
        let cursor = e.composition.as_ref().map(|c| c.range.start + c.caret);
        Some(AccessibleText {
            value: Arc::clone(&e.display),
            selection: cursor.map_or_else(|| e.range(), |c| c..c),
            carets: line
                .carets
                .iter()
                .map(|c| TextCaret {
                    byte: c.byte,
                    x: c.x + dx,
                })
                .collect(),
            clusters: line
                .clusters
                .iter()
                .map(|c| TextCluster {
                    start: c.start,
                    end: c.end,
                    rect: RectF::new(
                        dx + c.rect.left,
                        top + c.rect.top,
                        layout::width(c.rect),
                        layout::height(c.rect),
                    ),
                })
                .collect(),
            clip: e.clip,
            multiline: false,
            lines: vec![AccessibleLine {
                range: 0..e.display.len(),
                rect: RectF::new(dx, top, line.width, line.height),
            }],
        })
    }
}
impl Editor {
    /// Texte accessible d'un champ multiligne : carets et clusters ligne par ligne.
    fn accessible_rows(&self) -> AccessibleText {
        let rows = self.rows.as_ref().unwrap();
        let align = self.font.map_or(Default::default(), |f| f.align);
        let mut carets = Vec::new();
        let mut clusters = Vec::new();
        let mut lines = Vec::with_capacity(rows.lines.len());
        for (index, row) in rows.lines.iter().enumerate() {
            let dx = self.rect.left + rows.offset(index, align);
            let top = self.rect.top + row.top - self.scroll_y;
            carets.extend(row.line.carets.iter().map(|c| TextCaret {
                byte: c.byte,
                x: c.x + dx,
            }));
            clusters.extend(row.line.clusters.iter().map(|c| TextCluster {
                start: c.start,
                end: c.end,
                rect: RectF::new(
                    dx + c.rect.left,
                    top + c.rect.top,
                    layout::width(c.rect),
                    layout::height(c.rect),
                ),
            }));
            lines.push(AccessibleLine {
                range: row.range.clone(),
                rect: RectF::new(dx, top, row.line.width, row.line.height),
            });
        }
        let cursor = self.composition.as_ref().map(|c| c.range.start + c.caret);
        AccessibleText {
            value: Arc::clone(&self.display),
            selection: cursor.map_or_else(|| self.range(), |c| c..c),
            carets,
            clusters,
            clip: self.clip,
            multiline: true,
            lines,
        }
    }
}
