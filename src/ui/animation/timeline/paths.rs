//! Branches animées de l'arbre : les éléments dont le sous-arbre contient un élément
//! animé (`Element::motion` hors sortie seule), et le parent de chaque élément. Relevées
//! quand l'arbre ou ses styles changent ; entre-temps (défilement, horloge d'une
//! animation), la synchronisation des pistes ne parcourt que ces branches au lieu de
//! tout l'arbre.
use crate::ui::core::hash::FastMap;
use crate::ui::element::{Element, ElementId};

pub(super) struct MotionPaths {
    /// Éléments animés et leurs ancêtres.
    leads: FastMap<ElementId, ()>,
    /// Parent de chaque élément (ancêtres du survol et de l'appui).
    parents: FastMap<ElementId, ElementId>,
}

impl MotionPaths {
    pub(super) fn new(root: &Element) -> Self {
        fn walk(node: &Element, paths: &mut MotionPaths) -> bool {
            let mut any = node.motion.as_deref().is_some_and(|m| !m.exit_only());
            for child in &node.children {
                paths.parents.insert(child.id, node.id);
                any |= walk(child, paths);
            }
            if any {
                paths.leads.insert(node.id, ());
            }
            any
        }
        let mut paths = Self {
            leads: FastMap::default(),
            parents: FastMap::default(),
        };
        walk(root, &mut paths);
        paths
    }

    /// `true` si la branche de `id` contient un élément animé.
    pub(super) fn leads(&self, id: ElementId) -> bool {
        self.leads.contains_key(&id)
    }

    /// `id` et ses ancêtres.
    pub(super) fn chain(&self, id: Option<ElementId>) -> Vec<ElementId> {
        let mut out = Vec::new();
        let mut next = id;
        while let Some(id) = next {
            out.push(id);
            next = self.parents.get(&id).copied();
        }
        out
    }
}
