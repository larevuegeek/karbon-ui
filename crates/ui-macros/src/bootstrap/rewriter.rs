//! Réécriture des éléments : déclencheurs reliés à leurs cibles par des
//! `BootstrapAction`, classes d'état calculées à l'exécution, menus, modales et
//! panneaux remplacés par les composants natifs.
use super::Plan;
use super::query::{
    contains_dismiss, find_class, find_class_text, has_class, key, plain, target, text,
};
use crate::template::markup::{Attr, Element, Loc, Node, Value};
use proc_macro::TokenStream;
use std::collections::BTreeMap;
/// Réécriture d'un template.
pub(super) struct Rewriter {
    pub(super) root: TokenStream,
    pub(super) plan: Plan,
    /// Ancêtres fermables (alerte, modale, offcanvas) : clé.
    pub(super) dismissible: Vec<(String, String)>,
    /// État par défaut des cibles (`show` / `active` écrits dans le template).
    pub(super) defaults: BTreeMap<String, bool>,
    /// `data-bs-theme` des ancêtres : recopié sur les couches remontées hors de la page
    /// (menu, dialogue), qui ne sont plus leurs descendants.
    pub(super) themes: Vec<Value>,
}
impl Rewriter {
    fn expr(&self, code: &str) -> TokenStream {
        let mut tokens = self.root.clone();
        tokens.extend(code.parse::<TokenStream>().unwrap());
        tokens
    }
    fn shown(&self, target: &str, default: bool) -> TokenStream {
        self.expr(&format!("::ui::bootstrap::shown({target:?},{default})"))
    }
    fn not_shown(&self, target: &str, default: bool) -> TokenStream {
        let mut tokens: TokenStream = "!".parse().unwrap();
        tokens.extend(self.shown(target, default));
        tokens
    }
    fn list(items: &[String]) -> String {
        format!(
            "&[{}]",
            items
                .iter()
                .map(|i| format!("{i:?}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
    fn toggle(&self, target: &str, default: bool, group: &[String]) -> TokenStream {
        self.expr(&format!(
            "::ui::bootstrap::BootstrapAction::Toggle{{target:{target:?},default:{default},group:{}}}",
            Self::list(group)
        ))
    }
    fn hide(&self, target: &str) -> TokenStream {
        self.expr(&format!(
            "::ui::bootstrap::BootstrapAction::Hide({target:?})"
        ))
    }
    fn attr(name: &str, value: Value, loc: Loc) -> Attr {
        Attr {
            name: name.into(),
            value,
            loc,
        }
    }
    fn set(e: &mut Element, name: &str, value: Value) {
        let loc = e.loc;
        e.attrs.retain(|a| a.name != name);
        e.attrs.push(Self::attr(name, value, loc));
    }
    /// Retire `class` des classes littérales (posée ensuite selon l'état).
    fn remove_class(e: &mut Element, class: &str) -> bool {
        let mut removed = false;
        for a in &mut e.attrs {
            if a.name == "class"
                && let Value::Text(t) = &a.value
            {
                let kept = t
                    .split_whitespace()
                    .filter(|c| {
                        let same = *c == class;
                        removed |= same;
                        !same
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                a.value = Value::Text(kept);
            }
        }
        removed
    }
    fn add_class(e: &mut Element, class: &str) {
        match e.attrs.iter_mut().find(|a| a.name == "class") {
            Some(a) => {
                if let Value::Text(t) = &mut a.value {
                    t.push(' ');
                    t.push_str(class);
                }
            }
            None => {
                let loc = e.loc;
                e.attrs
                    .push(Self::attr("class", Value::Text(class.into()), loc));
            }
        }
    }
    pub(super) fn nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        let mut out = Vec::with_capacity(nodes.len());
        // Déclencheur de menu déroulant précédent (clé, identifiant, libellé).
        let mut dropdown: Option<(String, String, String)> = None;
        for node in nodes {
            match node {
                Node::Element(mut e) => {
                    if has_class(&e, "dropdown-menu")
                        && let Some((menu, anchor, label)) = dropdown.take()
                    {
                        out.push(self.dropdown_menu(e, &menu, &anchor, &label));
                        continue;
                    }
                    if text(&e, "data-bs-toggle").as_deref() == Some("dropdown") {
                        let id = text(&e, "id").unwrap_or_else(|| key(&e.loc));
                        Self::set(&mut e, "id", Value::Text(id.clone()));
                        let menu = format!("{id}-menu");
                        let label = plain(&e.children).unwrap_or_else(|| "Menu".into());
                        let toggle = self.toggle(&menu, false, &[]);
                        Self::set(&mut e, "onclick", Value::Expr(toggle));
                        Self::set(
                            &mut e,
                            "aria-expanded",
                            Value::Expr(self.shown(&menu, false)),
                        );
                        dropdown = Some((menu, id, label));
                        out.push(self.element(e));
                        continue;
                    }
                    out.extend(self.element_nodes(e));
                }
                Node::If {
                    branches,
                    otherwise,
                    loc,
                } => out.push(Node::If {
                    branches: branches
                        .into_iter()
                        .map(|(c, b)| (c, self.nodes(b)))
                        .collect(),
                    otherwise: otherwise.map(|o| self.nodes(o)),
                    loc,
                }),
                Node::Each(mut each) => {
                    each.body = self.nodes(std::mem::take(&mut each.body));
                    out.push(Node::Each(each));
                }
                Node::Fragment { nodes, slot } => out.push(Node::Fragment {
                    nodes: self.nodes(nodes),
                    slot,
                }),
                other => out.push(other),
            }
        }
        out
    }
    /// Élément pouvant devenir plusieurs nœuds (cible conditionnelle).
    fn element_nodes(&mut self, mut e: Element) -> Vec<Node> {
        let id = text(&e, "id");
        let loc = e.loc;
        // Modale : dialogue natif.
        if let Some(id) = &id
            && has_class(&e, "modal")
            && self.plan.modals.contains(id)
        {
            let label = find_class_text(&e.children, "modal-title")
                .or_else(|| text(&e, "aria-label"))
                .unwrap_or_else(|| "Dialog".into());
            let default = Self::remove_class(&mut e, "show");
            // `.modal-dialog` en haut (`margin: 1.75rem auto`) sauf `.modal-dialog-centered`.
            let placement = if find_class(&e.children, "modal-dialog-centered") {
                "center"
            } else {
                "top"
            };
            self.dismissible.push(("modal".into(), id.clone()));
            let children = self.nodes(std::mem::take(&mut e.children));
            self.dismissible.pop();
            let dialog = Element {
                name: "dialog".into(),
                path: TokenStream::new(),
                attrs: vec![
                    Self::attr("aria-label", Value::Text(label), loc),
                    // Variables `--bs-modal-*` de `.modal`, sans sa boîte fixe plein écran :
                    // le dialogue natif centre et voile.
                    Self::attr("class", Value::Text("modal show text-body".into()), loc),
                    Self::attr(
                        "style",
                        Value::Text(
                            "position:static;display:block;width:500px;height:auto;overflow:visible;padding:0;background:transparent;border-radius:0"
                                .into(),
                        ),
                        loc,
                    ),
                    Self::attr(
                        "data-bs-theme",
                        self.themes
                            .last()
                            .cloned()
                            .unwrap_or(Value::Text("light".into())),
                        loc,
                    ),
                    Self::attr("onclose", Value::Expr(self.hide(id)), loc),
                    // Voile de Bootstrap : `--bs-backdrop-bg` #000 à `--bs-backdrop-opacity` .5.
                    Self::attr("backdrop", Value::Text("rgba(0,0,0,.5)".into()), loc),
                    Self::attr("placement", Value::Text(placement.into()), loc),
                    // Comme le script de Bootstrap : le focus va à la modale elle-même.
                    Self::attr("autofocus", Value::Bool, loc),
                ],
                children,
                loc,
            };
            return vec![Node::If {
                branches: vec![(self.shown(id, default), vec![Node::Element(dialog)])],
                otherwise: None,
                loc,
            }];
        }
        // Offcanvas : panneau fixe et voile qui ferme.
        if let Some(id) = &id
            && has_class(&e, "offcanvas")
            && self.plan.offcanvas.contains(id)
        {
            let default = Self::remove_class(&mut e, "show");
            Self::add_class(&mut e, "show");
            self.dismissible.push(("offcanvas".into(), id.clone()));
            let el = self.element(e);
            self.dismissible.pop();
            let backdrop = Element {
                name: "div".into(),
                path: TokenStream::new(),
                attrs: vec![
                    Self::attr(
                        "class",
                        Value::Text("offcanvas-backdrop fade show".into()),
                        loc,
                    ),
                    Self::attr("onclick", Value::Expr(self.hide(id)), loc),
                ],
                children: Vec::new(),
                loc,
            };
            return vec![Node::If {
                branches: vec![(self.shown(id, default), vec![Node::Element(backdrop), el])],
                otherwise: None,
                loc,
            }];
        }
        // Alerte fermable.
        if has_class(&e, "alert") && contains_dismiss(&e.children, "alert") {
            let key = id.clone().unwrap_or_else(|| key(&loc));
            self.dismissible.push(("alert".into(), key.clone()));
            let el = self.element(e);
            self.dismissible.pop();
            return vec![Node::If {
                branches: vec![(self.shown(&key, true), vec![el])],
                otherwise: None,
                loc,
            }];
        }
        vec![self.element(e)]
    }
    fn element(&mut self, mut e: Element) -> Node {
        let id = text(&e, "id");
        match text(&e, "data-bs-toggle").as_deref() {
            Some("collapse") => {
                if let Some(t) = target(&e) {
                    let default = self.default_of(&t);
                    let group = self
                        .plan
                        .collapses
                        .get(&t)
                        .cloned()
                        .flatten()
                        .and_then(|g| self.plan.accordions.get(&g).cloned())
                        .unwrap_or_default();
                    let toggle = self.toggle(&t, default, &group);
                    Self::set(&mut e, "onclick", Value::Expr(toggle));
                    Self::remove_class(&mut e, "collapsed");
                    Self::set(
                        &mut e,
                        "class:collapsed",
                        Value::Expr(self.not_shown(&t, default)),
                    );
                    Self::set(
                        &mut e,
                        "aria-expanded",
                        Value::Expr(self.shown(&t, default)),
                    );
                }
            }
            Some("tab" | "pill" | "list") => {
                if let Some(t) = target(&e) {
                    let default = Self::remove_class(&mut e, "active");
                    let group = self
                        .plan
                        .panes
                        .get(&t)
                        .and_then(|g| self.plan.tab_sets.get(g))
                        .cloned()
                        .unwrap_or_default();
                    let show = self.expr(&format!(
                        "::ui::bootstrap::BootstrapAction::Show{{target:{t:?},group:{}}}",
                        Self::list(&group)
                    ));
                    Self::set(&mut e, "onclick", Value::Expr(show));
                    // L'onglet actif par défaut est celui du volet actif par défaut.
                    Self::set(&mut e, "class:active", Value::Expr(self.shown(&t, default)));
                }
            }
            Some("modal" | "offcanvas") => {
                if let Some(t) = target(&e) {
                    let toggle = self.toggle(&t, false, &[]);
                    Self::set(&mut e, "onclick", Value::Expr(toggle));
                }
            }
            Some("tooltip" | "popover") => {
                if e.attr("title").is_none()
                    && let Some(t) =
                        text(&e, "data-bs-title").or_else(|| text(&e, "data-bs-content"))
                {
                    Self::set(&mut e, "title", Value::Text(t));
                }
            }
            _ => {}
        }
        if let Some(kind) = text(&e, "data-bs-dismiss")
            && e.attr("onclick").is_none()
            && let Some((_, key)) = self.dismissible.iter().rev().find(|(k, _)| *k == kind)
        {
            let hide = self.hide(key);
            Self::set(&mut e, "onclick", Value::Expr(hide));
        }
        // Cibles : collapse et volets d'onglets.
        if let Some(id) = &id {
            if self.plan.collapses.contains_key(id) && has_class(&e, "collapse") {
                let default = Self::remove_class(&mut e, "show");
                Self::set(&mut e, "class:show", Value::Expr(self.shown(id, default)));
                // Ouverture et fermeture animées comme `.collapsing` (hauteur 0 ↔ naturelle,
                // contenu rogné), au minutage de la feuille.
                if e.attr("data-reveal").is_none() && !has_class(&e, "collapse-horizontal") {
                    Self::set(&mut e, "data-bs-reveal", Value::Text("collapsing".into()));
                }
            }
            if self.plan.panes.contains_key(id) {
                let default = Self::remove_class(&mut e, "active");
                Self::remove_class(&mut e, "show");
                Self::set(&mut e, "class:active", Value::Expr(self.shown(id, default)));
                Self::set(&mut e, "class:show", Value::Expr(self.shown(id, default)));
            }
        }
        // `.form-check` : case flottante à gauche du libellé, rendue par une rangée. La
        // case quitte le retrait du parent (`padding-left: 1.5em`, `margin-left: -1.5em`
        // dans Bootstrap) : retrait nul, case au bord, libellé après une marge de .5em (8 px, police de 16 px)
        // (largeur 1em + .5em = 1.5em ; interrupteur 2em + .5em = 2.5em).
        if has_class(&e, "form-check")
            && e.children
                .iter()
                .any(|c| matches!(c, Node::Element(i) if has_class(i, "form-check-input")))
        {
            let append = |e: &mut Element, extra: &str| {
                let style = match text(e, "style") {
                    Some(s) => format!("{s};{extra}"),
                    None => extra.into(),
                };
                Self::set(e, "style", Value::Text(style));
            };
            append(&mut e, "display:flex;padding-left:0");
            for child in &mut e.children {
                if let Node::Element(input) = child
                    && has_class(input, "form-check-input")
                {
                    append(input, "margin-left:0;margin-right:8px");
                }
            }
        }
        let theme = e.attr("data-bs-theme").map(|a| a.value.clone());
        if let Some(t) = &theme {
            self.themes.push(t.clone());
        }
        e.children = self.nodes(std::mem::take(&mut e.children));
        if theme.is_some() {
            self.themes.pop();
        }
        Node::Element(e)
    }
    /// État par défaut d'une cible : `show` (collapse) ou `active` (volet) écrit dans le
    /// template. Lu dans le plan collecté avant la réécriture.
    fn default_of(&self, target: &str) -> bool {
        self.defaults.get(target).copied().unwrap_or(false)
    }
    fn dropdown_menu(&mut self, mut e: Element, menu: &str, anchor: &str, label: &str) -> Node {
        let loc = e.loc;
        if e.attr("data-bs-theme").is_none()
            && let Some(t) = self.themes.last().cloned()
        {
            Self::set(&mut e, "data-bs-theme", t);
        }
        Self::add_class(&mut e, "show");
        // Le menu est placé par le popover : ni position absolue ni masquage.
        let style = match text(&e, "style") {
            Some(s) => format!("{s};position:static;display:block"),
            None => "position:static;display:block".into(),
        };
        Self::set(&mut e, "style", Value::Text(style));
        let hide = self.hide(menu);
        // Les éléments du menu le ferment.
        close_items(&mut e.children, &hide);
        let content = self.element(e);
        let popover = Element {
            name: "k-popover".into(),
            path: TokenStream::new(),
            attrs: vec![
                Self::attr("anchor", Value::Text(anchor.into()), loc),
                Self::attr("menu", Value::Bool, loc),
                Self::attr("aria-label", Value::Text(label.into()), loc),
                Self::attr("onclose", Value::Expr(hide), loc),
                Self::attr("placement", Value::Text("bottom-start".into()), loc),
                Self::attr("offset", Value::Text("2".into()), loc),
                // Surface du menu : celle de `.dropdown-menu`, pas celle du popover.
                Self::attr(
                    "style",
                    Value::Text("padding:0;background:transparent;border:none;width:160px".into()),
                    loc,
                ),
            ],
            children: vec![content],
            loc,
        };
        Node::If {
            branches: vec![(self.shown(menu, false), vec![Node::Element(popover)])],
            otherwise: None,
            loc,
        }
    }
}
/// Les `.dropdown-item` sont des éléments de menu ; sans `onclick`, ils ferment le menu.
fn close_items(nodes: &mut [Node], hide: &TokenStream) {
    for n in nodes {
        if let Node::Element(e) = n {
            if has_class(e, "dropdown-item") {
                let loc = e.loc;
                if e.attr("onclick").is_none() {
                    e.attrs.push(Attr {
                        name: "onclick".into(),
                        value: Value::Expr(hide.clone()),
                        loc,
                    });
                }
                if e.attr("role").is_none() {
                    e.attrs.push(Attr {
                        name: "role".into(),
                        value: Value::Text("menuitem".into()),
                        loc,
                    });
                }
            }
            close_items(&mut e.children, hide);
        }
    }
}
