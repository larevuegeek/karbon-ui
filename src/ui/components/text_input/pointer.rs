//! Pointeur dans un champ : un clic place le caret (et replie la sélection), le glisser
//! étend la sélection ; un double clic sélectionne le mot sous le pointeur sans les
//! espaces (ou la suite d'espaces visée), un triple clic la ligne entière (tout le texte
//! d'un champ sur une ligne, la ligne logique d'un champ multiligne). Le compte des clics
//! est tenu ici, à partir de l'horloge du runtime, comme dans un champ natif.
use super::*;
use std::time::Instant;

impl TextInputs {
    pub fn pointer_down(&mut self, id: ElementId, p: PointF, extend: bool, now: Instant) {
        let count = match self.click {
            Some(c)
                if c.id == id
                    && !extend
                    && now.duration_since(c.at) <= MULTI_CLICK
                    && (p.x - c.position.x).abs() <= MULTI_CLICK_DISTANCE
                    && (p.y - c.position.y).abs() <= MULTI_CLICK_DISTANCE =>
            {
                c.count % 3 + 1
            }
            _ => 1,
        };
        self.click = Some(Click {
            id,
            position: p,
            at: now,
            count,
        });
        let Some(e) = self.states.get_mut(&id) else {
            return;
        };
        e.composition = None;
        let (byte, upstream) = if e.rows.is_some() {
            e.row_hit(p)
        } else {
            let line = e.line.as_ref().unwrap();
            (
                line.hit(p.x - e.rect.left - e.text_offset + e.scroll),
                false,
            )
        };
        match count {
            2 => {
                let range = word_at(&e.text, byte, e.multiline);
                e.move_to(range.start, false);
                e.move_to(range.end, true);
            }
            3 => {
                let range = line_at(&e.text, byte, e.multiline);
                e.move_to(range.start, false);
                e.move_to(range.end, true);
            }
            _ => {
                e.move_to(byte, extend);
                e.upstream = upstream;
            }
        }
        self.dragging = Some(id);
        self.caret_visible = true;
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn pointer_move(&mut self, p: PointF) -> bool {
        // Après un double ou un triple clic, le glisser garde la sélection du mot ou de
        // la ligne.
        if self.click.is_some_and(|c| c.count > 1) {
            return false;
        }
        if let Some(e) = self.dragging.and_then(|id| self.states.get_mut(&id)) {
            if e.rows.is_some() {
                let (byte, upstream) = e.row_hit(p);
                e.cursor = byte;
                e.upstream = upstream;
                e.goal = None;
                // Glisser au-delà du bord fait défiler : le caret suit le pointeur.
                e.follow = true;
                self.caret_visible = true;
                self.revision = self.revision.wrapping_add(1);
                return true;
            }
            e.cursor = e
                .line
                .as_ref()
                .unwrap()
                .hit(p.x - e.rect.left - e.text_offset + e.scroll);
            self.caret_visible = true;
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }
}

/// Ligne logique qui contient l'octet `byte` : tout le texte d'un champ sur une ligne,
/// sinon de l'octet qui suit le saut de ligne précédent au saut de ligne suivant (exclu).
fn line_at(text: &str, byte: usize, multiline: bool) -> Range<usize> {
    if !multiline {
        return 0..text.len();
    }
    let byte = byte.min(text.len());
    let start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    let end = text[byte..].find('\n').map_or(text.len(), |i| byte + i);
    start..end
}

/// Mot (suite de caractères hors blancs) ou suite de blancs visée à l'octet `byte`,
/// dans sa ligne. Un appui à la fin d'un mot (frontière avec un blanc ou la fin du
/// texte) désigne ce mot.
fn word_at(text: &str, byte: usize, multiline: bool) -> Range<usize> {
    let line = line_at(text, byte, multiline);
    let text = &text[line.clone()];
    let byte = byte.clamp(line.start, line.end) - line.start;
    let space = |c: char| c.is_whitespace();
    let at = text[byte..].chars().next();
    let before = text[..byte].chars().next_back();
    // Classe du caractère visé : le précédent quand on est à la fin d'un mot.
    let blank = match (at, before) {
        (Some(c), Some(b)) if space(c) && !space(b) => false,
        (Some(c), _) => space(c),
        (None, Some(b)) => space(b),
        (None, None) => return line.start..line.start,
    };
    let start = text[..byte]
        .trim_end_matches(|c: char| space(c) == blank)
        .len();
    let end = byte
        + text[byte..]
            .find(|c: char| space(c) != blank)
            .unwrap_or(text.len() - byte);
    line.start + start..line.start + end
}

#[cfg(test)]
mod tests {
    use super::{line_at, word_at};

    #[test]
    fn words_and_lines_are_found_around_a_byte() {
        let text = "ab  cd";
        assert_eq!(word_at(text, 0, false), 0..2);
        assert_eq!(word_at(text, 2, false), 0..2);
        assert_eq!(word_at(text, 3, false), 2..4);
        assert_eq!(word_at(text, 5, false), 4..6);
        assert_eq!(word_at(text, 6, false), 4..6);
        assert_eq!(word_at("", 0, false), 0..0);
        assert_eq!(word_at("été", 2, false), 0..5);
        let lines = "abc\nde fg\nij";
        assert_eq!(line_at(lines, 0, false), 0..12);
        assert_eq!(line_at(lines, 5, true), 4..9);
        assert_eq!(line_at(lines, 9, true), 4..9);
        assert_eq!(line_at(lines, 10, true), 10..12);
        assert_eq!(word_at(lines, 8, true), 7..9);
        assert_eq!(word_at(lines, 3, true), 0..3);
    }
}
