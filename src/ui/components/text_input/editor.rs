use super::*;
impl Editor {
    pub(super) fn new(value: &str, max: usize, multiline: bool) -> Self {
        let text = sanitize(value, max, multiline);
        let end = text.len();
        Self {
            text,
            anchor: end,
            cursor: end,
            line: None,
            display: "".into(),
            composition: None,
            scroll: 0.,
            text_offset: 0.,
            rect: RectF::new(0., 0., 0., 0.),
            clip: RectF::new(0., 0., 0., 0.),
            undo: Vec::new(),
            redo: Vec::new(),
            font: None,
            reveal: None,
            multiline,
            rows: None,
            scroll_y: 0.,
            follow: false,
            upstream: false,
            goal: None,
        }
    }

    pub(super) fn range(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            anchor: self.anchor,
            cursor: self.cursor,
        }
    }

    /// Arrêt du curseur avant `byte` : celui de la ligne mesurée (grappe), ou le caractère
    /// précédent. La ligne date de la dernière préparation : quand `UiRuntime::event` est
    /// appelé sans `prepare` entre deux frappes, ses arrêts ne correspondent plus au
    /// texte. Elle ne sert que si elle finit avec lui, et seul un arrêt qui recule et
    /// tombe sur une frontière de caractère est pris.
    pub(super) fn previous_stop(&self, byte: usize) -> usize {
        let byte = self.boundary(byte);
        let character = self.text[..byte]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i);
        match self.current_line().map(|line| line.previous(byte)) {
            Some(stop) if stop < byte && self.text.is_char_boundary(stop) => stop,
            _ => character,
        }
    }

    /// Arrêt du curseur après `byte` (voir [`previous_stop`](Self::previous_stop)) :
    /// avance toujours tant que `byte` n'est pas la fin du texte.
    pub(super) fn next_stop(&self, byte: usize) -> usize {
        let byte = self.boundary(byte);
        let character = self.text[byte..]
            .chars()
            .next()
            .map_or(byte, |c| byte + c.len_utf8());
        match self.current_line().map(|line| line.next(byte)) {
            Some(stop) if stop > byte && self.text.is_char_boundary(stop) => stop,
            _ => character,
        }
    }

    /// Ligne mesurée, si elle l'a été pour le texte actuel (son dernier arrêt est sa fin).
    fn current_line(&self) -> Option<&TextLine> {
        self.line
            .as_ref()
            .filter(|line| line.carets.last().map(|c| c.byte) == Some(self.text.len()))
    }

    /// `byte` ramené dans le texte, sur une frontière de caractère.
    fn boundary(&self, byte: usize) -> usize {
        let mut byte = byte.min(self.text.len());
        while !self.text.is_char_boundary(byte) {
            byte -= 1;
        }
        byte
    }

    pub(super) fn push_undo(&mut self) {
        self.undo.push(self.snapshot());
        trim_history(&mut self.undo);
        self.redo.clear();
    }

    pub(super) fn replace(&mut self, value: &str, max: usize) -> bool {
        self.composition = None;
        self.reveal = None;
        let range = self.range();
        let remaining = max.saturating_sub(
            self.text[..range.start].chars().count() + self.text[range.end..].chars().count(),
        );
        let value = sanitize(value, remaining, self.multiline);
        self.follow = true;
        self.upstream = false;
        self.goal = None;
        if self.text[range.clone()] == value {
            self.anchor = range.start + value.len();
            self.cursor = self.anchor;
            return false;
        }
        self.push_undo();
        self.text.replace_range(range.clone(), &value);
        self.cursor = range.start + value.len();
        self.anchor = self.cursor;
        true
    }

    pub(super) fn move_to(&mut self, position: usize, extend: bool) {
        self.reveal = None;
        self.follow = true;
        self.upstream = false;
        self.goal = None;
        // Position lue sur une mesure en retard sur le texte (voir `previous_stop`).
        let position = self.boundary(position);
        self.cursor = position;
        if !extend {
            self.anchor = position;
        }
    }

    pub(super) fn displayed(&self) -> (Cow<'_, str>, usize, Option<Range<usize>>) {
        if let Some(c) = &self.composition {
            let mut value = self.text.clone();
            value.replace_range(c.range.clone(), &c.value);
            (
                Cow::Owned(value),
                c.range.start + c.caret,
                Some(c.range.start..c.range.start + c.value.len()),
            )
        } else {
            (Cow::Borrowed(&self.text), self.cursor, None)
        }
    }

    pub(super) fn caret(&self) -> RectF {
        if self.rows.is_some() {
            return self.row_caret();
        }
        let cursor = self
            .composition
            .as_ref()
            .map_or(self.cursor, |c| c.range.start + c.caret);
        let line = self.line.as_ref().unwrap();
        RectF::new(
            self.rect.left + self.text_offset + line.x(cursor) - self.scroll,
            self.rect.top + (layout::height(self.rect) - line.height).max(0.) / 2.,
            1.,
            line.height,
        )
    }
}
pub(super) fn trim_history(history: &mut Vec<Snapshot>) {
    while history.len() > 100 || history.iter().map(|s| s.text.len()).sum::<usize>() > 1_048_576 {
        history.remove(0);
    }
}
/// Sanitise `value` for a single-line field: CR, LF and tab become spaces, other
/// control characters are removed, and at most `max` characters are kept.
pub fn single_line(value: &str, max: usize) -> String {
    value
        .chars()
        .filter_map(|c| {
            if matches!(c, '\r' | '\n' | '\t') {
                Some(' ')
            } else if c.is_control() {
                None
            } else {
                Some(c)
            }
        })
        .take(max)
        .collect()
}
/// Sanitise `value` for a multiline field: CR LF, CR, U+2028 and U+2029 become a
/// line feed, tabs become spaces, other control characters are removed, and at most
/// `max` characters are kept (a line feed counts as one).
pub fn multi_line(value: &str, max: usize) -> String {
    let mut result = String::with_capacity(value.len());
    let mut count = 0;
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if count >= max {
            break;
        }
        let c = match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                '\n'
            }
            '\n' | '\u{2028}' | '\u{2029}' => '\n',
            '\t' => ' ',
            c if c.is_control() => continue,
            c => c,
        };
        result.push(c);
        count += 1;
    }
    result
}
/// Assainit selon le type de champ.
pub(super) fn sanitize(value: &str, max: usize, multiline: bool) -> String {
    if multiline {
        multi_line(value, max)
    } else {
        single_line(value, max)
    }
}
