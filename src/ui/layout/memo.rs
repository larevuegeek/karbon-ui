//! Mémoïsation des mesures le temps d'une passe de layout du runtime.
//!
//! Un layout flex imbriqué mesure chaque élément plusieurs fois pour les mêmes
//! contraintes : la base flexible, puis la taille principale attribuée, à chaque niveau
//! de la récursion (≈ 46 000 appels de `measure` par image sur l'accueil de RustMusic,
//! pour ≈ 6 000 contraintes différentes). Une mesure retrouvée rend la taille calculée
//! et remet sur l'élément ce que son calcul y aurait écrit (taille naturelle, limites
//! relatives résolues).
//!
//! Le calcul d'une mesure écrit aussi dans les descendants (tailles naturelles, limites
//! relatives, hauteur des révélations), relus plus tard par des mesures qui ne les
//! recalculent pas d'abord : contraintes `min`/`max` relatives sauvées par les lignes
//! flex, taille naturelle des feuilles relue par la ligne de base, taille naturelle d'un
//! widget conteneur relue à la peinture. Un élément dont un descendant porte un tel état
//! n'est donc jamais mémoïsé : son calcul refait ces écritures dans le même ordre que
//! sans mémoire. Les lignes des listes virtuelles, construites pendant le layout, ne le
//! sont pas non plus.
use super::*;
use crate::ui::core::hash::FastMap;
use std::cell::RefCell;

/// Contraintes d'un appel de `measure` : place offerte et dimensions que le parent
/// modifie le temps d'une mesure (base flexible, taille attribuée, limites, marges).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Key {
    id: ElementId,
    numbers: [u32; 10],
    lengths: [(u8, u32, usize); 2],
}
fn length(l: Length) -> (u8, u32, usize) {
    match l {
        // L'expression est statique : son adresse l'identifie.
        Length::Computed(v) => (
            0,
            v.scale.to_bits(),
            std::ptr::from_ref(v.expression) as usize,
        ),
        Length::Auto => (1, 0, 0),
        Length::Px(n) => (2, n.to_bits(), 0),
        Length::Percent(n) => (3, n.to_bits(), 0),
        Length::Fill(n) => (4, n.to_bits(), 0),
    }
}
struct Memo {
    /// Éléments dont aucun descendant ne porte d'état relu hors de sa propre mesure.
    allowed: FastMap<ElementId, ()>,
    /// Taille rendue et taille naturelle laissée sur l'élément.
    sizes: FastMap<Key, (SizeF, SizeF)>,
}
thread_local! {
    static MEMO: RefCell<Option<Memo>> = const { RefCell::new(None) };
}
/// Active la mémoire pour une passe sur `root` (`None` : la retire).
pub(super) fn begin(root: Option<&Element>) {
    MEMO.with(|memo| {
        *memo.borrow_mut() = root.map(|root| {
            let mut allowed = FastMap::default();
            collect(root, &mut allowed);
            Memo {
                allowed,
                sizes: FastMap::default(),
            }
        });
    });
}
/// Range dans `allowed` les éléments mémoïsables du sous-arbre ; `true` si le sous-arbre
/// contient un élément dont l'état est relu hors de sa propre mesure.
fn collect(node: &Element, allowed: &mut FastMap<ElementId, ()>) -> bool {
    // Lignes construites pendant le layout (la liste n'en mesure aucune) : jamais
    // mémoïsées.
    if matches!(node.layout, Layout::VirtualList) {
        allowed.insert(node.id, ());
        return false;
    }
    let mut below = false;
    for child in &node.children {
        below |= collect(child, allowed);
    }
    below |= baseline_reads(node);
    if !below {
        allowed.insert(node.id, ());
    }
    below || sensitive(node)
}
/// État écrit par la mesure de `node` et relu sans la refaire.
fn sensitive(node: &Element) -> bool {
    node.rules
        .as_ref()
        .is_some_and(|r| r.reveal.is_some() || r.relative_limits.iter().any(Option::is_some))
        || (node.widget.is_some() && !matches!(node.layout, Layout::Leaf))
}
/// Une rangée alignée sur la ligne de base relit la taille naturelle de la première
/// feuille de ses enfants avant de les mesurer de nouveau (`flex::first_baseline`).
fn baseline_reads(node: &Element) -> bool {
    matches!(node.layout, Layout::Row)
        && (node.style.align == Align::Baseline
            || node.children.iter().any(|c| {
                c.rules
                    .as_ref()
                    .is_some_and(|r| r.align_self == Some(Align::Baseline))
            }))
}
/// Clé de l'appel, si `node` est mémoïsable pendant la passe en cours, et la taille
/// rendue et la taille naturelle d'un appel déjà calculé avec ces contraintes.
pub(super) fn lookup(node: &Element, available: SizeF) -> Option<(Key, Option<(SizeF, SizeF)>)> {
    let s = &node.style;
    let key = Key {
        id: node.id,
        numbers: [
            available.width.to_bits(),
            available.height.to_bits(),
            s.min_width.to_bits(),
            s.min_height.to_bits(),
            s.max_width.to_bits(),
            s.max_height.to_bits(),
            s.margin.top.to_bits(),
            s.margin.right.to_bits(),
            s.margin.bottom.to_bits(),
            s.margin.left.to_bits(),
        ],
        lengths: [length(s.width), length(s.height)],
    };
    MEMO.with(|memo| {
        let memo = memo.borrow();
        let memo = memo.as_ref().filter(|m| m.allowed.contains_key(&node.id))?;
        Some((key, memo.sizes.get(&key).copied()))
    })
}
pub(super) fn put(key: Key, size: SizeF, natural: SizeF) {
    MEMO.with(|memo| {
        if let Some(memo) = memo.borrow_mut().as_mut() {
            memo.sizes.insert(key, (size, natural));
        }
    });
}
