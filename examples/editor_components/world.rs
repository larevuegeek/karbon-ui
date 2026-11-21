//! Les données de l'exemple : un niveau de 100 000 objets sur 8 niveaux d'imbrication,
//! leurs propriétés (faites à la demande d'après la clé), et l'historique d'annulation.
use karbon_ui::ui::editor::TreeSource;
use std::collections::HashMap;

/// Sortes d'objets, pour la liste de choix et la deuxième colonne de l'arbre.
pub const KINDS: [(u64, &str); 5] = [
    (0, "Maillage"),
    (1, "Lumière"),
    (2, "Caméra"),
    (3, "Groupe"),
    (4, "Décor"),
];

/// Les propriétés d'un objet.
#[derive(Clone, PartialEq)]
pub struct Props {
    pub name: String,
    pub visible: bool,
    pub kind: u64,
    pub position: [f64; 3],
    pub rotation: [f64; 3],
    pub scale: [f64; 3],
    pub intensity: f64,
}

impl Props {
    /// Propriétés de départ de l'objet `key`.
    fn initial(key: u64) -> Self {
        let f = key as f64;
        Self {
            name: format!("Objet {key:06}"),
            visible: !key.is_multiple_of(7),
            kind: key % 5,
            position: [(f * 0.37) % 50., 0., (f * 0.11) % 30.],
            rotation: [0., (f * 13.) % 360., 0.],
            scale: [1.; 3],
            intensity: 40.,
        }
    }
}

/// Le niveau : l'arbre des objets, les propriétés modifiées, l'historique.
pub struct Level {
    pub roots: Vec<u64>,
    pub children: HashMap<u64, Vec<u64>>,
    edited: HashMap<u64, Props>,
    /// Pour annuler : l'objet et ses propriétés avant chaque changement validé.
    history: Vec<(u64, Props)>,
    pub unsaved: bool,
}

impl TreeSource for Level {
    fn children(&self, parent: Option<u64>) -> &[u64] {
        match parent {
            None => &self.roots,
            Some(key) => self.children.get(&key).map_or(&[], Vec::as_slice),
        }
    }
}

impl Level {
    /// 8 niveaux : 4 racines, puis 4 enfants par nœud (87 380 objets), et un groupe
    /// « Décor » de 12 619 objets à plat : 100 000 objets.
    pub fn generate() -> Self {
        let mut next = 0u64;
        let mut fresh = |count: usize| -> Vec<u64> {
            (0..count)
                .map(|_| {
                    next += 1;
                    next
                })
                .collect()
        };
        let mut children = HashMap::new();
        let mut level = fresh(4);
        let mut roots = level.clone();
        for _ in 1..8 {
            let mut below = Vec::new();
            for &parent in &level {
                let kids = fresh(4);
                below.extend(&kids);
                children.insert(parent, kids);
            }
            level = below;
        }
        let scenery = fresh(1)[0];
        children.insert(scenery, fresh(12_619));
        roots.push(scenery);
        Self {
            roots,
            children,
            edited: HashMap::new(),
            history: Vec::new(),
            unsaved: false,
        }
    }

    /// Nombre d'objets.
    pub fn count(&self) -> usize {
        self.roots.len() + self.children.values().map(Vec::len).sum::<usize>()
    }

    /// Les propriétés de `key`.
    pub fn props(&self, key: u64) -> Props {
        self.edited
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Props::initial(key))
    }

    /// Change les propriétés de `key` ; `record` les garde dans l'historique (un
    /// changement validé, pas un aperçu pendant un glisser).
    pub fn edit(&mut self, key: u64, record: Option<Props>, change: impl FnOnce(&mut Props)) {
        let mut props = self.props(key);
        change(&mut props);
        if let Some(before) = record {
            self.history.push((key, before));
            self.unsaved = true;
        }
        self.edited.insert(key, props);
    }

    /// Annule le dernier changement validé ; rend l'objet touché.
    pub fn undo(&mut self) -> Option<u64> {
        let (key, before) = self.history.pop()?;
        self.edited.insert(key, before);
        self.unsaved = !self.history.is_empty();
        Some(key)
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Rattache `keys` à `parent` (glisser-déposer dans l'arbre).
    pub fn reparent(&mut self, keys: &[u64], parent: u64) {
        for list in std::iter::once(&mut self.roots).chain(self.children.values_mut()) {
            list.retain(|key| !keys.contains(key));
        }
        self.children.entry(parent).or_default().extend(keys);
        self.unsaved = true;
    }
}
