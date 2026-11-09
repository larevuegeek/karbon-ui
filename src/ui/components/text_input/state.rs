use super::*;
impl TextInputs {
    pub(crate) fn value_empty(&self, id: ElementId) -> Option<bool> {
        self.states.get(&id).map(|e| e.text.is_empty())
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn reveal(&mut self, id: ElementId, byte: usize) -> bool {
        let Some(e) = self.states.get_mut(&id) else {
            return false;
        };
        let Some(line) = &e.line else {
            return false;
        };
        e.reveal = Some(line.snap(byte.min(e.display.len())));
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn select(&mut self, id: ElementId, range: Range<usize>, expected: &str) -> bool {
        let Some(e) = self.states.get_mut(&id) else {
            return false;
        };
        if e.composition.is_some() || e.display.as_ref() != expected {
            return false;
        }
        let Some(line) = &e.line else {
            return false;
        };
        e.reveal = None;
        e.follow = true;
        e.upstream = false;
        e.anchor = line.snap(range.start.min(e.text.len()));
        e.cursor = line.snap(range.end.min(e.text.len()));
        self.dragging = None;
        self.caret_visible = true;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn set_value(&mut self, id: ElementId, value: &str, max: usize) -> Option<String> {
        if self.dragging == Some(id) {
            self.dragging = None;
        }
        self.caret_visible = true;
        self.revision = self.revision.wrapping_add(1);
        let editor = self.states.get_mut(&id)?;
        editor.composition = None;
        editor.anchor = 0;
        editor.cursor = editor.text.len();
        if editor.replace(value, max) {
            Some(editor.text.clone())
        } else {
            None
        }
    }

    pub fn sync(
        &mut self,
        root: &Element,
        focused: Option<ElementId>,
        text: &mut dyn TextMeasurer,
        rebuilt: bool,
    ) -> crate::Result<()> {
        if self.focused != focused {
            if let Some(old) = self.focused.and_then(|id| self.states.get_mut(&id)) {
                old.composition = None;
            }
            self.focused = focused;
            self.caret_visible = true;
        }
        let mut live = HashSet::new();
        fn visit(
            node: &Element,
            states: &mut HashMap<ElementId, Editor>,
            live: &mut HashSet<ElementId>,
            text: &mut dyn TextMeasurer,
            rebuilt: bool,
        ) -> crate::Result<()> {
            if node.checked_subtree().is_some_and(|c| !c.text_input) {
                return Ok(());
            }
            if let Some(input) = node.widget.as_ref().and_then(|w| w.text_input()) {
                live.insert(node.id);
                let editor = states
                    .entry(node.id)
                    .or_insert_with(|| Editor::new(&input.value, input.max_chars, input.multiline));
                if editor.multiline != input.multiline {
                    *editor = Editor::new(&input.value, input.max_chars, input.multiline);
                }
                let value = editor::sanitize(&input.value, input.max_chars, input.multiline);
                if rebuilt && editor.text != value {
                    editor.text = value;
                    editor.cursor = editor.cursor.min(editor.text.len());
                    while !editor.text.is_char_boundary(editor.cursor) {
                        editor.cursor -= 1;
                    }
                    editor.anchor = editor.cursor;
                    editor.undo.clear();
                    editor.redo.clear();
                    editor.composition = None;
                    editor.reveal = None;
                }
                editor.rect = input.inner(layout::content_rect(node));
                editor.clip = layout::intersect(editor.rect, node.clip);
                if input.multiline {
                    editor.sync_rows(input, text)?;
                    for child in &node.children {
                        visit(child, states, live, text, rebuilt)?;
                    }
                    return Ok(());
                }
                let (value, cursor, _) = editor.displayed();
                // Champ masqué : une puce par caractère, positions ramenées au texte réel.
                let shown = if input.masked {
                    std::borrow::Cow::Owned(value.chars().map(|_| '\u{2022}').collect::<String>())
                } else {
                    value.clone()
                };
                let format = input.format();
                if editor.line.is_none()
                    || editor.display.as_ref() != shown.as_ref()
                    || editor.font != Some(format)
                {
                    let mut line = text.layout_formatted_line(&shown, format)?;
                    if input.masked {
                        unmask(&mut line, &value);
                    }
                    let display = shown.into_owned().into();
                    editor.line = Some(line);
                    editor.display = display;
                    editor.font = Some(format);
                    if editor.composition.is_none() {
                        let line = editor.line.as_ref().unwrap();
                        editor.cursor = line.snap(editor.cursor);
                        editor.anchor = line.snap(editor.anchor);
                    }
                }
                let line = editor.line.as_ref().unwrap();
                let extra = (layout::width(editor.rect) - 2. - line.width).max(0.);
                editor.text_offset = match format.align {
                    crate::ui::styling::TextAlign::End => extra,
                    crate::ui::styling::TextAlign::Center => extra / 2.,
                    _ => 0.,
                };
                let x = line.x(editor.reveal.unwrap_or(cursor));
                let available = (layout::width(editor.rect) - 2.).max(0.);
                if x < editor.scroll {
                    editor.scroll = x;
                } else if x > editor.scroll + available {
                    editor.scroll = x - available;
                }
                editor.scroll = editor.scroll.clamp(0., (line.width - available).max(0.));
            }
            for child in &node.children {
                visit(child, states, live, text, rebuilt)?;
            }
            Ok(())
        }
        visit(root, &mut self.states, &mut live, text, rebuilt)?;
        self.states.retain(|id, _| live.contains(id));
        if self.dragging.is_some_and(|id| !live.contains(&id)) {
            self.dragging = None;
        }
        Ok(())
    }
}
/// Ramène les positions d'une ligne de puces (3 octets chacune) aux octets de `value`.
pub(super) fn unmask(line: &mut crate::ui::text_input::TextLine, value: &str) {
    let mut bytes = value.char_indices().map(|(b, _)| b).collect::<Vec<_>>();
    bytes.push(value.len());
    let map = |b: usize| bytes[(b / '\u{2022}'.len_utf8()).min(bytes.len() - 1)];
    for caret in &mut line.carets {
        caret.byte = map(caret.byte);
    }
    for cluster in &mut line.clusters {
        cluster.start = map(cluster.start);
        cluster.end = map(cluster.end);
    }
}
