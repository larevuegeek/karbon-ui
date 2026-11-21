//! État d'un arbre tenu par l'application : branches dépliées, sélection par clé, ligne
//! du curseur et geste de glisser en cours. Les gestes de la souris arrivent par
//! [`TreeMsg`] ; le clavier par `TreeState::key` (`keys.rs`).
use super::rows::{TreeRow, TreeSource, flatten, parent_index, subtree_end};
use crate::ui::runtime::{PointerButton, PointerEvent, PointerPhase};
use std::{collections::HashSet, rc::Rc};

/// Distance, en pixels logiques, qu'un appui doit parcourir pour devenir un glisser.
const DRAG_THRESHOLD: f32 = 4.;

/// Where a dragged row lands relative to the row under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPlace {
    /// Above the target, as its previous sibling.
    Before,
    /// Inside the target, as its last child: the gesture that changes a parent.
    Into,
    /// Below the target, as its next sibling.
    After,
}

/// Rows dropped on another row ([`TreeEvent::Dropped`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeDrop {
    /// Keys of the dragged nodes, in display order: the selection when the dragged row
    /// was selected, otherwise that row alone. Never contains the target nor one of its
    /// ancestors.
    pub keys: Vec<u64>,
    /// Key of the row under the pointer.
    pub target: u64,
    /// Where the rows land relative to the target.
    pub place: DropPlace,
}

/// What a tree tells the application after a gesture or a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    /// The selection changed; read it with [`TreeState::selection`].
    Selected,
    /// A row was double-clicked, or Enter was pressed on the cursor row: open it, frame
    /// it in the 3D view, rename it.
    Activated(u64),
    /// A node was expanded (`true`) or collapsed (`false`).
    Expanded(u64, bool),
    /// Rows were dropped on another row: change their parent or their place.
    Dropped(TreeDrop),
}

/// Answer of [`TreeState::update`] and [`TreeState::key`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeReply {
    /// The tree changed on screen: return `true` from `Application::update` to rebuild
    /// the view.
    pub rebuild: bool,
    /// What the application may act on.
    pub event: Option<TreeEvent>,
}

impl TreeReply {
    fn changed(event: Option<TreeEvent>) -> Self {
        Self {
            rebuild: true,
            event,
        }
    }
}

/// A gesture of the mouse on a [`TreeView`](super::TreeView), to pass to
/// [`TreeState::update`]. The view builds them; an application only forwards them.
#[derive(Clone, Debug, PartialEq)]
pub enum TreeMsg {
    /// A button, a move or a release on the row of `key`.
    Pointer {
        /// Row that received the event (the row where the gesture started, during it).
        key: u64,
        /// The event, with the position in that row.
        event: PointerEvent,
    },
    /// The list scrolled; its visible height, in logical pixels.
    Scrolled(f32),
}

/// Glisser en cours, depuis l'appui sur la ligne `origin`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Drag {
    pub(super) origin: u64,
    pub(super) start: (f32, f32),
    pub(super) moved: bool,
    /// Appui simple sur une ligne déjà choisie parmi d'autres : la sélection ne se réduit
    /// à elle qu'au relâchement, si l'appui n'est pas devenu un glisser.
    pub(super) collapse_on_release: bool,
    pub(super) target: Option<(usize, DropPlace)>,
}

/// State of a [`TreeView`](super::TreeView), kept by the application: expanded
/// branches, selection by key (kept through sorting and filtering), cursor row and the
/// drag in progress. Only the visible rows are stored, flattened in display order.
///
/// ```
/// use karbon_ui::ui::editor::{TreeSource, TreeState};
///
/// struct Two;
/// impl TreeSource for Two {
///     fn children(&self, parent: Option<u64>) -> &[u64] {
///         match parent {
///             None => &[1],
///             Some(1) => &[2],
///             Some(_) => &[],
///         }
///     }
/// }
/// let mut tree = TreeState::new(&Two);
/// assert_eq!(tree.rows().len(), 1);
/// tree.set_expanded(1, true, &Two);
/// assert_eq!(tree.rows().len(), 2);
/// ```
pub struct TreeState {
    pub(super) rows: Rc<Vec<TreeRow>>,
    pub(super) expanded: HashSet<u64>,
    pub(super) selected: Rc<HashSet<u64>>,
    pub(super) cursor: Option<u64>,
    pub(super) anchor: Option<u64>,
    /// Ligne à rendre visible à la prochaine vue : celle où le clavier a mené. Oubliée
    /// dès que l'utilisateur fait défiler la liste.
    pub(super) reveal: Option<u64>,
    /// Lignes visibles d'une page (Page précédente, Page suivante).
    pub(super) page: usize,
    pub(super) row_height: f32,
    pub(super) drag: Option<Drag>,
}

