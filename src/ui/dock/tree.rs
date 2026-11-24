//! L'arbre d'une disposition ancrée : des groupes d'onglets, partagés deux à deux par des
//! séparations horizontales ou verticales. Un onglet se pose à gauche, à droite, en
//! haut, en bas ou au centre d'un groupe ; un groupe vidé disparaît avec sa séparation.
use crate::ui::element::Axis;

/// Where a panel lands relative to a group of tabs ([`DockTree::dock`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSide {
    /// A new group to the left of the target group.
    Left,
    /// A new group to the right.
    Right,
    /// A new group above.
    Top,
    /// A new group below.
    Bottom,
    /// One more tab in the target group.
    Center,
}

/// A node of a [`DockTree`]: a group of tabs, or two nodes side by side or stacked.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DockNode {
    /// Panels shown as tabs, one at a time.
    Tabs {
        /// Keys of the panels, in tab order (never empty).
        panels: Vec<u64>,
        /// The panel shown.
        active: u64,
    },
    /// Two nodes sharing the space.
    Split {
        /// [`Axis::Horizontal`]: side by side; [`Axis::Vertical`]: stacked.
        axis: Axis,
        /// Share of the first node, from 0.05 to 0.95.
        fraction: f32,
        /// Left or top node.
        first: Box<DockNode>,
        /// Right or bottom node.
        second: Box<DockNode>,
    },
}

impl DockNode {
    /// A group of one tab.
    pub fn tab(panel: u64) -> Self {
        Self::Tabs {
            panels: vec![panel],
            active: panel,
        }
    }

    /// A group of tabs showing `active` (the first panel when `active` is not among
    /// them). `panels` must not be empty.
    pub fn tabs(panels: Vec<u64>, active: u64) -> Self {
        let active = if panels.contains(&active) {
            active
        } else {
            panels[0]
        };
        Self::Tabs { panels, active }
    }

    /// Two nodes side by side (`Axis::Horizontal`) or stacked, the first taking
    /// `fraction` of the space.
    pub fn split(axis: Axis, fraction: f32, first: DockNode, second: DockNode) -> Self {
        Self::Split {
            axis,
            fraction: fraction.clamp(0.05, 0.95),
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// The panels of this node, left to right and top to bottom.
    pub fn panels(&self) -> Vec<u64> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<u64>) {
        match self {
            Self::Tabs { panels, .. } => out.extend(panels),
            Self::Split { first, second, .. } => {
                first.collect(out);
                second.collect(out);
            }
        }
    }

    /// Premier onglet du nœud : il nomme une séparation de façon stable.
    pub(super) fn first_panel(&self) -> u64 {
        match self {
            Self::Tabs { panels, .. } => panels[0],
            Self::Split { first, .. } => first.first_panel(),
        }
    }

    /// Le groupe qui contient `panel`.
    fn group_mut(&mut self, panel: u64) -> Option<&mut DockNode> {
        match self {
            Self::Tabs { panels, .. } if panels.contains(&panel) => Some(self),
            Self::Tabs { .. } => None,
            Self::Split { first, second, .. } => match first.group_mut(panel) {
                Some(group) => Some(group),
                None => second.group_mut(panel),
            },
        }
    }

    /// Retire `panel` ; `true` si le nœud entier est vide ensuite. Une séparation dont
    /// une moitié se vide est remplacée par l'autre moitié.
    fn take(&mut self, panel: u64) -> bool {
        match self {
            Self::Tabs { panels, active } => {
                let Some(index) = panels.iter().position(|&p| p == panel) else {
                    return false;
                };
                panels.remove(index);
                if *active == panel
                    && let Some(&next) = panels.get(index.min(panels.len().saturating_sub(1)))
                {
                    *active = next;
                }
                panels.is_empty()
            }
            Self::Split { first, second, .. } => {
                let kept = if first.take(panel) {
                    Some(std::mem::replace(second.as_mut(), DockNode::tab(0)))
                } else if second.take(panel) {
                    Some(std::mem::replace(first.as_mut(), DockNode::tab(0)))
                } else {
                    None
                };
                if let Some(kept) = kept {
                    *self = kept;
                }
                false
            }
        }
    }

    /// La part de la séparation dont les deux volets commencent par les onglets `key`.
    fn split_mut(&mut self, key: (u64, u64)) -> Option<&mut f32> {
        let Self::Split {
            fraction,
            first,
            second,
            ..
        } = self
        else {
            return None;
        };
        if (first.first_panel(), second.first_panel()) == key {
            return Some(fraction);
        }
        match first.split_mut(key) {
            Some(found) => Some(found),
            None => second.split_mut(key),
        }
    }
}

/// The layout of docked panels: groups of tabs split side by side or stacked, as in
/// the editors of game engines. It holds only keys of the application's panels; a
/// [`DockArea`](super::DockArea) shows it, [`to_json`](Self::to_json) and
/// [`from_json`](Self::from_json) keep it.
///
/// ```
/// use karbon_ui::ui::dock::{DockSide, DockTree};
///
/// let mut layout = DockTree::single(1);
/// layout.dock(2, 1, DockSide::Right);
/// layout.dock(3, 2, DockSide::Center);
/// assert_eq!(layout.panels(), [1, 2, 3]);
/// let saved = layout.to_json();
/// assert_eq!(DockTree::from_json(&saved).unwrap(), layout);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockTree {
    pub(super) root: Option<DockNode>,
}

