//! Les lignes visibles d'un arbre : ses nœuds, aplatis dans l'ordre d'affichage, en ne
//! descendant que dans les branches dépliées.
use std::collections::HashSet;

/// Branches nested deeper than this are not shown: a source that is not a tree (a node
/// among its own descendants) cannot make the flattening loop forever.
const MAX_DEPTH: u16 = 1024;

/// The data behind a [`TreeView`](super::TreeView): the children of each node, by stable
/// key. The application keeps its objects; the tree only reads their keys.
///
/// ```
/// use karbon_ui::ui::editor::TreeSource;
/// use std::collections::HashMap;
///
/// struct Scene {
///     roots: Vec<u64>,
///     children: HashMap<u64, Vec<u64>>,
/// }
/// impl TreeSource for Scene {
///     fn children(&self, parent: Option<u64>) -> &[u64] {
///         match parent {
///             None => &self.roots,
///             Some(key) => self.children.get(&key).map_or(&[], Vec::as_slice),
///         }
///     }
/// }
/// ```
pub trait TreeSource {
    /// Children of `parent` in display order; the top-level nodes for `None`. A key must
    /// appear once in the whole tree.
    fn children(&self, parent: Option<u64>) -> &[u64];
}

/// A visible row of a tree: a node whose ancestors are all expanded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeRow {
    /// Key of the node.
    pub key: u64,
    /// Nesting level, `0` for a top-level node.
    pub depth: u16,
    /// The node has children (its row shows an arrow).
    pub has_children: bool,
    /// Its children are shown below it.
    pub expanded: bool,
}

/// Les lignes visibles de `source`, dans l'ordre d'affichage : un parcours en profondeur
/// qui ne descend que dans les nœuds de `expanded`. Le coût suit le nombre de lignes
/// visibles, pas la taille de l'arbre.
pub(super) fn flatten(source: &impl TreeSource, expanded: &HashSet<u64>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    // Pile des fratries en cours : (frères, prochain indice, profondeur).
    let mut stack: Vec<(&[u64], usize, u16)> = vec![(source.children(None), 0, 0)];
    while let Some(top) = stack.last_mut() {
        let (siblings, next, depth) = (top.0, top.1, top.2);
        let Some(&key) = siblings.get(next) else {
            stack.pop();
            continue;
        };
        top.1 += 1;
        let children = source.children(Some(key));
        let open = !children.is_empty() && expanded.contains(&key) && depth < MAX_DEPTH;
        rows.push(TreeRow {
            key,
            depth,
            has_children: !children.is_empty(),
            expanded: open,
        });
        if open {
            stack.push((children, 0, depth + 1));
        }
    }
    rows
}

/// Indice de la ligne du parent de la ligne `index` : la première ligne au-dessus, moins
/// profonde. `None` pour un nœud de premier niveau.
pub(super) fn parent_index(rows: &[TreeRow], index: usize) -> Option<usize> {
    let depth = rows.get(index)?.depth;
    rows[..index].iter().rposition(|row| row.depth < depth)
}

/// Fin (exclue) du sous-arbre visible de la ligne `index` : les lignes qui la suivent et
/// sont plus profondes qu'elle.
pub(super) fn subtree_end(rows: &[TreeRow], index: usize) -> usize {
    let depth = rows[index].depth;
    rows[index + 1..]
        .iter()
        .position(|row| row.depth <= depth)
        .map_or(rows.len(), |offset| index + 1 + offset)
}
