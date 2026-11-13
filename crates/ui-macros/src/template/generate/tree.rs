//! Parcours des enfants : regroupement du contenu en ligne, blocs `{#if}`, renvoi des
//! boucles `{#each}` à `each`, emplacements de slots.
use super::{
    Dest, Generator, Item, R,
    inputs::{input_type, wraps_input},
};
use crate::{
    Error, Result,
    template::{
        Document,
        html::{self, Kind},
        markup::{Attr, Element, Node, Value},
    },
};

pub(super) fn is_whitespace(node: &Node) -> bool {
    matches!(node, Node::Text(t, _) if t.chars().all(|c| c.is_whitespace() && c != '\u{a0}'))
}
pub(super) fn flatten<'a>(
    nodes: &'a [Node],
    slot: Option<&'a str>,
    out: &mut Vec<(&'a Node, Option<&'a str>)>,
) {
    for node in nodes {
        if let Node::Fragment {
            nodes,
            slot: forced,
        } = node
        {
            flatten(nodes, forced.as_deref().or(slot), out);
        } else {
            out.push((node, slot));
        }
    }
}
/// `true` pour un nœud de contenu en ligne (texte, `{expr}`, `<b>`…).
pub(super) fn inline(node: &Node) -> bool {
    match node {
        Node::Text(..) | Node::Expr(..) => true,
        Node::Element(e) => {
            matches!(html::kind(&e.name), Ok(Kind::Inline | Kind::LineBreak))
                && !(e.name == "label" && wraps_input(e))
        }
        Node::If {
            branches,
            otherwise,
            ..
        } => {
            branches.iter().all(|(_, b)| b.iter().all(inline))
                && otherwise.iter().all(|b| b.iter().all(inline))
        }
        Node::Each(e) => e.body.iter().all(inline),
        Node::Fragment { nodes, .. } => nodes.iter().all(inline),
        Node::Embed(..) | Node::Include { .. } => false,
    }
}
/// Nombre d'éléments à partir duquel un sous-arbre a sa propre fonction.
const SUBTREE: usize = 3;
/// Nombre d'éléments (et d'insertions Rust) d'une suite de nœuds, blocs compris.
fn weight(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Element(e) => 1 + weight(&e.children),
            Node::Embed(..) | Node::Include { .. } => 1,
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                branches.iter().map(|(_, b)| weight(b)).sum::<usize>()
                    + otherwise.as_deref().map_or(0, weight)
            }
            Node::Each(e) => weight(&e.body) + e.otherwise.as_deref().map_or(0, weight),
            Node::Fragment { nodes, .. } => weight(nodes),
            Node::Text(..) | Node::Expr(..) => 0,
        })
        .sum()
}
impl Generator {
    pub(super) fn root(&mut self, nodes: &[Node], doc: &Document) -> Result<String> {
        let mut flat = Vec::new();
        flatten(nodes, None, &mut flat);
        let items = self.group(&flat)?;
        let fail = || {
            Error::new(
                format!(
                    "{}: a template needs one root element; wrap siblings in <div>",
                    doc.display
                ),
                doc.span,
            )
        };
        match items.as_slice() {
            [Item::Block(Node::Element(e), _)] => {
                if e.attr("hidden").is_some() {
                    return Err(e.loc.error("The root element cannot be hidden"));
                }
                if e.name == "slot" {
                    return Err(e.loc.error("<slot> cannot be the root of a template"));
                }
                self.element(e)
            }
            [Item::Block(Node::Embed(t, _), _)] => {
                let site = self.next();
                let value = self.out.value(t);
                Ok(format!(
                    "{R}::ui::template::embed({value},__ui_scope.item({site}))"
                ))
            }
            [Item::Inline(group, _)] => self.text_group(group),
            [Item::Block(node, _)] => Err(node.loc().map_or_else(fail, |l| {
                l.error("The root of a template must be an element, not a block")
            })),
            _ => Err(fail()),
        }
    }
    /// Regroupe le contenu en ligne adjacent (texte anonyme) entre les blocs.
    pub(super) fn group<'a>(&self, flat: &[(&'a Node, Option<&'a str>)]) -> Result<Vec<Item<'a>>> {
        let mut items = Vec::new();
        let mut current: Option<(Vec<&'a Node>, Option<&'a str>)> = None;
        let close = |current: &mut Option<(Vec<&'a Node>, Option<&'a str>)>,
                     items: &mut Vec<Item<'a>>| {
            if let Some((mut nodes, slot)) = current.take() {
                while nodes.last().is_some_and(|n| is_whitespace(n)) {
                    nodes.pop();
                }
                if nodes.is_empty() {
                    return;
                }
                if let [node] = nodes.as_slice()
                    && matches!(node, Node::If { .. })
                {
                    items.push(Item::Block(node, slot));
                } else {
                    items.push(Item::Inline(nodes, slot));
                }
            }
        };
        for (node, slot) in flat {
            if is_whitespace(node) {
                if let Some((nodes, _)) = &mut current {
                    nodes.push(node);
                }
                continue;
            }
            if inline(node) {
                // Un élément en ligne qui vise un slot forme son propre paragraphe.
                let own = match node {
                    Node::Element(e) => match e.attr("slot").map(|a| &a.value) {
                        Some(Value::Text(name)) => Some(name.as_str()),
                        _ => None,
                    },
                    _ => None,
                };
                let slot = own.or(*slot);
                if let Some((_, s)) = &current
                    && (*s != slot || own.is_some())
                {
                    close(&mut current, &mut items);
                }
                current
                    .get_or_insert_with(|| (Vec::new(), slot))
                    .0
                    .push(node);
                if own.is_some() {
                    close(&mut current, &mut items);
                }
            } else {
                close(&mut current, &mut items);
                items.push(Item::Block(node, *slot));
            }
        }
        close(&mut current, &mut items);
        Ok(items)
    }
    pub(super) fn push(dest: Dest, slot: Option<&str>, element: &str) -> String {
        if dest.slots {
            format!("{}.push({:?},{element});", dest.var, slot.unwrap_or(""))
        } else {
            format!("{}.push({element});", dest.var)
        }
    }
    pub(super) fn children(
        &mut self,
        nodes: &[Node],
        dest: Dest,
        slot: Option<&str>,
    ) -> Result<String> {
        let mut flat = Vec::new();
        flatten(nodes, slot, &mut flat);
        let items = self.group(&flat)?;
        let mut code = String::new();
        let mut index = 0;
        while index < items.len() {
            match &items[index] {
                Item::Inline(group, slot) => {
                    let paragraph = self
                        .text_group(group)
                        .map(|text| Self::push(dest, *slot, &text));
                    let significant = group
                        .iter()
                        .filter(|n| !is_whitespace(n))
                        .copied()
                        .collect::<Vec<_>>();
                    let splittable = significant.len() > 1
                        && significant.iter().all(|n| {
                            matches!(n, Node::Text(..) | Node::Expr(..))
                                || matches!(n, Node::Element(e) if e.name != "br")
                        });
                    match self.flex.last().cloned().flatten() {
                        Some(flex) if splittable => {
                            // Parent flex ou grille : chaque élément en ligne (et chaque
                            // texte anonyme) est un élément flex ; sinon un paragraphe.
                            let mut items = String::new();
                            for node in significant {
                                let text = self.text_group(&[node])?;
                                items += &Self::push(dest, *slot, &text);
                            }
                            code += &match paragraph {
                                Ok(paragraph) if flex != "true" => {
                                    format!("if {flex}{{{items}}}else{{{paragraph}}}")
                                }
                                // Portions à id/title/onclick (refusées dans un paragraphe),
                                // ou parent flex connu : des éléments séparés.
                                _ => items,
                            };
                        }
                        _ => code += &paragraph?,
                    }
                }
                Item::Block(node, forced) => {
                    // Texte qui suit une case à cocher : son libellé, comme en HTML. Un
                    // `<label for>` qui la vise reste un élément à part (Bootstrap).
                    let label = match (node, items.get(index + 1)) {
                        (Node::Element(e), Some(Item::Inline(group, _)))
                            if e.name == "input"
                                && matches!(input_type(e)?.as_str(), "checkbox" | "radio")
                                && e.attr("aria-label").is_none()
                                && !matches!(e.attr("id"), Some(Attr { value: Value::Text(id), .. })
                                    if self.labels.contains_key(id)) =>
                        {
                            index += 1;
                            Some(self.plain_text(group, "a checkbox label")?)
                        }
                        _ => None,
                    };
                    code += &self.block(node, dest, *forced, label)?;
                }
            }
            index += 1;
        }
        Ok(code)
    }
    fn block(
        &mut self,
        node: &Node,
        dest: Dest,
        forced: Option<&str>,
        label: Option<String>,
    ) -> Result<String> {
        match node {
            Node::Element(e) => {
                let own = match e.attr("slot") {
                    Some(a) => Some(Self::literal(a)?.to_owned()),
                    None => None,
                };
                let slot = forced.map(str::to_owned).or(own);
                if !dest.slots && slot.as_deref().is_some_and(|s| !s.is_empty()) {
                    return Err(e
                        .loc
                        .error("slot=\"…\" belongs to a direct child of a component"));
                }
                if e.name == "slot" {
                    return self.slot(e, dest, slot.as_deref());
                }
                let hidden = match e.attr("hidden") {
                    None => None,
                    Some(a) => match &a.value {
                        Value::Bool => return Ok(String::new()),
                        _ => Some(self.boolean(a)?),
                    },
                };
                let element = if e.name == "input" {
                    self.input(e, label)?
                } else {
                    self.element(e)?
                };
                // Un sous-arbre (include, section…) est construit dans son propre cadre
                // de pile : sinon toute la page tient dans une seule fonction, dont les
                // temporaires débordent la pile d'un build debug.
                let element = if weight(std::slice::from_ref(node)) >= SUBTREE {
                    format!("{R}::ui::template::subtree(||{element})")
                } else {
                    element
                };
                let push = Self::push(dest, slot.as_deref(), &element);
                Ok(match hidden {
                    Some(cond) => format!("if !{cond}{{{push}}}"),
                    None => push,
                })
            }
            Node::Embed(t, _) => {
                let site = self.next();
                let value = self.out.value(t);
                Ok(Self::push(
                    dest,
                    forced,
                    &format!("{R}::ui::template::embed({value},__ui_scope.item({site}))"),
                ))
            }
            Node::If {
                branches,
                otherwise,
                ..
            } => {
                let mut code = String::new();
                for (i, (cond, body)) in branches.iter().enumerate() {
                    let cond = self.out.value(cond);
                    if i > 0 {
                        code += "else ";
                    }
                    code += &format!("if {cond}{{{}}}", self.children(body, dest, forced)?);
                }
                if let Some(body) = otherwise {
                    code += &format!("else{{{}}}", self.children(body, dest, forced)?);
                }
                Ok(code)
            }
            Node::Each(each) => self.each(each, dest, forced),
            Node::Text(..) | Node::Expr(..) | Node::Include { .. } | Node::Fragment { .. } => {
                unreachable!("grouped or expanded")
            }
        }
    }
    /// Borne haute du nombre d'éléments que [`children`](Self::children) ajoute à sa
    /// destination pour `nodes`, hors des boucles `{#each}` (qui réservent leur part) et
    /// des slots : capacité initiale d'une liste d'enfants.
    pub(super) fn capacity(&self, nodes: &[Node]) -> usize {
        let mut flat = Vec::new();
        flatten(nodes, None, &mut flat);
        let Ok(items) = self.group(&flat) else {
            return 0;
        };
        items
            .iter()
            .map(|item| match item {
                // Un paragraphe, ou un élément par portion dans un parent flex.
                Item::Inline(group, _) => group.iter().filter(|n| !is_whitespace(n)).count().max(1),
                Item::Block(Node::Element(e), _) if e.name == "slot" => 0,
                Item::Block(Node::Element(_) | Node::Embed(..), _) => 1,
                Item::Block(
                    Node::If {
                        branches,
                        otherwise,
                        ..
                    },
                    _,
                ) => branches
                    .iter()
                    .map(|(_, body)| body)
                    .chain(otherwise)
                    .map(|body| self.capacity(body))
                    .max()
                    .unwrap_or(0),
                Item::Block(..) => 0,
            })
            .sum()
    }
    fn slot(&mut self, e: &Element, dest: Dest, slot: Option<&str>) -> Result<String> {
        for a in &e.attrs {
            if a.name != "name" {
                return Err(a
                    .loc
                    .error(format!("Unknown attribute '{}' on <slot>", a.name)));
            }
        }
        if !self.slots {
            return Err(e
                .loc
                .error("<slot> requires @slots(slots); or ui_file!(…, slots = slots)"));
        }
        let requested = match e.attr("name") {
            Some(a) => Self::literal(a)?.to_owned(),
            None => String::new(),
        };
        let fallback = self.children(&e.children, dest, slot)?;
        let extend = if dest.slots {
            format!("{}.extend({:?},__ui_taken);", dest.var, slot.unwrap_or(""))
        } else {
            format!("{}.extend(__ui_taken);", dest.var)
        };
        Ok(format!(
            "{{let __ui_taken=__ui_slots.take({requested:?});if __ui_taken.is_empty(){{{fallback}}}else{{{extend}}}}}"
        ))
    }
}