impl DockTree {
    /// A layout showing `panel` alone.
    pub fn single(panel: u64) -> Self {
        Self {
            root: Some(DockNode::tab(panel)),
        }
    }

    /// A layout of `root`.
    pub fn from_node(root: DockNode) -> Self {
        Self { root: Some(root) }
    }

    /// The root node, `None` when no panel is docked.
    pub fn root(&self) -> Option<&DockNode> {
        self.root.as_ref()
    }

    /// Every docked panel, left to right and top to bottom.
    pub fn panels(&self) -> Vec<u64> {
        self.root.as_ref().map_or_else(Vec::new, DockNode::panels)
    }

    /// `true` when `panel` is docked.
    pub fn contains(&self, panel: u64) -> bool {
        self.panels().contains(&panel)
    }

    /// The panel shown by the group that holds `panel`.
    pub fn active_of(&self, panel: u64) -> Option<u64> {
        let mut root = self.root.clone()?;
        match root.group_mut(panel)? {
            DockNode::Tabs { active, .. } => Some(*active),
            DockNode::Split { .. } => None,
        }
    }

    /// Put `panel` (docked or new) at `side` of the group holding `target`: a tab of
    /// that group ([`DockSide::Center`], shown at once), or a new group beside it taking
    /// half of its space. `false`, and nothing changes, when `target` is not docked or
    /// when the move would change nothing (a panel alone in its group beside itself).
    pub fn dock(&mut self, panel: u64, target: u64, side: DockSide) -> bool {
        if !self.contains(target) || (panel == target && side == DockSide::Center) {
            return false;
        }
        if panel == target && self.group_len(panel) == 1 {
            return false;
        }
        // L'onglet tiré de son propre groupe vers un côté : le groupe reste la cible.
        let anchor = if panel == target {
            self.neighbour(panel)
        } else {
            Some(target)
        };
        self.remove(panel);
        let Some(anchor) = anchor else {
            return false;
        };
        let Some(group) = self.root.as_mut().and_then(|r| r.group_mut(anchor)) else {
            return false;
        };
        if side == DockSide::Center {
            if let DockNode::Tabs { panels, active } = group {
                panels.push(panel);
                *active = panel;
            }
            return true;
        }
        let old = std::mem::replace(group, DockNode::tab(0));
        let new = DockNode::tab(panel);
        *group = match side {
            DockSide::Left => DockNode::split(Axis::Horizontal, 0.5, new, old),
            DockSide::Right => DockNode::split(Axis::Horizontal, 0.5, old, new),
            DockSide::Top => DockNode::split(Axis::Vertical, 0.5, new, old),
            DockSide::Bottom | DockSide::Center => DockNode::split(Axis::Vertical, 0.5, old, new),
        };
        true
    }

    /// Nombre d'onglets du groupe de `panel`.
    fn group_len(&self, panel: u64) -> usize {
        let mut root = match self.root.clone() {
            Some(root) => root,
            None => return 0,
        };
        match root.group_mut(panel) {
            Some(DockNode::Tabs { panels, .. }) => panels.len(),
            _ => 0,
        }
    }

    /// Un autre onglet du groupe de `panel`.
    fn neighbour(&self, panel: u64) -> Option<u64> {
        let mut root = self.root.clone()?;
        match root.group_mut(panel)? {
            DockNode::Tabs { panels, .. } => panels.iter().copied().find(|&p| p != panel),
            DockNode::Split { .. } => None,
        }
    }

    /// Take `panel` out of the layout (closed, or floating in its own window); its group
    /// disappears when it was its last tab. `false` when it was not docked.
    pub fn remove(&mut self, panel: u64) -> bool {
        if !self.contains(panel) {
            return false;
        }
        if self.root.as_mut().is_some_and(|root| root.take(panel)) {
            self.root = None;
        }
        true
    }

    /// Show `panel` in its group. `false` when it is not docked.
    pub fn activate(&mut self, panel: u64) -> bool {
        match self.root.as_mut().and_then(|r| r.group_mut(panel)) {
            Some(DockNode::Tabs { active, .. }) => {
                *active = panel;
                true
            }
            _ => false,
        }
    }

    /// Set the share of the split whose two halves begin with the panels `key` (what a
    /// [`DockArea`](super::DockArea) reports when a divider moves).
    pub fn set_fraction(&mut self, key: (u64, u64), value: f32) -> bool {
        match self.root.as_mut().and_then(|root| root.split_mut(key)) {
            Some(fraction) => {
                *fraction = value.clamp(0.05, 0.95);
                true
            }
            None => false,
        }
    }
}
