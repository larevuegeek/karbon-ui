//! Le clavier dans un arbre : flèches pour aller d'une ligne à l'autre, gauche et droite
//! pour plier et déplier, Maj pour étendre la sélection, Ctrl pour déplacer le curseur
//! seul, Entrée pour ouvrir.
use super::{
    rows::{TreeSource, parent_index},
    state::{TreeEvent, TreeReply, TreeState},
};
use crate::ui::runtime::{KeyCode, KeyPress};
use std::rc::Rc;

impl TreeState {
    /// Apply a key given to [`Application::key`](crate::application::Application::key)
    /// while the tree is the active panel:
    ///
    /// | Key | Effect |
    /// | --- | --- |
    /// | Up, Down, Home, End, Page Up, Page Down | move the cursor and select its row |
    /// | the same with Shift | extend the selection from the anchor row |
    /// | the same with Ctrl | move the cursor only |
    /// | Ctrl+Space | add or remove the cursor row from the selection |
    /// | Left | collapse the cursor node, or go to its parent |
    /// | Right | expand the cursor node, or go to its first child |
    /// | Enter | [`TreeEvent::Activated`] on the cursor row |
    /// | Ctrl+A | select every visible row |
    /// | Escape | cancel a drag |
    ///
    /// Releases and other keys change nothing (`rebuild` stays `false`). The view that
    /// follows a move scrolls to show the cursor row.
    ///
    /// ```
    /// use karbon_ui::ui::{
    ///     editor::{TreeSource, TreeState},
    ///     runtime::{KeyCode, KeyPress},
    /// };
    ///
    /// struct Flat;
    /// impl TreeSource for Flat {
    ///     fn children(&self, parent: Option<u64>) -> &[u64] {
    ///         if parent.is_none() { &[10, 20, 30] } else { &[] }
    ///     }
    /// }
    /// let mut tree = TreeState::new(&Flat);
    /// tree.select(10);
    /// let reply = tree.key(KeyPress::new(KeyCode::Down, true), &Flat);
    /// assert!(reply.rebuild);
    /// assert_eq!(tree.cursor(), Some(20));
    /// assert!(tree.is_selected(20) && !tree.is_selected(10));
    /// ```
    pub fn key(&mut self, press: KeyPress, source: &impl TreeSource) -> TreeReply {
        if !press.pressed || self.rows.is_empty() {
            return TreeReply::default();
        }
        let last = self.rows.len() - 1;
        let current = self.cursor.and_then(|key| self.index_of(key));
        let at = current.unwrap_or(0);
        let target = match press.key {
            KeyCode::Up => Some(if current.is_some() {
                at.saturating_sub(1)
            } else {
                0
            }),
            KeyCode::Down => Some(if current.is_some() {
                (at + 1).min(last)
            } else {
                0
            }),
            KeyCode::Home => Some(0),
            KeyCode::End => Some(last),
            KeyCode::PageUp => Some(at.saturating_sub(self.page)),
            KeyCode::PageDown => Some((at + self.page).min(last)),
            _ => None,
        };
        if let Some(index) = target {
            return self.move_cursor(index, press);
        }
        match press.key {
            KeyCode::Left => self.left(at, source),
            KeyCode::Right => self.right(at, source),
            KeyCode::Enter => match current {
                Some(index) => TreeReply {
                    rebuild: false,
                    event: Some(TreeEvent::Activated(self.rows[index].key)),
                },
                None => TreeReply::default(),
            },
            KeyCode::Space if press.control => {
                let key = self.rows[at].key;
                let selected = Rc::make_mut(&mut self.selected);
                if !selected.remove(&key) {
                    selected.insert(key);
                }
                self.cursor = Some(key);
                self.anchor = Some(key);
                self.selection_changed()
            }
            KeyCode::A if press.control => {
                let rows = Rc::clone(&self.rows);
                Rc::make_mut(&mut self.selected).extend(rows.iter().map(|row| row.key));
                self.selection_changed()
            }
            KeyCode::Escape if self.cancel_drag() => TreeReply {
                rebuild: true,
                event: None,
            },
            _ => TreeReply::default(),
        }
    }

    fn selection_changed(&self) -> TreeReply {
        TreeReply {
            rebuild: true,
            event: Some(TreeEvent::Selected),
        }
    }

    /// Curseur sur la ligne `index` : seul (Ctrl), en étendant la sélection (Maj) ou en
    /// choisissant sa ligne.
    fn move_cursor(&mut self, index: usize, press: KeyPress) -> TreeReply {
        let key = self.rows[index].key;
        self.reveal = Some(key);
        if press.shift {
            self.select_range(key, press.control);
        } else if press.control {
            self.cursor = Some(key);
            return TreeReply {
                rebuild: true,
                event: None,
            };
        } else {
            self.select_only(key);
        }
        self.selection_changed()
    }

    fn left(&mut self, at: usize, source: &impl TreeSource) -> TreeReply {
        let row = self.rows[at];
        if row.expanded {
            self.collapse_at(at, source);
            return TreeReply {
                rebuild: true,
                event: Some(TreeEvent::Expanded(row.key, false)),
            };
        }
        match parent_index(&self.rows, at) {
            Some(parent) => self.move_cursor(parent, KeyPress::new(KeyCode::Left, true)),
            None => TreeReply::default(),
        }
    }

    fn right(&mut self, at: usize, source: &impl TreeSource) -> TreeReply {
        let row = self.rows[at];
        if !row.has_children {
            return TreeReply::default();
        }
        if !row.expanded {
            self.cursor = Some(row.key);
            self.set_expanded(row.key, true, source);
            return TreeReply {
                rebuild: true,
                event: Some(TreeEvent::Expanded(row.key, true)),
            };
        }
        self.move_cursor(at + 1, KeyPress::new(KeyCode::Right, true))
    }
}