impl TreeState {
    /// A tree with every branch collapsed, showing the top-level nodes of `source`.
    pub fn new(source: &impl TreeSource) -> Self {
        let mut state = Self {
            rows: Rc::new(Vec::new()),
            expanded: HashSet::new(),
            selected: Rc::new(HashSet::new()),
            cursor: None,
            anchor: None,
            reveal: None,
            page: 10,
            row_height: super::view::ROW_HEIGHT,
            drag: None,
        };
        state.refresh(source);
        state
    }

    /// Rows of `height` logical pixels instead of 22; the [`TreeView`](super::TreeView)
    /// of this state uses it.
    pub fn with_row_height(mut self, height: f32) -> Self {
        self.row_height = height.max(1.);
        self
    }

    /// Read `source` again after the application changed it (nodes added, removed,
    /// moved, sorted). The selection is kept by key; the cursor goes to the first row
    /// when its node is no longer visible.
    pub fn refresh(&mut self, source: &impl TreeSource) {
        self.rows = Rc::new(flatten(source, &self.expanded));
        if self.cursor.is_some_and(|key| self.index_of(key).is_none()) {
            self.cursor = self.rows.first().map(|row| row.key);
        }
    }

    /// The visible rows, in display order.
    pub fn rows(&self) -> &[TreeRow] {
        &self.rows
    }

    /// Keys of the selected nodes, in no particular order (hidden nodes included).
    pub fn selection(&self) -> &HashSet<u64> {
        &self.selected
    }

    /// `true` when the node of `key` is selected.
    pub fn is_selected(&self, key: u64) -> bool {
        self.selected.contains(&key)
    }

    /// Key of the cursor row, the one the arrow keys move from.
    pub fn cursor(&self) -> Option<u64> {
        self.cursor
    }

    /// `true` when the node of `key` shows its children.
    pub fn is_expanded(&self, key: u64) -> bool {
        self.expanded.contains(&key)
    }

    /// Select only `key`, put the cursor on it and show its row at the next view.
    pub fn select(&mut self, key: u64) {
        self.select_only(key);
        self.reveal = Some(key);
    }

    /// Expand or collapse the node of `key`, then read `source` again.
    pub fn set_expanded(&mut self, key: u64, expanded: bool, source: &impl TreeSource) {
        if expanded {
            self.expanded.insert(key);
        } else {
            self.expanded.remove(&key);
        }
        self.refresh(source);
    }

    /// Apply a gesture of the view ([`TreeMsg`]).
    pub fn update(&mut self, message: TreeMsg, source: &impl TreeSource) -> TreeReply {
        match message {
            TreeMsg::Scrolled(height) => {
                self.reveal = None;
                self.page = ((height / self.row_height.max(1.)).floor() as usize).max(1);
                TreeReply::default()
            }
            TreeMsg::Pointer { key, event } => match event.phase {
                PointerPhase::Down => self.press(key, event, source),
                PointerPhase::Move => self.drag_to(key, event),
                PointerPhase::Up => self.release(),
                _ => TreeReply::default(),
            },
        }
    }

    /// Ligne de `key` parmi les lignes visibles.
    pub(super) fn index_of(&self, key: u64) -> Option<usize> {
        self.rows.iter().position(|row| row.key == key)
    }

    pub(super) fn select_only(&mut self, key: u64) {
        let selected = Rc::make_mut(&mut self.selected);
        selected.clear();
        selected.insert(key);
        self.cursor = Some(key);
        self.anchor = Some(key);
    }

    /// Choisir les lignes de l'ancre à `key` (Maj) ; avec `add` (Ctrl+Maj), les ajouter à
    /// la sélection au lieu de la remplacer.
    pub(super) fn select_range(&mut self, key: u64, add: bool) {
        let anchor = self.anchor.unwrap_or(key);
        let (Some(from), Some(to)) = (self.index_of(anchor), self.index_of(key)) else {
            return self.select_only(key);
        };
        let (start, end) = (from.min(to), from.max(to));
        let rows = Rc::clone(&self.rows);
        let selected = Rc::make_mut(&mut self.selected);
        if !add {
            selected.clear();
        }
        selected.extend(rows[start..=end].iter().map(|row| row.key));
        self.cursor = Some(key);
    }

