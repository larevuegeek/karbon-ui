//! `content-visibility` : contenu sauté loin de la vue, comme dans Chromium.
//!
//! Un élément `content-visibility: auto` éloigné de la vue garde sa boîte (taille donnée
//! par `contain-intrinsic-size`, ou la dernière taille de son contenu avec `auto`) mais
//! son contenu n'est ni mesuré, ni placé, ni peint, ni touché, ni animé. Une liste de
//! morceaux chargée par lots de cent ne dispose ainsi que la trentaine de lignes proches
//! de la vue.
//!
//! La proximité se constate après le placement et après chaque défilement (comme
//! l'`IntersectionObserver` de Chromium) : la vue de chaque conteneur défilant, élargie
//! de la moitié de sa taille, et la fenêtre élargie de même. Un élément qui s'en
//! approche est disposé sur place quand sa taille ne dépend pas de son contenu
//! ([`arrange_near`]), sinon ses ancêtres oublient leur placement retenu et un layout
//! refait leur chemin ([`unsettle`]) ; un élément qui s'en éloigne cesse simplement
//! d'être peint, son contenu garde son dernier placement (repris tel quel s'il revient).
//! Un élément qui contient le focus reste disposé.
//!
//! `content-visibility: hidden` saute toujours le contenu. Dans les deux cas l'élément
//! rogne son contenu à sa boîte de padding (confinement de la peinture, voir
//! `node_clip`).
use super::*;
use crate::ui::core::hash::FastMap;
use std::cell::RefCell;

/// Marge de proximité : fraction de la taille de la vue ajoutée de chaque côté
/// (`kViewportMarginPercentage` de Chromium).
const MARGIN: f32 = 0.5;

/// État des éléments `content-visibility: auto` d'un arbre, d'un layout à l'autre.
#[derive(Default)]
pub(crate) struct Containment {
    /// Éléments proches de la vue au dernier constat : leur contenu est disposé.
    near: FastMap<ElementId, ()>,
    /// Taille du contenu à son dernier placement (`contain-intrinsic-size: auto`).
    remembered: FastMap<ElementId, SizeF>,
    /// L'arbre contient un élément `content-visibility: auto` : un défilement peut en
    /// rapprocher un de la vue.
    pub(crate) present: bool,
}

thread_local! {
    /// État prêté par le runtime le temps d'une passe de layout (`enter`, `leave`).
    /// Sans lui (layout hors runtime), aucun contenu `auto` n'est sauté.
    static STATE: RefCell<Option<Containment>> = const { RefCell::new(None) };
}

/// Prête l'état du runtime aux mesures et placements qui suivent.
pub(crate) fn enter(state: Containment) {
    STATE.with(|s| *s.borrow_mut() = Some(state));
}

/// L'état est prêté (passe de layout ou résolution des styles en cours).
pub(crate) fn entered() -> bool {
    STATE.with(|s| s.borrow().is_some())
}

/// Reprend l'état prêté par [`enter`].
pub(crate) fn leave() -> Containment {
    STATE.with(|s| s.borrow_mut().take()).unwrap_or_default()
}

fn visibility(node: &Element) -> ContentVisibility {
    node.rules
        .as_ref()
        .map_or(ContentVisibility::Visible, |r| r.content_visibility)
}

/// L'élément confine sa peinture : son contenu est rogné à sa boîte de padding.
pub(crate) fn paint_contained(rules: &LayoutRules) -> bool {
    rules.content_visibility != ContentVisibility::Visible
}

/// Le contenu de `node` est sauté par ce layout.
pub(crate) fn skipped(node: &Element) -> bool {
    match visibility(node) {
        ContentVisibility::Visible => false,
        ContentVisibility::Hidden => true,
        ContentVisibility::Auto => STATE.with(|s| {
            s.borrow()
                .as_ref()
                .is_some_and(|s| !s.near.contains_key(&node.id))
        }),
    }
}

/// Taille de la boîte de contenu d'un élément sauté : la dernière taille de son contenu
/// avec `auto`, sinon la longueur déclarée (zéro pour `none`).
pub(crate) fn placeholder(node: &Element) -> SizeF {
    let declared = node
        .rules
        .as_ref()
        .map_or([ContainIntrinsicLength::default(); 2], |r| {
            r.contain_intrinsic_size
        });
    let remembered = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .and_then(|s| s.remembered.get(&node.id).copied())
    });
    let axis = |i: usize, last: Option<f32>| {
        let d = declared[i];
        last.filter(|_| d.remembered)
            .or(d.length)
            .unwrap_or(0.)
            .max(0.)
    };
    SizeF {
        width: axis(0, remembered.map(|s| s.width)),
        height: axis(1, remembered.map(|s| s.height)),
    }
}

/// Retient la taille du contenu d'un élément `auto` disposé (boîte de contenu).
pub(super) fn remember(node: &Element, content: SizeF) {
    if visibility(node) != ContentVisibility::Auto {
        return;
    }
    STATE.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            s.remembered.insert(node.id, content);
        }
    });
}

