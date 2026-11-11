//! Correspondance des sélecteurs : faits lus sur chaque élément et cadre de ses ancêtres
//! et frères.
use super::environment::StyleEnvironment;
use crate::ui::{
    element::{CheckState, ChoiceState, Element},
    layout,
    styling::{Container, Properties, StyleState},
};

/// Faits d'un élément lus par les sélecteurs : classes, états d'interaction et rang.
#[derive(Clone, Copy)]
pub(super) struct Facts<'a> {
    classes: &'a [&'static str],
    /// Balise et attributs écrits dans le template (feuilles de frameworks).
    tag: &'static str,
    attributes: &'a [(&'static str, std::borrow::Cow<'static, str>)],
    hot: bool,
    /// `pointer-events` déclaré par l'élément (`Some(false)` : `none`).
    pointer_events: Option<bool>,
    pub(super) contains_press: bool,
    pub(super) contains_focus: bool,
    focused: bool,
    enabled: bool,
    checked: bool,
    indeterminate: bool,
    read_only: bool,
    read_write: bool,
    placeholder: bool,
    empty: bool,
    index: usize,
    siblings: usize,
    /// Conteneur de requêtes déclaré et taille de sa boîte de contenu.
    container: Option<(Container, crate::ui::element::SizeF)>,
}
impl Facts<'_> {
    pub(super) fn with_classes<'b>(
        &self,
        classes: &'b [&'static str],
        attributes: &'b [(&'static str, std::borrow::Cow<'static, str>)],
    ) -> Facts<'b> {
        Facts {
            classes,
            tag: self.tag,
            attributes,
            hot: self.hot,
            pointer_events: self.pointer_events,
            contains_press: self.contains_press,
            contains_focus: self.contains_focus,
            focused: self.focused,
            enabled: self.enabled,
            checked: self.checked,
            indeterminate: self.indeterminate,
            read_only: self.read_only,
            read_write: self.read_write,
            placeholder: self.placeholder,
            empty: self.empty,
            index: self.index,
            siblings: self.siblings,
            container: self.container,
        }
    }
}
/// Cadre de correspondance : l'élément, la chaîne de ses ancêtres (descendant `A B`,
/// enfant `A > B`) et ses frères précédents (`A + B`, `A ~ B`), évalués à la demande.
pub(super) struct Frame<'a> {
    pub(super) facts: Facts<'a>,
    pub(super) parent: Option<&'a Frame<'a>>,
    pub(super) previous: &'a [Element],
    /// Frères suivants (`:last-of-type`).
    pub(super) following: &'a [Element],
    /// Pointeur rogné par un ancêtre (partagé avec les frères).
    pub(super) parent_clipped: bool,
    /// Règles de l'élément pour ses enfants (cadre transmis aux enfants seulement).
    pub(super) children: &'a [(StyleState, Properties, u32)],
    /// Rayons calculés de l'élément (cadre transmis aux enfants seulement) :
    /// `border-*-radius: inherit`.
    pub(super) corners: [f32; 4],
}
impl Frame<'_> {
    /// Empreinte de ce que la cascade lit de ce cadre et de ses ancêtres pour les
    /// descendants : faits de chaque élément (états, classes, conteneur), règles pour les
    /// enfants et rayons hérités (reprise des styles d'un sous-arbre mémorisé).
    /// Empreinte des faits du frère `sibling` (rang `index` parmi `siblings`) que lit une
    /// règle `[.a+&]` de l'élément suivant ; sans son rang sauf `positions`.
    pub(super) fn sibling_fingerprint(
        sibling: &Element,
        env: &StyleEnvironment,
        (index, siblings): (usize, usize),
        parent_clipped: bool,
        positions: bool,
        h: &mut impl std::hash::Hasher,
    ) {
        let frame = Frame {
            facts: facts(
                sibling,
                env,
                index,
                siblings,
                clipped(sibling, env, parent_clipped),
            ),
            parent: None,
            previous: &[],
            following: &[],
            parent_clipped,
            children: &[],
            corners: [0.; 4],
        };
        frame.fingerprint(h, positions);
    }
    pub(super) fn fingerprint(&self, h: &mut impl std::hash::Hasher, positions: bool) {
        use std::hash::Hash;
        let mut frame = Some(self);
        while let Some(f) = frame {
            let x = &f.facts;
            // Les classes et les règles pour les enfants sont des listes refaites à chaque
            // résolution : leur contenu (chaînes statiques), pas leur adresse.
            for class in x.classes {
                (class.as_ptr() as usize, class.len()).hash(h);
            }
            (x.classes.len(), x.tag).hash(h);
            for (name, value) in x.attributes {
                (name, value).hash(h);
            }
            (
                x.hot,
                x.pointer_events,
                x.contains_press,
                x.contains_focus,
                x.focused,
            )
                .hash(h);
            (
                x.enabled,
                x.checked,
                x.indeterminate,
                x.read_only,
                x.read_write,
            )
                .hash(h);
            (x.placeholder, x.empty).hash(h);
            // Rangs parmi les frères : seulement si une règle lue les compare.
            if positions {
                (x.index, x.siblings).hash(h);
            }
            if let Some((container, size)) = x.container {
                (
                    format!("{container:?}"),
                    size.width.to_bits(),
                    size.height.to_bits(),
                )
                    .hash(h);
            }
            for (state, _, rank) in f.children {
                (format!("{state:?}"), rank).hash(h);
            }
            (f.children.len(), f.parent_clipped).hash(h);
            f.corners.map(f32::to_bits).hash(h);
            frame = f.parent;
        }
    }
}
/// Le pointeur est hors du clip arrondi de `node` ou d'un ancêtre.
pub(super) fn clipped(node: &Element, env: &StyleEnvironment, parent: bool) -> bool {
    parent
        || env.pointer.is_some_and(|p| {
            env.project
                .map_or(Some(p), |f| f(node.id, p))
                .is_none_or(|p| {
                    layout::rounded_clip(node)
                        .is_some_and(|(r, radii)| !layout::contains_rounded(r, radii, p))
                })
        })
}
/// Le pointeur est sur `node` (rectangle projeté et découpe), hors de toute découpe
/// arrondie : fait `:hover` de l'élément.
fn hot(node: &Element, env: &StyleEnvironment, pointer_clipped: bool) -> bool {
    // Chaîne relevée par le hit-test : l'élément peint au-dessus et ses ancêtres, comme
    // un navigateur (rien n'est survolé à travers un élément qui le recouvre).
    if let Some(chain) = env.hovered {
        return env.pointer.is_some() && chain.contains(&node.id);
    }
    !pointer_clipped
        && env.pointer.is_some_and(|p| {
            env.project
                .map_or(Some(p), |f| f(node.id, p))
                .is_some_and(|local| layout::contains(node.rect, local.x, local.y))
                && layout::contains(node.clip, p.x, p.y)
        })
}
/// Faits que la position du pointeur donne à chaque élément de l'arbre (pointeur rogné,
/// survol), deux bits par élément dans l'ordre du parcours de la résolution, avec les
/// identités parcourues. La résolution ne lit le pointeur qu'à travers eux : deux
/// positions qui donnent les mêmes faits au même arbre donnent les mêmes styles.
#[derive(PartialEq)]
pub(crate) struct PointerFacts {
    ids: Vec<crate::ui::element::ElementId>,
    bits: Vec<u64>,
}
impl PointerFacts {
    /// Éléments présents dans les deux relevés dont les faits diffèrent (arbres
    /// différents : un sous-arbre repris garde ses identités).
    pub(crate) fn changed_by_id(&self, previous: &Self) -> Vec<crate::ui::element::ElementId> {
        let bits = |facts: &Self, i: usize| (facts.bits[i * 2 / 64] >> (i * 2 % 64)) & 3;
        let before: crate::ui::core::hash::FastMap<_, _> = previous
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, bits(previous, i)))
            .collect();
        (0..self.ids.len())
            .filter(|&i| {
                before
                    .get(&self.ids[i])
                    .is_some_and(|b| *b != bits(self, i))
            })
            .map(|i| self.ids[i])
            .collect()
    }
    /// Éléments dont les faits diffèrent de `previous` (même arbre), `None` si l'arbre a
    /// changé.
    pub(crate) fn changed(&self, previous: &Self) -> Option<Vec<crate::ui::element::ElementId>> {
        if self.ids != previous.ids {
            return None;
        }
        let bits = |facts: &Self, i: usize| (facts.bits[i * 2 / 64] >> (i * 2 % 64)) & 3;
        Some(
            (0..self.ids.len())
                .filter(|&i| bits(self, i) != bits(previous, i))
                .map(|i| self.ids[i])
                .collect(),
        )
    }
}
pub(crate) fn pointer_facts(root: &Element, env: &StyleEnvironment) -> PointerFacts {
    fn visit(node: &Element, env: &StyleEnvironment, parent: bool, out: &mut PointerFacts) {
        let pointer_clipped = clipped(node, env, parent);
        let hot = hot(node, env, pointer_clipped);
        let index = out.ids.len() * 2;
        if index.is_multiple_of(64) {
            out.bits.push(0);
        }
        *out.bits.last_mut().unwrap() |=
            (u64::from(pointer_clipped) | u64::from(hot) << 1) << (index % 64);
        out.ids.push(node.id);
        if node.content_skipped {
            // Contenu sauté (`content-visibility`) : ni touché ni restylé, ses positions
            // ne suivent plus le défilement. Faits nuls, sans calcul ; les identités
            // restent pour que deux relevés gardent la même forme.
            for child in &node.children {
                skipped(child, out);
            }
            return;
        }
        for child in &node.children {
            visit(child, env, pointer_clipped && !layout::fixed(child), out);
        }
    }
    fn skipped(node: &Element, out: &mut PointerFacts) {
        if (out.ids.len() * 2).is_multiple_of(64) {
            out.bits.push(0);
        }
        out.ids.push(node.id);
        for child in &node.children {
            skipped(child, out);
        }
    }
    let mut out = PointerFacts {
        ids: Vec::new(),
        bits: Vec::new(),
    };
    visit(root, env, false, &mut out);
    out
}
pub(super) fn facts<'a>(
    node: &'a Element,
    env: &StyleEnvironment,
    index: usize,
    siblings: usize,
    pointer_clipped: bool,
) -> Facts<'a> {
    let widget = node.widget.as_ref();
    let choice = widget.and_then(|w| w.choice());
    let read_only = widget.is_some_and(|w| {
        w.text_input().is_some_and(|i| i.read_only) || w.slider().is_some_and(|s| s.read_only)
    });
    Facts {
        classes: node.styles.as_ref().map_or(&[], |s| &s.classes),
        tag: node.styles.as_ref().map_or("", |s| s.tag),
        attributes: node.styles.as_ref().map_or(&[], |s| &s.attributes),
        pointer_events: node.styles.as_ref().and_then(|s| s.pointer_events),
        hot: hot(node, env, pointer_clipped),
        contains_press: env.pressed.is_some_and(|id| node.find(id).is_some()),
        contains_focus: env.focused.is_some_and(|id| node.find(id).is_some()),
        focused: env.focused == Some(node.id),
        enabled: widget.is_none_or(|w| w.semantics().enabled),
        checked: choice.is_some_and(|c| {
            matches!(
                c,
                ChoiceState::Toggle(CheckState::On) | ChoiceState::Radio(true)
            )
        }),
        indeterminate: matches!(choice, Some(ChoiceState::Toggle(CheckState::Mixed))),
        read_only,
        read_write: !read_only && widget.is_some_and(|w| w.text_input().is_some()),
        placeholder: widget.and_then(|w| w.text_input()).is_some_and(|i| {
            env.input_empty
                .and_then(|f| f(node.id))
                .unwrap_or_else(|| i.value.is_empty())
                && !i.placeholder.is_empty()
        }),
        empty: node.children.is_empty() && widget.is_none_or(|w| w.semantics().name.is_empty()),
        index,
        siblings,
        container: node.styles.as_ref().and_then(|s| {
            let kind = s.container_type?;
            let container = Container {
                kind,
                names: s.container_names,
            };
            Some((container, s.container_seen?))
        }),
    }
}
/// `pointer-events` effectif (hérité) de l'élément du cadre.
fn pointer_events(c: &Frame) -> bool {
    let mut frame = Some(c);
    while let Some(f) = frame {
        if let Some(enabled) = f.facts.pointer_events {
            return enabled;
        }
        frame = f.parent;
    }
    true
}
pub(super) fn matches(state: StyleState, c: &Frame, env: &StyleEnvironment) -> bool {
    let f = &c.facts;
    match state {
        StyleState::Any(states) => states.iter().any(|s| matches(*s, c, env)),
        StyleState::Media(query) => {
            query.matches(env.viewport)
                && query.reduced_motion.is_none_or(|r| r == env.reduced_motion)
                && query.dark.is_none_or(|d| d == env.dark)
                && query.high_contrast.is_none_or(|h| h == env.high_contrast)
        }
        StyleState::Container(query) => {
            // Le conteneur interrogé est le plus proche ancêtre accepté (jamais l'élément).
            let mut ancestor = c.parent;
            while let Some(frame) = ancestor {
                if let Some((container, size)) = frame.facts.container
                    && query.accepts(container)
                {
                    return query.matches(size);
                }
                ancestor = frame.parent;
            }
            false
        }
        StyleState::All(states) => states.iter().all(|s| matches(*s, c, env)),
        StyleState::Not(state) => !matches(*state, c, env),
        StyleState::Where(state) => matches(*state, c, env),
        StyleState::Class(name) => f.classes.contains(&name),
        StyleState::Ancestor(state) => {
            let mut ancestor = c.parent;
            while let Some(frame) = ancestor {
                if matches(*state, frame, env) {
                    return true;
                }
                ancestor = frame.parent;
            }
            false
        }
        StyleState::Parent(state) => c.parent.is_some_and(|p| matches(*state, p, env)),
        StyleState::PreviousSibling(inner) | StyleState::AnyPreviousSibling(inner) => {
            let any = matches!(state, StyleState::AnyPreviousSibling(_));
            let count = c.previous.len();
            let first = if any { 0 } else { count.saturating_sub(1) };
            (first..count).rev().any(|i| {
                let sibling = &c.previous[i];
                let frame = Frame {
                    facts: facts(
                        sibling,
                        env,
                        i,
                        f.siblings,
                        clipped(sibling, env, c.parent_clipped),
                    ),
                    parent: c.parent,
                    previous: &c.previous[..i],
                    following: &[],
                    parent_clipped: c.parent_clipped,
                    children: &[],
                    corners: [0.; 4],
                };
                matches(*inner, &frame, env)
            })
        }
        StyleState::FirstChild => f.index == 0,
        StyleState::LastChild => f.index + 1 == f.siblings,
        StyleState::OnlyChild => f.siblings == 1,
        StyleState::Empty => f.empty,
        StyleState::NthChild(a, b) => {
            let n = f.index as i64 + 1 - b as i64;
            if a == 0 {
                n == 0
            } else {
                n % (a as i64) == 0 && n / (a as i64) >= 0
            }
        }
        // Avec la chaîne du hit-test, un ancêtre `pointer-events: none` d'un élément
        // touché est survolé, comme dans un navigateur.
        StyleState::Hover => f.hot && (env.hovered.is_some() || pointer_events(c)),
        StyleState::Active => f.contains_press,
        StyleState::Focus => f.focused,
        StyleState::FocusVisible => env.focus_visible && f.focused,
        StyleState::FocusWithin => f.contains_focus,
        StyleState::Disabled => !f.enabled,
        StyleState::Enabled => f.enabled,
        StyleState::Checked => f.checked,
        StyleState::Indeterminate => f.indeterminate,
        StyleState::ReadOnly => f.read_only,
        StyleState::ReadWrite => f.read_write,
        StyleState::PlaceholderShown => f.placeholder,
        StyleState::Tag(tag) => f.tag == tag,
        StyleState::Attribute(a) => f
            .attributes
            .iter()
            .any(|(name, value)| *name == a.name && a.matches(value)),
        StyleState::Root => {
            // Premier élément balisé de la chaîne des ancêtres.
            if f.tag.is_empty() {
                return false;
            }
            let mut ancestor = c.parent;
            while let Some(frame) = ancestor {
                if !frame.facts.tag.is_empty() {
                    return false;
                }
                ancestor = frame.parent;
            }
            true
        }
        StyleState::FirstOfType => !same_tag(f.tag, c.previous),
        StyleState::LastOfType => !same_tag(f.tag, c.following),
        StyleState::OnlyOfType => !same_tag(f.tag, c.previous) && !same_tag(f.tag, c.following),
        StyleState::NthLastChild(a, b) => nth(a, b, f.siblings - f.index),
        StyleState::NthOfType(a, b) => nth(a, b, of_type(f.tag, c.previous) + 1),
        StyleState::NthLastOfType(a, b) => nth(a, b, of_type(f.tag, c.following) + 1),
    }
}
/// Rang `n` (1-based) satisfait `an+b`.
fn nth(a: i32, b: i32, n: usize) -> bool {
    let n = n as i64 - b as i64;
    if a == 0 {
        n == 0
    } else {
        n % (a as i64) == 0 && n / (a as i64) >= 0
    }
}
fn tag_of(e: &Element) -> &'static str {
    e.styles.as_ref().map_or("", |s| s.tag)
}
fn same_tag(tag: &str, siblings: &[Element]) -> bool {
    !tag.is_empty() && siblings.iter().any(|e| tag_of(e) == tag)
}
fn of_type(tag: &str, siblings: &[Element]) -> usize {
    siblings.iter().filter(|e| tag_of(e) == tag).count()
}
