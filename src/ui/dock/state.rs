//! L'état d'une zone ancrée tenu par l'application : son arbre, et la place visée par
//! un onglet qu'on tire. Les gestes de la vue arrivent par [`DockMsg`].
use super::tree::{DockSide, DockTree};
use crate::ui::scene::RectF;

/// A gesture of a [`DockArea`](super::DockArea), to pass to [`DockState::update`]. The
/// view builds them; an application only forwards them.
#[derive(Clone, Debug, PartialEq)]
pub struct DockMsg(pub(super) Inner);

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Inner {
    /// Clic sur un onglet.
    Activate(u64),
    /// Clic sur la croix d'un onglet.
    Close(u64),
    /// Séparation déplacée : les onglets qui commencent ses deux volets, et sa part.
    Share((u64, u64), f32),
    /// Un onglet tiré survole un groupe (l'onglet montré de ce groupe, le côté, et la
    /// boîte survolée dans la fenêtre), ou n'en survole plus.
    Aim(Option<(u64, DockSide, RectF)>),
    /// Un onglet lâché sur un groupe.
    Drop {
        panel: u64,
        target: u64,
        side: DockSide,
    },
    /// Fin du glisser d'un onglet, lâché ou non.
    DragEnd,
}

/// What a dock area tells the application after a gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DockEvent {
    /// A tab was clicked: its panel is shown.
    Activated(u64),
    /// The close button of a tab was clicked: the panel left the layout.
    Closed(u64),
    /// A tab was dropped at `side` of the group showing `target`.
    Moved {
        /// The panel moved.
        panel: u64,
        /// A panel of the group it was dropped on.
        target: u64,
        /// Where it landed.
        side: DockSide,
    },
    /// A divider moved: save the layout if it is kept.
    Resized,
}

/// Answer of [`DockState::update`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DockReply {
    /// The area changed on screen: return `true` from `Application::update`.
    pub rebuild: bool,
    /// What the application may act on (save the layout, close a panel for good).
    pub event: Option<DockEvent>,
}

/// State of a [`DockArea`](super::DockArea), kept by the application: the layout, and
/// where a dragged tab would land.
///
/// ```
/// use karbon_ui::ui::dock::{DockState, DockTree};
///
/// let state = DockState::new(DockTree::single(1));
/// assert_eq!(state.tree().panels(), [1]);
/// ```
#[derive(Clone, Debug, Default)]
pub struct DockState {
    pub(super) tree: DockTree,
    pub(super) aim: Option<(u64, DockSide)>,
    /// Boîte survolée par l'onglet tiré, dans la fenêtre : la marque s'y dessine.
    pub(super) aim_rect: Option<RectF>,
}

impl DockState {
    /// The state of `tree`.
    pub fn new(tree: DockTree) -> Self {
        Self {
            tree,
            aim: None,
            aim_rect: None,
        }
    }

    /// The layout.
    pub fn tree(&self) -> &DockTree {
        &self.tree
    }

    /// The layout, to dock or remove panels from the application (a menu « Show the
    /// console »); rebuild the view afterwards.
    pub fn tree_mut(&mut self) -> &mut DockTree {
        &mut self.tree
    }

    /// Where a dragged tab would land now: a panel of the group under the pointer and
    /// the side.
    pub fn aim(&self) -> Option<(u64, DockSide)> {
        self.aim
    }

    /// Apply a gesture of the view ([`DockMsg`]).
    pub fn update(&mut self, message: DockMsg) -> DockReply {
        let (rebuild, event) = match message.0 {
            Inner::Activate(panel) => {
                (self.tree.activate(panel), Some(DockEvent::Activated(panel)))
            }
            Inner::Close(panel) => (self.tree.remove(panel), Some(DockEvent::Closed(panel))),
            Inner::Share(key, fraction) => (
                false,
                self.tree
                    .set_fraction(key, fraction)
                    .then_some(DockEvent::Resized),
            ),
            Inner::Aim(aim) => {
                let rect = aim.map(|a| a.2);
                let aim = aim.map(|a| (a.0, a.1));
                let changed = self.aim != aim || self.aim_rect != rect;
                self.aim = aim;
                self.aim_rect = rect;
                (changed, None)
            }
            Inner::Drop {
                panel,
                target,
                side,
            } => {
                self.aim = None;
                self.aim_rect = None;
                let moved = self.tree.dock(panel, target, side);
                (
                    true,
                    moved.then_some(DockEvent::Moved {
                        panel,
                        target,
                        side,
                    }),
                )
            }
            Inner::DragEnd => {
                self.aim_rect = None;
                (self.aim.take().is_some(), None)
            }
        };
        DockReply {
            rebuild: rebuild || event.is_some_and(|e| e != DockEvent::Resized),
            event,
        }
    }
}
