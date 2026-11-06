//! Aplatissement d'une feuille analysée : règles imbriquées résolues (`&`), at-rules
//! appliquées (`@media`, `@container`, `@supports`, `@layer`) ou sautées et comptées,
//! `@keyframes` et `@property` relevés pour la compilation de la feuille.
use super::conditions::{Condition, media, supports};
use super::{Report, selectors, syntax};

/// Contexte d'aplatissement.
#[derive(Clone, Default)]
pub(super) struct Context {
    pub(super) conditions: Vec<String>,
    pub(super) layer: Option<String>,
    pub(super) parent: Option<Vec<String>>,
}
/// Règle aplatie avant découpe en alternatives.
pub(super) struct Flat {
    pub(super) selectors: Vec<String>,
    pub(super) declarations: Vec<syntax::Declaration>,
    pub(super) conditions: Vec<String>,
    pub(super) layer: Option<String>,
    pub(super) at: usize,
}
pub(super) struct Walker {
    pub(super) flats: Vec<Flat>,
    pub(super) layers: Vec<String>,
    pub(super) anonymous: u32,
    pub(super) keyframes: Vec<(String, String)>,
    pub(super) properties: Vec<(String, Option<String>, bool)>,
    pub(super) report: Report,
    pub(super) text: String,
}
/// Fin (exclue) du bloc d'une at-rule commençant à `at`.
fn block_end(text: &str, at: usize) -> usize {
    let Some(open) = text[at..].find('{').map(|i| at + i) else {
        return text.len();
    };
    let mut depth = 0;
    let mut quote = None;
    for (i, c) in text[open..].char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return open + i + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}
