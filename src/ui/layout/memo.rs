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
//! recalculent pas d'abord : taille naturelle des feuilles relue par la ligne de base,
//! taille naturelle d'un widget conteneur relue à la peinture
//! (`Widget::paints_natural_size` : cases à cocher, interrupteurs, boutons radio),
//! hauteur des révélations. Les autres widgets conteneurs (boutons, liens, éléments
//! cliquables) ne relisent pas cette taille : sur une liste de morceaux, chacun coupait la
//! mémoire de sa ligne, de la liste et de la page (≈ 40 mesures par élément et par passe).
//! Les limites relatives (`max-width: 40%`) n'en font plus partie : la ligne flex les
//! résout contre la place de chaque mesure au lieu de relire celles d'un appel
//! précédent. Un élément dont un descendant porte un tel état
//! n'est donc jamais mémoïsé : son calcul refait ces écritures dans le même ordre que
//! sans mémoire. Les lignes des listes virtuelles, construites pendant le layout, ne le
//! sont pas non plus.
use super::*;
use crate::ui::core::hash::FastMap;
use std::cell::RefCell;

/// Contraintes d'un appel de `measure` : place offerte et dimensions que le parent
/// modifie le temps d'une mesure (base flexible, taille attribuée, limites, marges).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    id: ElementId,
    /// Contenu sauté (`content-visibility`) : mesuré sans ses enfants.
    skipped: bool,
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
        Length::MaxContent => (5, 0, 0),
    }
}
struct Memo {
    /// Taille rendue et taille naturelle laissée sur l'élément.
    sizes: FastMap<Key, (SizeF, SizeF)>,
}
thread_local! {
    static MEMO: RefCell<Option<Memo>> = const { RefCell::new(None) };
    /// Numéro de la passe en cours (mesures des sous-arbres mémorisés, `Element::memo`).
    static PASS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Les sous-arbres repris peuvent rendre les mesures de l'image précédente.
    static CARRY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// `Element::layout_bits` : aucun descendant ne porte d'état relu hors de sa propre
/// mesure (mesure mémoïsable pendant la passe).
pub(crate) const MEASURE_MEMO: u8 = 1;
/// `Element::layout_bits` : conteneur qui ne garde pas son layout d'une image à l'autre
/// (voir `flags`). Un élément créé pendant la passe n'a aucun indicateur.
pub(crate) const UNCACHED: u8 = 2;
/// Active la mémoire pour une passe sur `root` (`None` : la retire). Les indicateurs
/// sont posés sur les éléments (un parcours sans table) : la passe d'une image où
/// presque tout est repris reste légère.
pub(super) fn begin(root: Option<&mut Element>) {
    PASS.with(|p| p.set(p.get() + 1));
    CARRY.with(|c| c.set(root.is_some()));
    let active = root.map(|root| {
        flags(root);
        Memo {
            sizes: FastMap::default(),
        }
    });
    MEMO.with(|memo| *memo.borrow_mut() = active);
}
/// Pose `MEASURE_MEMO` et `UNCACHED` sur le sous-arbre ; renvoie (le sous-arbre contient
/// un état relu hors de sa propre mesure, ses ancêtres ne gardent pas leur layout).
fn flags(node: &mut Element) -> (bool, bool) {
    // Lignes construites pendant le layout (la liste n'en mesure aucune) : jamais
    // mémoïsées, et la liste ne garde pas son layout.
    if matches!(node.layout, Layout::VirtualList) {
        node.layout_bits = MEASURE_MEMO | UNCACHED;
        return (false, true);
    }
    let (mut sensitive_below, mut history_below) = (false, false);
    // Sous-arbre mémorisé repris et inchangé (tout changement de métrique retire `settled`) :
    // ses indicateurs de la passe précédente sont encore justes.
    if node.memo.is_some() && node.layout_cache.as_deref().is_some_and(|c| c.settled) {
        sensitive_below = node.layout_bits & MEASURE_MEMO == 0;
        history_below = node.layout_bits & UNCACHED != 0;
    } else {
        for child in &mut node.children {
            let (s, h) = flags(child);
            sensitive_below |= s;
            history_below |= h;
        }
        let baseline = baseline_reads(node);
        sensitive_below |= baseline;
        history_below |= baseline;
        node.layout_bits = if sensitive_below { 0 } else { MEASURE_MEMO }
            | if history_below { UNCACHED } else { 0 };
    }
    let relative = node.rules.as_ref().is_some_and(|r| r.reveal.is_some());
    (
        sensitive_below
            || relative
            || (node
                .widget
                .as_ref()
                .is_some_and(|w| w.paints_natural_size())
                && !matches!(node.layout, Layout::Leaf)),
        history_below || relative,
    )
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
    let key = key(node, available);
    MEMO.with(|memo| {
        let memo = memo.borrow();
        let memo = memo
            .as_ref()
            .filter(|_| node.layout_bits & MEASURE_MEMO != 0)?;
        Some((key, memo.sizes.get(&key).copied()))
    })
}
/// Le conteneur `node` garde ses mesures et son placement d'une image à l'autre : passe
/// avec mémoire, et un sous-arbre dont la mesure ne dépend que de ses contraintes, sans
/// liste virtuelle (voir `flags`). Un sous-arbre mémorisé (`Element::memo`) les garde
/// même sans cette garantie : l'auteur l'a demandé.
pub(super) fn cacheable(node: &Element) -> bool {
    !node.children.is_empty()
        && MEMO.with(|memo| {
            memo.borrow()
                .as_ref()
                .is_some_and(|_| node.layout_bits & UNCACHED == 0 || node.memo.is_some())
        })
}
/// Conteneur disposé et inchangé depuis l'image précédente : la taille rendue et la
/// taille naturelle de la même mesure à cette image (sa mesure ne dépend que de ses
/// contraintes, voir `cacheable`). Une mesure absente marque la passe : les suivantes
/// refont le calcul.
pub(super) fn carried(node: &mut Element, available: SizeF) -> Option<(SizeF, SizeF)> {
    let pass = PASS.with(std::cell::Cell::get);
    let carry = CARRY.with(std::cell::Cell::get);
    let key = key(node, available);
    let memo = node.layout_cache.as_deref_mut()?;
    if memo.pass != pass {
        memo.last = std::mem::take(&mut memo.current);
        memo.pass = pass;
        memo.missed = false;
        memo.hit = false;
    }
    if !carry || !memo.settled || memo.missed {
        return None;
    }
    let found = memo
        .last
        .iter()
        .find(|(k, ..)| *k == key)
        .map(|(_, size, natural, _)| (*size, *natural));
    let Some((size, natural)) = found else {
        memo.missed = true;
        return None;
    };
    memo.current.push((key, size, natural, available));
    memo.hit = true;
    Some((size, natural))
}
/// Une passe de layout avec mémoire est en cours (les sous-arbres mémorisés peuvent
/// reprendre mesures et placement).
pub(super) fn carrying() -> bool {
    CARRY.with(std::cell::Cell::get)
}
/// Retient une mesure calculée du conteneur ; ses descendants portent son état.
pub(super) fn record(node: &mut Element, available: SizeF, size: SizeF) {
    let pass = PASS.with(std::cell::Cell::get);
    let key = key(node, available);
    let natural = node.intrinsic;
    let memo = node.layout_cache.get_or_insert_with(Default::default);
    if memo.pass != pass {
        memo.last = std::mem::take(&mut memo.current);
        memo.pass = pass;
        memo.missed = false;
        memo.hit = false;
    }
    memo.current.retain(|(k, ..)| *k != key);
    memo.current.push((key, size, natural, available));
}
/// Avant de placer un conteneur dont des mesures ont été reprises : ses descendants
/// portent l'état laissé par la dernière mesure de la passe précédente. Si la dernière
/// mesure de cette passe en diffère, la place offerte à refaire, pour que le sous-arbre
/// soit dans l'état d'une passe sans mémoire (tailles naturelles relues à la peinture).
pub(super) fn stale_descendants(node: &mut Element) -> Option<SizeF> {
    let pass = PASS.with(std::cell::Cell::get);
    let memo = node.layout_cache.as_deref_mut()?;
    if memo.pass != pass || memo.missed || !memo.hit {
        return None;
    }
    let (key, .., available) = *memo.current.last()?;
    if memo.last.last().is_some_and(|(k, ..)| *k == key) {
        return None;
    }
    memo.missed = true;
    Some(available)
}
fn key(node: &Element, available: SizeF) -> Key {
    let s = &node.style;
    Key {
        id: node.id,
        skipped: super::containment::skipped(node),
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
    }
}
pub(super) fn put(key: Key, size: SizeF, natural: SizeF) {
    MEMO.with(|memo| {
        if let Some(memo) = memo.borrow_mut().as_mut() {
            memo.sizes.insert(key, (size, natural));
        }
    });
}
