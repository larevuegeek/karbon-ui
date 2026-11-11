//! État d'un sous-arbre mémorisé ([`Element::memo`]) : clé de la vue, reprise du
//! sous-arbre de la vue précédente ; et cache de layout de chaque conteneur (mesures et
//! placement de l'image précédente).
use super::{Element, ElementId, Layout, SizeF};
use crate::ui::scene::RectF;

/// Une mesure de l'élément : contraintes (clé de `layout::memo`), taille rendue, taille
/// naturelle laissée sur l'élément et place offerte.
pub(crate) type MemoMeasure = (crate::ui::layout::MeasureKey, SizeF, SizeF, SizeF);

/// Entrées du dernier placement d'un conteneur.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct MemoArranged {
    pub bounds: RectF,
    pub clip: RectF,
    pub viewport: RectF,
    pub containing: RectF,
    pub parent: RectF,
}

/// Mesures et placement d'un conteneur à l'image précédente, repris tant que son
/// sous-arbre n'a pas changé (layout incrémental, comme les bits « sale » des
/// navigateurs) ; voir `layout/memo.rs` (`carried`) et `layout/lazy.rs`.
#[derive(Default)]
pub(crate) struct LayoutCache {
    /// Sous-arbre disposé et inchangé depuis : mesures et placement réutilisables. Une
    /// résolution des styles qui change une métrique dans le sous-arbre le remet à `false`.
    pub settled: bool,
    /// Passe de layout des mesures de `current`.
    pub pass: u64,
    /// Mesures de la passe précédente ; la dernière a laissé les descendants dans leur
    /// état actuel.
    pub last: Vec<MemoMeasure>,
    /// Mesures de la passe en cours.
    pub current: Vec<MemoMeasure>,
    /// Une mesure de la passe n'a pas été retrouvée : le sous-arbre a été mesuré de
    /// nouveau, les mesures suivantes de la passe le sont aussi (état des descendants).
    pub missed: bool,
    /// Une mesure de la passe a été reprise de la passe précédente.
    pub hit: bool,
    pub arranged: Option<MemoArranged>,
}

/// Ce que la vérification d'un sous-arbre mémorisé (`layout::validate`) retient de lui :
/// tout ce que ses ancêtres, qui peuvent changer d'une vue à l'autre, peuvent rendre
/// invalide. Le reste ne dépend que du sous-arbre, déjà vérifié.
#[derive(Clone, Copy, Default)]
pub(crate) struct CheckedSubtree {
    /// Nombre d'éléments, racine comprise.
    pub elements: u32,
    /// Profondeur du plus long chemin sous la racine.
    pub height: u32,
    /// Contient un élément `position: fixed`.
    pub fixed: bool,
    /// Contient un élément refusé sous une transformation animée.
    pub motion_limited: bool,
    /// Contient un champ de saisie.
    pub text_input: bool,
    /// Contient un groupe ou un bouton radio.
    pub radio: bool,
    /// Contient un dialogue, un popover ou une pile de couches.
    pub overlay: bool,
}

impl Element {
    /// Résumé de la dernière vérification, si cet élément est un sous-arbre mémorisé
    /// vérifié : les parcours qui ne cherchent qu'une sorte d'élément s'en servent pour
    /// ne pas y descendre.
    pub(crate) fn checked_subtree(&self) -> Option<CheckedSubtree> {
        self.memo.as_deref().and_then(|m| m.checked.get())
    }
}

pub(crate) struct MemoState {
    /// Empreinte de la clé donnée par la vue.
    pub key: u64,
    /// Élément vide posé par la vue : le sous-arbre de la vue précédente le remplace.
    pub placeholder: bool,
    /// Sous-arbre remis en place par la dernière reconstruction, pas encore restylé.
    pub transplanted: bool,
    /// Un élément du sous-arbre a changé d'état (pointeur, focus, animation) depuis la
    /// dernière résolution de ses styles.
    pub style_dirty: bool,
    /// Empreinte des entrées de la cascade à la racine du sous-arbre (héritage, position,
    /// faits des ancêtres) lors de sa dernière résolution, et portée des variables.
    pub style_inputs: Option<u64>,
    /// Typographie héritée lors de cette résolution, comparée telle quelle (la hacher
    /// passait par son texte `Debug`, ~4 µs par sous-arbre : 2 ms pour 500 lignes).
    pub style_inherited: Option<crate::ui::styling::Typography>,
    pub style_scope: Option<crate::ui::styling::CssVariables>,
    /// Résumé de la dernière vérification du sous-arbre.
    pub checked: std::cell::Cell<Option<CheckedSubtree>>,
    /// Le sous-arbre contient d'autres éléments mémorisés.
    pub nested: bool,
    /// Un dialogue ou un popover a quitté ce sous-arbre pour la racine de la vue
    /// (`layers::hoist`) : repris, le sous-arbre le perdrait. Il n'est jamais repris.
    pub hoisted: bool,
}
impl MemoState {
    fn new(key: u64, placeholder: bool) -> Self {
        Self {
            key,
            placeholder,
            transplanted: false,
            style_dirty: false,
            style_inputs: None,
            style_inherited: None,
            style_scope: None,
            checked: std::cell::Cell::new(None),
            nested: false,
            hoisted: false,
        }
    }
}

impl Element {
    /// Mark this element's subtree as depending only on `key` (a hash, see
    /// [`crate::ui::view::memo_key`]): when the next view gives the same key at the same
    /// identifier, [`crate::ui::view::lazy`] skips building the subtree and the runtime
    /// reuses the previous one with its styles, measures and placement.
    pub fn memo(mut self, key: u64) -> Self {
        fn contains_memo(node: &Element) -> bool {
            node.memo.is_some() || node.children.iter().any(contains_memo)
        }
        let mut state = Box::new(MemoState::new(key, false));
        state.nested = self.children.iter().any(contains_memo);
        self.memo = Some(state);
        self
    }
    /// Élément vide qui reprend le sous-arbre `id` de la vue précédente (clé `key`).
    pub(crate) fn memo_placeholder(id: ElementId, key: u64) -> Self {
        let mut node = Element::new(id.0, Layout::Column, Vec::new(), None);
        node.memo = Some(Box::new(MemoState::new(key, true)));
        node
    }
}