/// Sélecteurs d'une règle imbriquée : `&` remplacé par le parent, sinon descendant.
fn nest(prelude: &str, parent: Option<&[String]>) -> Vec<String> {
    let own = selectors::split_list(prelude);
    let Some(parent) = parent else {
        return own;
    };
    let parent_text = if parent.len() == 1 {
        parent[0].clone()
    } else {
        format!(":is({})", parent.join(", "))
    };
    own.iter()
        .map(|s| {
            if s.contains('&') {
                s.replace('&', &parent_text)
            } else {
                format!("{parent_text} {s}")
            }
        })
        .collect()
}
impl Walker {
    fn layer(&mut self, name: &str) {
        if !self.layers.iter().any(|l| l == name) {
            self.layers.push(name.to_owned());
        }
    }
    fn count_inactive(&mut self, nodes: &[syntax::Node]) {
        for n in nodes {
            match n {
                syntax::Node::Rule { children, .. } => {
                    self.report.rules_inactive += 1;
                    self.count_inactive(children);
                }
                syntax::Node::At { block: Some(b), .. } => self.count_inactive(&b.children),
                _ => {}
            }
        }
    }
    fn count_skipped(&mut self, nodes: &[syntax::Node], category: &str) {
        for n in nodes {
            match n {
                syntax::Node::Rule {
                    prelude,
                    declarations,
                    children,
                    at,
                } => {
                    self.report.rules += 1;
                    let selectors = selectors::split_list(prelude).len() as u32;
                    self.report.selectors += selectors;
                    self.report.declarations += declarations.len() as u32;
                    self.report.skip(*at, category.to_owned(), prelude);
                    self.count_skipped(children, category);
                }
                syntax::Node::At { block: Some(b), .. } => {
                    self.count_skipped(&b.children, category)
                }
                _ => {}
            }
        }
    }
    pub(super) fn walk(&mut self, nodes: &[syntax::Node], cx: &Context) {
        for node in nodes {
            match node {
                syntax::Node::Invalid { at, reason } => {
                    self.report.skip(*at, "invalid syntax", reason);
                }
                syntax::Node::Rule {
                    prelude,
                    declarations,
                    children,
                    at,
                } => {
                    let selectors = nest(prelude, cx.parent.as_deref());
                    self.flats.push(Flat {
                        selectors: selectors.clone(),
                        declarations: declarations.clone(),
                        conditions: cx.conditions.clone(),
                        layer: cx.layer.clone(),
                        at: *at,
                    });
                    if !children.is_empty() {
                        let inner = Context {
                            parent: Some(selectors),
                            ..cx.clone()
                        };
                        self.walk(children, &inner);
                    }
                }
                syntax::Node::At {
                    name,
                    prelude,
                    block,
                    at,
                } => self.at_rule(name, prelude, block.as_ref(), *at, cx),
            }
        }
    }
    fn at_rule(
        &mut self,
        name: &str,
        prelude: &str,
        block: Option<&syntax::Block>,
        at: usize,
        cx: &Context,
    ) {
        self.report.at_rules += 1;
        let children = block.map_or(&[][..], |b| &b.children[..]);
        // Déclarations d'un bloc conditionnel imbriqué dans une règle (`.a { @media … }`).
        let nested_rule = |walker: &mut Self, cx: &Context| {
            if let (Some(parent), Some(b)) = (&cx.parent, block)
                && !b.declarations.is_empty()
            {
                walker.flats.push(Flat {
                    selectors: parent.clone(),
                    declarations: b.declarations.clone(),
                    conditions: cx.conditions.clone(),
                    layer: cx.layer.clone(),
                    at,
                });
            }
        };
        match name {
            "media" => match media(prelude) {
                Ok(Condition::Static(true)) => {
                    self.report.at_rules_accepted += 1;
                    nested_rule(self, cx);
                    self.walk(children, cx);
                }
                Ok(Condition::Static(false)) => {
                    self.report.at_rules_accepted += 1;
                    self.count_inactive(children);
                }
                Ok(Condition::Runtime(code)) => {
                    self.report.at_rules_accepted += 1;
                    let mut inner = cx.clone();
                    inner.conditions.push(code);
                    nested_rule(self, &inner);
                    self.walk(children, &inner);
                }
                Err(e) => {
                    self.report.skip(at, format!("@media {e}"), prelude);
                    self.count_skipped(children, "inside unsupported @media");
                }
            },
            "container" => match crate::css::media::condition(&format!("@container {prelude}")) {
                Ok(code) => {
                    self.report.at_rules_accepted += 1;
                    let mut inner = cx.clone();
                    inner.conditions.push(code);
                    nested_rule(self, &inner);
                    self.walk(children, &inner);
                }
                Err(_) => {
                    self.report.skip(at, "@container query", prelude);
                    self.count_skipped(children, "inside unsupported @container");
                }
            },
            "supports" => {
                self.report.at_rules_accepted += 1;
                if supports(prelude) {
                    nested_rule(self, cx);
                    self.walk(children, cx);
                } else {
                    self.count_inactive(children);
                }
            }
            "layer" => {
                self.report.at_rules_accepted += 1;
                let base = cx.layer.clone();
                let full = |name: &str| match &base {
                    Some(b) => format!("{b}.{name}"),
                    None => name.to_owned(),
                };
                match block {
                    None => {
                        for name in prelude.split(',') {
                            let name = full(name.trim());
                            self.layer(&name);
                        }
                    }
                    Some(_) => {
                        let name = if prelude.trim().is_empty() {
                            self.anonymous += 1;
                            full(&format!("<anonymous {}>", self.anonymous))
                        } else {
                            full(prelude.trim())
                        };
                        self.layer(&name);
                        let inner = Context {
                            layer: Some(name),
                            ..cx.clone()
                        };
                        nested_rule(self, &inner);
                        self.walk(children, &inner);
                    }
                }
            }
            "keyframes" => {
                self.report.at_rules_accepted += 1;
                let end = block_end(&self.text, at);
                self.keyframes
                    .push((prelude.trim().to_owned(), self.text[at..end].to_owned()));
            }
            "-webkit-keyframes" | "-moz-keyframes" | "-o-keyframes" => {
                // Doublons préfixés d'un @keyframes standard.
                self.report.at_rules_accepted += 1;
            }
            "property" => {
                self.report.at_rules_accepted += 1;
                let mut initial = None;
                let mut inherits = true;
                for d in block.map_or(&[][..], |b| &b.declarations[..]) {
                    match d.name.as_str() {
                        "initial-value" => initial = Some(d.value.clone()),
                        "inherits" => inherits = d.value.trim() != "false",
                        _ => {}
                    }
                }
                self.properties
                    .push((prelude.trim().to_owned(), initial, inherits));
            }
            "charset" | "namespace" => self.report.at_rules_accepted += 1,
            "font-face" => self.report.skip(
                at,
                "@font-face (fonts are loaded by the application)",
                prelude,
            ),
            "import" => self
                .report
                .skip(at, "@import inside a framework stylesheet", prelude),
            _ => {
                self.report.skip(at, format!("@{name}"), prelude);
                self.count_skipped(children, &format!("inside @{name}"));
            }
        }
    }
}
