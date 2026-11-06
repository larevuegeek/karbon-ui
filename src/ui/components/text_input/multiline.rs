//! Champ multiligne : mise en page des lignes, défilement vertical qui garde le caret
//! visible, caret et clic sur plusieurs lignes, touches propres (Haut/Bas en gardant la
//! colonne visuelle, pages, début et fin de ligne, Entrée).
use super::*;
use crate::ui::{runtime::Key, styling::TextAlign};

impl Editor {
    fn align(&self) -> TextAlign {
        self.font.map_or(TextAlign::Start, |f| f.align)
    }

    /// Caret affiché et son affinité (le caret de composition n'en a pas).
    fn shown_caret(&self) -> (usize, bool) {
        match &self.composition {
            Some(c) => (c.range.start + c.caret, false),
            None => (self.cursor, self.upstream),
        }
    }

    /// Met à jour les lignes (texte, police ou largeur changés) puis le défilement.
    pub(super) fn sync_rows(
        &mut self,
        input: &TextInput,
        text: &mut dyn TextMeasurer,
    ) -> crate::Result<()> {
        let format = input.format();
        let width = (layout::width(self.rect) - 2.).max(1.);
        let value = self.displayed().0.into_owned();
        let shown = if input.masked {
            value.chars().map(|_| '\u{2022}').collect()
        } else {
            value.clone()
        };
        if self.rows.as_ref().is_none_or(|r| r.width != width)
            || *self.display != *shown
            || self.font != Some(format)
        {
            let mut lines = text.layout_wrapped_text(&shown, format, width)?;
            if input.masked {
                let mut bytes = value.char_indices().map(|(b, _)| b).collect::<Vec<_>>();
                bytes.push(value.len());
                let map = |b: usize| bytes[(b / '\u{2022}'.len_utf8()).min(bytes.len() - 1)];
                for line in &mut lines {
                    super::state::unmask(&mut line.line, &value);
                    line.range = map(line.range.start)..map(line.range.end);
                }
            }
            let rows = Rows::new(lines, width, &value);
            let line = rows.flatten();
            if self.composition.is_none() {
                self.cursor = line.snap(self.cursor);
                self.anchor = line.snap(self.anchor);
            }
            self.line = Some(line);
            self.rows = Some(rows);
            self.display = shown.into();
            self.font = Some(format);
        }
        self.scroll = 0.;
        self.text_offset = 0.;
        let visible = layout::height(self.rect);
        let rows = self.rows.as_ref().unwrap();
        if self.follow || self.reveal.is_some() {
            let (cursor, upstream) = self.shown_caret();
            let row = match self.reveal {
                Some(byte) => &rows.lines[rows.index(byte, false)],
                None => &rows.lines[rows.index(cursor, upstream)],
            };
            if row.top < self.scroll_y {
                self.scroll_y = row.top;
            } else if row.bottom() > self.scroll_y + visible {
                self.scroll_y = row.bottom() - visible;
            }
            self.follow = false;
        }
        self.scroll_y = self.scroll_y.clamp(0., (rows.height - visible).max(0.));
        Ok(())
    }

    /// Caret d'un champ multiligne, en coordonnées de la fenêtre.
    pub(super) fn row_caret(&self) -> RectF {
        let rows = self.rows.as_ref().unwrap();
        let (cursor, upstream) = self.shown_caret();
        let index = rows.index(cursor, upstream);
        let row = &rows.lines[index];
        RectF::new(
            self.rect.left + rows.offset(index, self.align()) + row.line.x(cursor),
            self.rect.top + row.top - self.scroll_y,
            1.,
            row.line.height,
        )
    }

    /// Octet sous le point `p` (fenêtre) et son affinité.
    pub(super) fn row_hit(&self, p: PointF) -> (usize, bool) {
        let rows = self.rows.as_ref().unwrap();
        let index = rows.at_y(p.y - self.rect.top + self.scroll_y);
        rows.hit(index, p.x - self.rect.left, self.align())
    }

    /// Déplace le caret de `lines` lignes visuelles en gardant la colonne visée.
    fn vertical(&mut self, lines: isize, extend: bool) {
        let rows = self.rows.as_ref().unwrap();
        let align = self.align();
        let index = rows.index(self.cursor, self.upstream);
        let goal = self
            .goal
            .unwrap_or_else(|| rows.offset(index, align) + rows.lines[index].line.x(self.cursor));
        let target = index as isize + lines;
        let (byte, upstream) = if target < 0 {
            (0, false)
        } else if target as usize >= rows.lines.len() {
            (self.text.len(), false)
        } else {
            rows.hit(target as usize, goal, align)
        };
        self.move_to(byte, extend);
        self.upstream = upstream;
        self.goal = Some(goal);
    }

    /// Touches propres au champ multiligne : `Some(changed)` si la touche est traitée
    /// ici, `None` pour le traitement commun.
    pub(super) fn multiline_key(
        &mut self,
        key: Key,
        shift: bool,
        input: &TextInput,
    ) -> Option<bool> {
        let rows = self.rows.as_ref()?;
        let page = {
            let height = rows.lines[0].line.height.max(1.);
            ((layout::height(self.rect) / height).floor() as isize).max(1)
        };
        let index = rows.index(self.cursor, self.upstream);
        let (start, end, soft) = (
            rows.lines[index].range.start,
            rows.lines[index].range.end,
            rows.soft(index),
        );
        match key {
            Key::Up => self.vertical(-1, shift),
            Key::Down => self.vertical(1, shift),
            Key::PageUp => self.vertical(-page, shift),
            Key::PageDown => self.vertical(page, shift),
            Key::Home => self.move_to(start, shift),
            Key::End => {
                self.move_to(end, shift);
                self.upstream = soft;
            }
            Key::Enter => {
                return Some(!input.read_only && self.replace("\n", input.max_chars));
            }
            _ => return None,
        }
        Some(false)
    }
}
impl TextInputs {
    /// Fait défiler verticalement le contenu d'un champ multiligne de `delta` ; renvoie
    /// le défilement effectivement appliqué (0 si le champ est au bord ou monoligne).
    pub fn scroll_by(&mut self, id: ElementId, delta: f32) -> f32 {
        let Some(e) = self.states.get_mut(&id) else {
            return 0.;
        };
        let Some(rows) = &e.rows else {
            return 0.;
        };
        let max = (rows.height - layout::height(e.rect)).max(0.);
        let target = (e.scroll_y + delta).clamp(0., max);
        let consumed = target - e.scroll_y;
        if consumed != 0. {
            e.scroll_y = target;
            e.reveal = None;
            e.follow = false;
            self.revision = self.revision.wrapping_add(1);
        }
        consumed
    }
}
