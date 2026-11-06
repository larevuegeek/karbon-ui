//! Lecture du balisage pour les comportements : attributs texte, classes, cibles
//! (`data-bs-target`, `href="#…"`) et recherches dans les descendants.
use crate::template::markup::{Element, Loc, Node, Value};
pub(super) fn text(e: &Element, name: &str) -> Option<String> {
    match e.attr(name).map(|a| &a.value) {
        Some(Value::Text(t)) => Some(t.clone()),
        _ => None,
    }
}
pub(super) fn classes(e: &Element) -> Vec<String> {
    text(e, "class")
        .map(|c| c.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}
pub(super) fn has_class(e: &Element, class: &str) -> bool {
    classes(e).iter().any(|c| c == class)
}
pub(super) fn target(e: &Element) -> Option<String> {
    text(e, "data-bs-target")
        .or_else(|| text(e, "href"))
        .and_then(|t| t.strip_prefix('#').map(str::to_owned))
}
pub(super) fn key(loc: &Loc) -> String {
    format!("bs-{}-{}-{}", loc.file, loc.line, loc.col)
}
/// Un élément de `nodes` (ou de leurs descendants) porte la classe `class`.
pub(super) fn find_class(nodes: &[Node], class: &str) -> bool {
    nodes.iter().any(|n| match n {
        Node::Element(e) => has_class(e, class) || find_class(&e.children, class),
        _ => false,
    })
}
pub(super) fn contains_dismiss(nodes: &[Node], kind: &str) -> bool {
    nodes.iter().any(|n| match n {
        Node::Element(e) => {
            text(e, "data-bs-dismiss").as_deref() == Some(kind)
                || contains_dismiss(&e.children, kind)
        }
        _ => false,
    })
}
pub(super) fn plain(nodes: &[Node]) -> Option<String> {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(t, _) => out.push_str(t),
            Node::Element(e) => out.push_str(&plain(&e.children).unwrap_or_default()),
            _ => {}
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    (!out.is_empty()).then_some(out)
}
pub(super) fn find_class_text(nodes: &[Node], class: &str) -> Option<String> {
    nodes.iter().find_map(|n| match n {
        Node::Element(e) if has_class(e, class) => plain(&e.children),
        Node::Element(e) => find_class_text(&e.children, class),
        _ => None,
    })
}
