//! Blocs de texte et texte en ligne : paragraphes, titres, groupes de nœuds en
//! ligne, texte brut des attributs et libellés.
use super::{
    Generator, R,
    runs::{Run, collapse, emit, trim_end},
    tree::{inline, is_whitespace},
};
use crate::{
    Error, Result,
    template::{
        html::{self, Kind},
        markup::{Element, Node},
    },
};
use proc_macro::Span;

/// Texte statique d'un nœud (pour `<label for>` et les titres de dialogue).
pub(super) fn static_text(nodes: &[Node]) -> Option<String> {
    let mut text = String::new();
    for node in nodes {
        match node {
            Node::Text(t, _) => text.push_str(t),
            Node::Element(e) if inline(node) => text.push_str(&static_text(&e.children)?),
            _ => return None,
        }
    }
    Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
}
/// Texte littéral des descendants (hors `<svg>`), blancs regroupés ; `None` s'il dépend
/// d'une expression.
pub(super) fn text_content(nodes: &[Node]) -> Option<String> {
    fn walk(nodes: &[Node], out: &mut String) -> bool {
        for node in nodes {
            match node {
                Node::Text(t, _) => {
                    out.push(' ');
                    out.push_str(t);
                }
                Node::Element(e) if e.name == "svg" => {}
                Node::Element(e) => {
                    if !walk(&e.children, out) {
                        return false;
                    }
                }
                Node::Fragment { nodes, .. } => {
                    if !walk(nodes, out) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }
    let mut text = String::new();
    walk(nodes, &mut text).then(|| text.split_whitespace().collect::<Vec<_>>().join(" "))
}
impl Generator {
    // -- Texte enrichi ---------------------------------------------------------
    pub(super) fn text_block(&mut self, e: &Element) -> Result<String> {
        self.check(e, Kind::TextBlock, &[])?;
        if let Some(a) = e.attr("aria-label") {
            return Err(a
                .loc
                .error("aria-label is not supported on text: the text is its accessible name"));
        }
        for child in &e.children {
            self.inline_only(child, &e.name)?;
        }
        let site = self.next();
        let id = self.id(e, site)?;
        let base = html::span_defaults(if e.name == "blockquote" {
            "blockquote"
        } else {
            ""
        });
        let mut runs = Vec::new();
        for child in &e.children {
            self.runs(child, &base, &mut runs)?;
        }
        let pre = e.name == "pre";
        let role = if e.name.starts_with('h') {
            format!(".role({R}::ui::element::Role::Heading)")
        } else {
            String::new()
        };
        self.text_node(e, Kind::TextBlock, &id, runs, pre, &role)
    }
    pub(super) fn inline_only(&self, node: &Node, parent: &str) -> Result<()> {
        if inline(node) || is_whitespace(node) {
            return Ok(());
        }
        let loc = node.loc();
        let what = match node {
            Node::Element(e) => format!("<{}>", e.name),
            _ => "this block".into(),
        };
        Err(loc.map_or_else(
            || Error::new("Inline content expected", Span::call_site()),
            |l| l.error(format!("{what} cannot appear inside <{parent}>: text elements accept inline content only (text, {{expr}}, <b>, <i>, <a>, <code>, <span>, <br>…)")),
        ))
    }
    fn text_node(
        &mut self,
        e: &Element,
        kind: Kind,
        id: &str,
        mut runs: Vec<Run>,
        pre: bool,
        role: &str,
    ) -> Result<String> {
        if pre {
            if let Some(Run::Text(t, _)) = runs.first_mut()
                && let Some(rest) = t.strip_prefix('\n')
            {
                *t = rest.to_owned();
            }
        } else {
            collapse(&mut runs, true);
            trim_end(&mut runs);
        }
        let code = emit(&runs);
        let before = format!(
            "#[allow(unused_mut)] let mut __ui_rich={R}::ui::template::rich(&__ui_context){role};{code}"
        );
        self.finish(
            e,
            kind,
            id,
            &before,
            &format!("{R}::ui::template::Node::rich_text(__ui_id,__ui_rich)"),
        )
    }
    /// Texte anonyme (contenu en ligne d'un conteneur) ou élément en ligne promu.
    pub(super) fn text_group(&mut self, group: &[&Node]) -> Result<String> {
        let significant = group
            .iter()
            .filter(|n| !is_whitespace(n))
            .collect::<Vec<_>>();
        if let [Node::Element(e)] = significant.as_slice()
            && e.name != "br"
        {
            // Un seul élément en ligne : il devient le paragraphe et garde ses attributs.
            self.check(e, Kind::Inline, &["onclick", "for"])?;
            let site = self.next();
            let id = self.id(e, site)?;
            let mut runs = Vec::new();
            if e.name == "a" {
                let base = html::span_defaults("");
                let node: &Node = significant[0];
                self.runs(node, &base, &mut runs)?;
            } else {
                let base = match e.name.as_str() {
                    "i" | "em" => html::span_defaults("i"),
                    _ => html::span_defaults(""),
                };
                for child in &e.children {
                    self.inline_only(child, &e.name)?;
                    self.runs(child, &base, &mut runs)?;
                }
            }
            if e.name == "a" {
                // Les styles du lien vont à sa portion ; le nœud reste neutre.
                return self.text_node_plain(&id, runs);
            }
            return self.text_node(e, Kind::Inline, &id, runs, false, "");
        }
        let site = self.next();
        let base = html::span_defaults("");
        let mut runs = Vec::new();
        for node in group {
            self.runs(node, &base, &mut runs)?;
        }
        self.text_node_plain(&format!("__ui_scope.item({site})"), runs)
    }
    fn text_node_plain(&mut self, id: &str, mut runs: Vec<Run>) -> Result<String> {
        collapse(&mut runs, true);
        trim_end(&mut runs);
        let code = emit(&runs);
        Ok(format!(
            "{{let __ui_id={id};#[allow(unused_mut)] let mut __ui_rich={R}::ui::template::rich(&__ui_context);{code}{R}::ui::template::Node::rich_text(__ui_id,__ui_rich).build()}}"
        ))
    }
    /// Style d'une portion ayant `class` ou `style` : extrait au runtime d'un nœud neutre.
    pub(super) fn span_style(&mut self, e: &Element, parent: &str) -> Result<String> {
        let own = html::span_defaults(&e.name);
        let has_style = e.attr("class").is_some()
            || e.attr("style").is_some()
            || e.attrs
                .iter()
                .any(|a| a.name.starts_with("class:") || a.name.starts_with("style:"));
        if !has_style {
            return Ok(format!("({own}).over({parent})"));
        }
        let classes = self.classes(e)?;
        let styles = self.styles(e)?;
        Ok(format!(
            "{{let __ui_node={R}::ui::template::Node::element({R}::ui::element::Element::column(0,::std::vec::Vec::new()));{classes}{styles}__ui_node.span_style().over(({own}).over({parent}))}}"
        ))
    }
    /// Texte sans mise en forme (libellés de boutons, liens, cases) : code d'une `String`.
    pub(super) fn plain_text(&mut self, nodes: &[&Node], what: &str) -> Result<String> {
        let mut parts = Vec::new();
        fn walk(
            g: &mut Generator,
            node: &Node,
            what: &str,
            parts: &mut Vec<(bool, String)>,
        ) -> Result<()> {
            match node {
                Node::Text(t, _) => parts.push((true, t.clone())),
                Node::Expr(t, _) => parts.push((false, g.out.value(t))),
                Node::Element(e) if e.name == "br" => parts.push((true, "\u{e000}".into())),
                Node::Element(e) if inline(node) && e.attrs.is_empty() => {
                    for c in &e.children {
                        walk(g, c, what, parts)?;
                    }
                }
                Node::Fragment { nodes, .. } => {
                    for c in nodes {
                        walk(g, c, what, parts)?;
                    }
                }
                other => {
                    return Err(other.loc().map_or_else(
                        || Error::new("Plain text expected", Span::call_site()),
                        |l| {
                            l.error(format!(
                                "Only text and {{expressions}} are allowed in {what}"
                            ))
                        },
                    ));
                }
            }
            Ok(())
        }
        for node in nodes {
            walk(self, node, what, &mut parts)?;
        }
        // Blancs regroupés et bords supprimés.
        let mut code = Vec::new();
        let mut space = true;
        for (literal, value) in parts {
            if literal {
                let mut out = String::new();
                for c in value.chars() {
                    if c == '\u{e000}' {
                        // <br> : retour à la ligne, les blancs suivants sont ignorés.
                        out.push('\n');
                        space = true;
                    } else if c.is_whitespace() && c != '\u{a0}' {
                        if !space {
                            out.push(' ');
                            space = true;
                        }
                    } else {
                        out.push(c);
                        space = false;
                    }
                }
                code.push((true, out));
            } else {
                space = false;
                code.push((false, value));
            }
        }
        if let Some((true, last)) = code.last_mut() {
            let len = last.trim_end_matches(' ').len();
            last.truncate(len);
        }
        code.retain(|(l, v)| !(*l && v.is_empty()));
        Ok(match code.as_slice() {
            [] => "\"\"".into(),
            [(true, text)] => format!("{text:?}"),
            _ => {
                let mut s = "{let mut __ui_text=::std::string::String::new();".to_owned();
                for (literal, value) in code {
                    if literal {
                        s += &format!("__ui_text.push_str({value:?});");
                    } else {
                        s += &format!(
                            "__ui_text.push_str(&::std::string::ToString::to_string(&({value})));"
                        );
                    }
                }
                s + "__ui_text}"
            }
        })
    }
}