/// Constate la proximité des éléments `auto` d'après leur placement actuel. Ceux qui
/// s'éloignent cessent d'être peints ; ceux qui s'approchent sont rendus, leur contenu
/// reste à disposer ([`arrange_near`], ou [`unsettle`] puis un layout). `window` : zone
/// de la fenêtre.
pub(crate) fn refresh(
    state: &mut Containment,
    root: &mut Element,
    window: RectF,
    focused: Option<ElementId>,
) -> Vec<ElementId> {
    let previous = std::mem::take(&mut state.near);
    let mut next = Walk {
        previous: &previous,
        near: FastMap::default(),
        live: FastMap::default(),
        approaching: Vec::new(),
        present: false,
        focused,
        found: false,
        deep: false,
        window: grow(window, window),
    };
    let area = next.window;
    next.visit(root, area);
    // Focus dans un contenu sauté (tabulation, focus programmé) : son élément `auto` se
    // rapproche ; le parcours descend alors aussi dans les contenus sautés.
    if focused.is_some() && !next.found {
        next.near.clear();
        next.live.clear();
        next.approaching.clear();
        next.deep = true;
        next.visit(root, area);
    }
    state.near = next.near;
    // Tailles retenues des seuls éléments encore dans l'arbre.
    state.remembered.retain(|id, _| next.live.contains_key(id));
    state.present = next.present;
    next.approaching
}

/// Les ancêtres de `ids` oublient leur placement retenu : le layout suivant refait leur
/// chemin. `true` si l'un d'eux est dans l'arbre.
pub(crate) fn unsettle(node: &mut Element, ids: &[ElementId]) -> bool {
    if ids.is_empty() {
        return false;
    }
    let mut found = ids.contains(&node.id);
    for child in &mut node.children {
        found |= unsettle(child, ids);
    }
    if found && let Some(cache) = node.layout_cache.as_deref_mut() {
        cache.settled = false;
    }
    found
}

/// Dispose sur place le contenu d'un élément qui s'approche de la vue, quand sa taille
/// n'en dépend pas : mesuré avec son contenu, il garde la taille de son dernier layout et
/// son contenu se place dans sa boîte sans rien déplacer autour. `false` : sa taille
/// change, un layout de son chemin est nécessaire. L'état doit être prêté (`enter`).
pub(crate) fn arrange_near(node: &mut Element, text: &mut dyn TextMeasurer) -> crate::Result<bool> {
    let Some(cache) = node.layout_cache.as_deref() else {
        return Ok(false);
    };
    let (Some(a), Some(available)) = (
        cache.arranged,
        cache.current.last().or(cache.last.last()).map(|m| m.3),
    ) else {
        return Ok(false);
    };
    // Taille du dernier layout (contenu sauté), puis avec le contenu.
    let near = |id, value: bool| {
        STATE.with(|s| {
            if let Some(s) = s.borrow_mut().as_mut() {
                if value {
                    s.near.insert(id, ());
                } else {
                    s.near.remove(&id);
                }
            }
        })
    };
    near(node.id, false);
    let before = super::measure(node, available, text);
    near(node.id, true);
    let after = super::measure(node, available, text)?;
    if before? != after {
        return Ok(false);
    }
    super::arrange_in(
        node,
        a.bounds,
        a.clip,
        text,
        LayoutContext {
            viewport: a.viewport,
            containing: a.containing,
            parent: a.parent,
        },
    )?;
    Ok(true)
}

struct Walk<'a> {
    previous: &'a FastMap<ElementId, ()>,
    near: FastMap<ElementId, ()>,
    live: FastMap<ElementId, ()>,
    approaching: Vec<ElementId>,
    present: bool,
    focused: Option<ElementId>,
    /// Le focus a été rencontré.
    found: bool,
    /// Descendre aussi dans les contenus sautés (focus introuvable sinon).
    deep: bool,
    window: RectF,
}

impl Walk<'_> {
    /// Renvoie `true` si le sous-arbre contient le focus.
    fn visit(&mut self, node: &mut Element, area: RectF) -> bool {
        if super::position::hidden(node) {
            return false;
        }
        // Un élément `fixed` se place dans la fenêtre, hors des défilements.
        let area = if fixed(node) { self.window } else { area };
        let inner = match node.scroll {
            Some(s) => intersect(area, grow(s.viewport, s.viewport)),
            None => area,
        };
        let mut focus = self.focused == Some(node.id);
        self.found |= focus;
        // Contenu sauté : rien à constater dessous (il revient avec son élément).
        if !node.content_skipped || self.deep {
            for child in &mut node.children {
                focus |= self.visit(child, inner);
            }
        }
        if visibility(node) == ContentVisibility::Auto {
            self.present = true;
            self.live.insert(node.id, ());
            let r = node.rect;
            let near = focus
                || (r.right >= area.left
                    && r.left <= area.right
                    && r.bottom >= area.top
                    && r.top <= area.bottom
                    && area.right > area.left
                    && area.bottom > area.top);
            if near {
                self.near.insert(node.id, ());
                if !self.previous.contains_key(&node.id) {
                    self.approaching.push(node.id);
                }
            } else {
                // Contenu gardé tel quel, simplement plus peint ni touché.
                node.content_skipped = true;
            }
        }
        focus
    }
}

/// `r` élargi de [`MARGIN`] fois la taille de `view` de chaque côté.
fn grow(r: RectF, view: RectF) -> RectF {
    let (x, y) = (width(view) * MARGIN, height(view) * MARGIN);
    RectF {
        left: r.left - x,
        top: r.top - y,
        right: r.right + x,
        bottom: r.bottom + y,
    }
}
