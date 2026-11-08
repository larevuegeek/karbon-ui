//! Arbre publié aux clients UI Automation : nœuds, identités et navigation, géométrie
//! physique.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum NodeId {
    Root,
    Element(u64),
    Caption(u8),
    SelectPopup(u64),
    SelectOption(u64, u64),
    Tooltip(u64),
}
impl NodeId {
    pub(super) fn automation_id(self) -> String {
        match self {
            Self::Root => "window".into(),
            Self::Element(id) => format!("element-{id}"),
            Self::Caption(id) => format!("caption-{id}"),
            Self::SelectPopup(owner) => format!("select-{owner}-popup"),
            Self::SelectOption(owner, option) => format!("select-{owner}-option-{option}"),
            Self::Tooltip(owner) => format!("tooltip-{owner}"),
        }
    }
}
#[derive(Clone)]
pub(super) struct Node {
    pub(super) key: NodeId,
    pub(super) parent: Option<NodeId>,
    pub(super) name: String,
    pub(super) help: String,
    pub(super) control_type: i32,
    pub(super) is_dialog: bool,
    pub(super) enabled: bool,
    pub(super) focusable: bool,
    pub(super) focused: bool,
    pub(super) offscreen: bool,
    pub(super) invokable: bool,
    pub(super) value: Option<String>,
    pub(super) text: Option<Arc<text::TextData>>,
    pub(super) choice: Option<ChoiceState>,
    pub(super) selection_container: Option<NodeId>,
    pub(super) range: Option<crate::ui::slider::RangeInfo>,
    pub(super) expanded: Option<bool>,
    /// `aria-current` : jeton exposé dans AriaProperties (`current=page`).
    pub(super) current: Option<&'static str>,
    pub(super) read_only: bool,
    pub(super) bounds: UiaRect,
    pub(super) clipped: UiaRect,
}
pub(super) struct Snapshot {
    pub(super) nodes: Vec<Arc<Node>>,
    pub(super) index: std::sync::OnceLock<std::collections::HashMap<NodeId, usize>>,
}
impl Snapshot {
    pub(super) fn new(nodes: Vec<Arc<Node>>) -> Self {
        Self {
            nodes,
            index: Default::default(),
        }
    }
}
impl Node {
    pub(super) fn selection_container(&self) -> bool {
        // Groupe (radios), liste déroulante, liste d'onglets.
        !self.is_dialog && matches!(self.control_type, 50026 | 50003 | 50018)
    }
}
impl Snapshot {
    pub(super) fn node(&self, key: NodeId) -> Result<Arc<Node>> {
        self.index()
            .get(&key)
            .map(|&i| Arc::clone(&self.nodes[i]))
            .ok_or_else(|| fail(UNAVAILABLE))
    }
    /// Position de chaque nœud, construite au premier accès : les clients interrogent les
    /// nœuds un par un, une recherche linéaire parcourait tout l'arbre à chaque requête.
    fn index(&self) -> &std::collections::HashMap<NodeId, usize> {
        self.index.get_or_init(|| {
            self.nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.key, i))
                .collect()
        })
    }
    pub(super) fn navigate(&self, key: NodeId, direction: i32) -> Result<Option<NodeId>> {
        let node = self.node(key)?;
        if direction == 0 {
            return Ok(node.parent);
        }
        if direction == 3 || direction == 4 {
            let mut children = self.nodes.iter().filter(|n| n.parent == Some(key));
            return Ok(if direction == 3 {
                children.next()
            } else {
                children.next_back()
            }
            .map(|n| n.key));
        }
        if direction != 1 && direction != 2 {
            return Err(fail(INVALID_ARGUMENT));
        }
        let siblings: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| n.parent == node.parent)
            .collect();
        let i = siblings
            .iter()
            .position(|n| n.key == key)
            .ok_or_else(|| fail(UNAVAILABLE))?;
        Ok(if direction == 1 {
            siblings.get(i + 1)
        } else {
            i.checked_sub(1).and_then(|i| siblings.get(i))
        }
        .map(|n| n.key))
    }
}
/// Rectangle logique du runtime converti en pixels physiques à l'écran.
pub(super) fn physical(rect: crate::ui::scene::RectF, origin: Point, scale: f32) -> UiaRect {
    UiaRect {
        left: origin.x as f64 + rect.left as f64 * scale as f64,
        top: origin.y as f64 + rect.top as f64 * scale as f64,
        width: (rect.right - rect.left).max(0.) as f64 * scale as f64,
        height: (rect.bottom - rect.top).max(0.) as f64 * scale as f64,
    }
}
