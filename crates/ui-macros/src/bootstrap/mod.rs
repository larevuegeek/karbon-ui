//! Comportements Bootstrap déclaratifs (`data-bs-toggle`, `data-bs-dismiss`) réécrits
//! en composants natifs avant la génération d'un template.
//!
//! - `collapse` : le déclencheur émet `BootstrapAction::Toggle` ; la cible reçoit la
//!   classe `show` (et le déclencheur `collapsed`) selon l'état ; `data-bs-parent`
//!   forme un accordéon. La cible porte `data-bs-reveal="collapsing"` : sa hauteur
//!   s'anime entre 0 et sa hauteur naturelle au minutage de `.collapsing`
//!   (`Element::reveal`).
//! - `tab` / `pill` / `list` : `Show` ; le déclencheur et le volet reçoivent `active`
//!   (et `show` pour le volet), les autres volets du même `.nav` les perdent.
//! - `dropdown` : le `.dropdown-menu` qui suit le déclencheur s'ouvre dans un
//!   `<k-popover menu>` ancré au déclencheur (fermeture au clic extérieur, Échap).
//! - `modal` : la `.modal` devient un `<dialog>` natif (focus piégé, Échap, voile).
//! - `offcanvas` : panneau `.offcanvas.show` et voile `.offcanvas-backdrop` qui ferme.
//! - `tooltip` / `popover` : `data-bs-title` devient l'info-bulle native (`title`).
//! - `data-bs-dismiss="alert" | "modal" | "offcanvas"` : ferme l'ancêtre concerné.
mod query;
mod rewriter;
use crate::template::markup::Node;
use proc_macro::TokenStream;
use query::{has_class, key, target, text};
use rewriter::Rewriter;
use std::collections::BTreeMap;

/// Le balisage utilise-t-il des attributs `data-bs-*` de comportement ?
fn uses_behaviours(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| match n {
        Node::Element(e) => {
            e.attrs
                .iter()
                .any(|a| a.name == "data-bs-toggle" || a.name == "data-bs-dismiss")
                || has_class(e, "form-check-input")
                || uses_behaviours(&e.children)
        }
        Node::If {
            branches,
            otherwise,
            ..
        } => {
            branches.iter().any(|(_, b)| uses_behaviours(b))
                || otherwise.as_deref().is_some_and(uses_behaviours)
        }
        Node::Each(each) => uses_behaviours(&each.body),
        Node::Fragment { nodes, .. } => uses_behaviours(nodes),
        _ => false,
    })
}
/// Rôle d'une cible et état par défaut.
#[derive(Default)]
struct Plan {
    /// Cibles de `collapse` → groupe (`data-bs-parent`).
    collapses: BTreeMap<String, Option<String>>,
    /// Groupes d'accordéon → cibles.
    accordions: BTreeMap<String, Vec<String>>,
    /// Volets de `tab` → clé du groupe (le `.nav` des déclencheurs).
    panes: BTreeMap<String, String>,
    /// Groupes d'onglets → volets.
    tab_sets: BTreeMap<String, Vec<String>>,
    modals: Vec<String>,
    offcanvas: Vec<String>,
}
fn collect(nodes: &[Node], navs: &mut Vec<String>, plan: &mut Plan) {
    for n in nodes {
        match n {
            Node::Element(e) => {
                let is_nav = has_class(e, "nav")
                    || has_class(e, "list-group")
                    || text(e, "role").as_deref() == Some("tablist");
                if is_nav {
                    navs.push(key(&e.loc));
                }
                match text(e, "data-bs-toggle").as_deref() {
                    Some("collapse") => {
                        if let Some(t) = target(e) {
                            plan.collapses.entry(t).or_insert(None);
                        }
                    }
                    Some("tab" | "pill" | "list") => {
                        if let Some(t) = target(e) {
                            let group = navs.last().cloned().unwrap_or_else(|| key(&e.loc));
                            plan.panes.insert(t.clone(), group.clone());
                            plan.tab_sets.entry(group).or_default().push(t);
                        }
                    }
                    Some("modal") => {
                        if let Some(t) = target(e) {
                            plan.modals.push(t);
                        }
                    }
                    Some("offcanvas") => {
                        if let Some(t) = target(e) {
                            plan.offcanvas.push(t);
                        }
                    }
                    _ => {}
                }
                if let (Some(id), Some(parent)) = (text(e, "id"), text(e, "data-bs-parent")) {
                    let group = parent.trim_start_matches('#').to_owned();
                    plan.collapses.insert(id.clone(), Some(group.clone()));
                    plan.accordions.entry(group).or_default().push(id);
                }
                collect(&e.children, navs, plan);
                if is_nav {
                    navs.pop();
                }
            }
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                for (_, b) in branches {
                    collect(b, navs, plan);
                }
                if let Some(o) = otherwise {
                    collect(o, navs, plan);
                }
            }
            Node::Each(each) => collect(&each.body, navs, plan),
            Node::Fragment { nodes, .. } => collect(nodes, navs, plan),
            _ => {}
        }
    }
}
/// Classes `show` / `active` des cibles écrites dans le template.
fn defaults(nodes: &[Node], out: &mut BTreeMap<String, bool>) {
    for n in nodes {
        match n {
            Node::Element(e) => {
                if let Some(id) = text(e, "id") {
                    out.insert(id, has_class(e, "show") || has_class(e, "active"));
                }
                defaults(&e.children, out);
            }
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                for (_, b) in branches {
                    defaults(b, out);
                }
                if let Some(o) = otherwise {
                    defaults(o, out);
                }
            }
            Node::Each(each) => defaults(&each.body, out),
            Node::Fragment { nodes, .. } => defaults(nodes, out),
            _ => {}
        }
    }
}
/// Réécrit les comportements Bootstrap de `nodes` ; `root` est le chemin de la crate
/// `karbon_ui`.
pub fn rewrite(nodes: Vec<Node>, root: TokenStream) -> Vec<Node> {
    if !uses_behaviours(&nodes) {
        return nodes;
    }
    let mut plan = Plan::default();
    collect(&nodes, &mut Vec::new(), &mut plan);
    let mut known = BTreeMap::new();
    defaults(&nodes, &mut known);
    let mut rewriter = Rewriter {
        root,
        plan,
        dismissible: Vec::new(),
        defaults: known,
        themes: Vec::new(),
    };
    rewriter.nodes(nodes)
}