    fn press(&mut self, key: u64, event: PointerEvent, source: &impl TreeSource) -> TreeReply {
        let Some(index) = self.index_of(key) else {
            return TreeReply::default();
        };
        let row = self.rows[index];
        let arrow = f32::from(row.depth) * super::view::INDENT;
        let on_arrow = event.local_x >= arrow && event.local_x < arrow + super::view::INDENT;
        if event.button == Some(PointerButton::Primary) && row.has_children && on_arrow {
            self.set_expanded(key, !row.expanded, source);
            return TreeReply::changed(Some(TreeEvent::Expanded(key, !row.expanded)));
        }
        let selected = self.selected.contains(&key);
        if event.button == Some(PointerButton::Secondary) {
            // Clic droit : la ligne rejoint la sélection pour le menu contextuel.
            if !selected {
                self.select_only(key);
            }
            return TreeReply::changed(Some(TreeEvent::Selected));
        }
        if event.button != Some(PointerButton::Primary) {
            return TreeReply::default();
        }
        let mut collapse_on_release = false;
        if event.shift {
            self.select_range(key, event.control);
        } else if event.control {
            let set = Rc::make_mut(&mut self.selected);
            if !set.remove(&key) {
                set.insert(key);
            }
            self.cursor = Some(key);
            self.anchor = Some(key);
        } else if selected && self.selected.len() > 1 {
            collapse_on_release = true;
            self.cursor = Some(key);
            self.anchor = Some(key);
        } else {
            self.select_only(key);
        }
        self.drag = Some(Drag {
            origin: key,
            start: (event.x, event.y),
            moved: false,
            collapse_on_release,
            target: None,
        });
        let event = if event.clicks == 2 {
            TreeEvent::Activated(key)
        } else {
            TreeEvent::Selected
        };
        TreeReply::changed(Some(event))
    }

    fn drag_to(&mut self, key: u64, event: PointerEvent) -> TreeReply {
        let Some(mut drag) = self.drag.filter(|d| d.origin == key) else {
            return TreeReply::default();
        };
        let (dx, dy) = (event.x - drag.start.0, event.y - drag.start.1);
        drag.moved |= dx.hypot(dy) >= DRAG_THRESHOLD;
        let target = if drag.moved {
            self.drop_target(key, event.local_y)
        } else {
            None
        };
        let changed = target != drag.target || drag.moved != self.drag.is_some_and(|d| d.moved);
        drag.target = target;
        self.drag = Some(drag);
        TreeReply {
            rebuild: changed,
            event: None,
        }
    }

    fn release(&mut self) -> TreeReply {
        let Some(drag) = self.drag.take() else {
            return TreeReply::default();
        };
        if !drag.moved {
            if drag.collapse_on_release {
                self.select_only(drag.origin);
                return TreeReply::changed(Some(TreeEvent::Selected));
            }
            return TreeReply::default();
        }
        let Some((index, place)) = drag.target else {
            return TreeReply::changed(None);
        };
        let keys = self.dragged(drag.origin);
        let drop = TreeDrop {
            keys,
            target: self.rows[index].key,
            place,
        };
        TreeReply::changed(Some(TreeEvent::Dropped(drop)))
    }

    /// Abandonner le glisser en cours (Échap) ; `true` s'il y en avait un.
    pub(super) fn cancel_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }

    /// Clés glissées depuis `origin`, dans l'ordre d'affichage : la sélection si `origin`
    /// en fait partie, sinon `origin` seule.
    pub(super) fn dragged(&self, origin: u64) -> Vec<u64> {
        if !self.selected.contains(&origin) {
            return vec![origin];
        }
        self.rows
            .iter()
            .filter(|row| self.selected.contains(&row.key))
            .map(|row| row.key)
            .collect()
    }

    /// Ligne et place de dépôt sous le pointeur, `local_y` étant mesuré depuis le haut de
    /// la ligne `origin` ; `None` hors des lignes ou sur une ligne glissée (ou dans l'un de
    /// ses descendants).
    pub(super) fn drop_target(&self, origin: u64, local_y: f32) -> Option<(usize, DropPlace)> {
        let from = self.index_of(origin)?;
        let stride = self.row_height.max(1.);
        let rows_down = (local_y / stride).floor();
        let index = usize::try_from(from as i64 + rows_down as i64).ok()?;
        if index >= self.rows.len() {
            return None;
        }
        let within = (local_y - rows_down * stride) / stride;
        let place = if within < 0.25 {
            DropPlace::Before
        } else if within > 0.75 {
            DropPlace::After
        } else {
            DropPlace::Into
        };
        let dragged: HashSet<u64> = self.dragged(origin).into_iter().collect();
        // La cible et ses ancêtres ne doivent pas être glissés.
        let mut row = Some(index);
        while let Some(at) = row {
            if dragged.contains(&self.rows[at].key) {
                return None;
            }
            row = parent_index(&self.rows, at);
        }
        Some((index, place))
    }

    /// Plier le nœud de la ligne `index` ; le curseur, s'il était dans ses descendants,
    /// remonte sur lui.
    pub(super) fn collapse_at(&mut self, index: usize, source: &impl TreeSource) {
        let key = self.rows[index].key;
        let end = subtree_end(&self.rows, index);
        if self
            .cursor
            .is_some_and(|cursor| self.rows[index + 1..end].iter().any(|r| r.key == cursor))
        {
            self.cursor = Some(key);
        }
        self.set_expanded(key, false, source);
    }
}
