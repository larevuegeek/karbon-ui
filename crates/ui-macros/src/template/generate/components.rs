//! Composants et inclusions : `{@include}` développés avant la génération, appels
//! de composants Rust avec leurs slots.
use super::{Dest, Generator, R};
use crate::{
    Result,
    input::Source,
    template::{
        html::Kind,
        markup::{self, Element, Node, Value},
    },
};

impl Generator {
    pub(super) fn expand(&mut self, nodes: Vec<Node>, source: &Source) -> Result<Vec<Node>> {
        let mut out = Vec::with_capacity(nodes.len());
        for node in nodes {
            out.push(match node {
                Node::Include { path, slot, loc } => {
                    self.reader.source = source.clone();
                    let (text, nested, target) = self
                        .reader
                        .text(&path, loc.span)
                        .map_err(|e| loc.error(e.message))?;
                    let parsed = markup::parse_text(&text, &nested.display, loc.span)?;
                    self.reader.stack.push(target);
                    let nodes = self.expand(parsed, &nested)?;
                    self.reader.stack.pop();
                    Node::Fragment { nodes, slot }
                }
                Node::Element(mut e) => {
                    if e.name == "svg" && e.attr("src").is_some() {
                        e = self.svg_file(e, source)?;
                    }
                    e.children = self.expand(std::mem::take(&mut e.children), source)?;
                    Node::Element(e)
                }
                Node::If {
                    branches,
                    otherwise,
                    loc,
                } => Node::If {
                    branches: branches
                        .into_iter()
                        .map(|(c, b)| Ok((c, self.expand(b, source)?)))
                        .collect::<Result<_>>()?,
                    otherwise: otherwise.map(|b| self.expand(b, source)).transpose()?,
                    loc,
                },
                Node::Each(mut each) => {
                    each.body = self.expand(std::mem::take(&mut each.body), source)?;
                    each.otherwise = each
                        .otherwise
                        .take()
                        .map(|b| self.expand(b, source))
                        .transpose()?;
                    Node::Each(each)
                }
                other => other,
            });
        }
        Ok(out)
    }
    pub(super) fn component(&mut self, e: &Element) -> Result<String> {
        let site = self.next();
        let id = self.id(e, site)?;
        self.flex.push(None);
        let children = self.children(
            &e.children,
            Dest {
                var: "__ui_component_slots",
                slots: true,
            },
            None,
        )?;
        self.flex.pop();
        let path = self.out.value(&e.path);
        let reserved = |name: &str| {
            matches!(
                name,
                "id" | "class"
                    | "slot"
                    | "hidden"
                    | "props"
                    | "tabindex"
                    | "onhover"
                    | "onpress"
                    | "onfocus"
                    | "ondblclick"
            ) || name.starts_with("data-")
                || name.starts_with("class:")
                || name.starts_with("style:")
                || (name == "style"
                    && matches!(e.attr("style").map(|a| &a.value), Some(Value::Text(_))))
        };
        let mut fields = String::new();
        for a in &e.attrs {
            if reserved(&a.name) {
                continue;
            }
            let field = a.name.replace('-', "_");
            if !field.chars().all(|c| c.is_alphanumeric() || c == '_')
                || field.starts_with(|c: char| c.is_ascii_digit())
            {
                return Err(a
                    .loc
                    .error(format!("'{}' is not a valid component prop name", a.name)));
            }
            let value = self.value(a);
            fields += &format!("{field}:{R}::ui::template::prop({value}),");
        }
        let props = if let Some(p) = e.attr("props") {
            if !fields.is_empty() {
                return Err(p
                    .loc
                    .error("Use either props={…} or individual component props"));
            }
            let Value::Expr(t) = &p.value else {
                return Err(p
                    .loc
                    .error("props expects a Rust value: props={Card { … }}"));
            };
            let value = self.out.value(t);
            format!("{{let __ui_props:{path}={value};__ui_props}}")
        } else {
            format!(
                "{{#[allow(clippy::needless_update)] let __ui_props={path}{{{fields}..::core::default::Default::default()}};__ui_props}}"
            )
        };
        let set_id = if e.attr("id").is_some() {
            "__ui_component.id=__ui_id;"
        } else {
            ""
        };
        let constructor = format!(
            "{R}::ui::template::Node::element({{let mut __ui_component_slots={R}::ui::template::Slots::new();{children}#[allow(unused_mut)] let mut __ui_component={R}::ui::template::Component::render({props},&__ui_context.child(__ui_id),__ui_component_slots);{set_id}__ui_component}})"
        );
        // Les attributs restants sont des props : on ne valide que les communs.
        let kind = Kind::Component;
        self.finish_component(e, kind, &id, &constructor)
    }
    /// Survol, appui, focus et double clic : posés sur l'élément, ou sur la racine d'un
    /// composant, qui voit ainsi ce qui arrive à ses descendants.
    pub(super) fn listeners(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        for (attr, method) in [
            ("onhover", "on_hover"),
            ("onpress", "on_press"),
            ("onfocus", "on_focus"),
            ("ondblclick", "on_double_click"),
        ] {
            if let Some(a) = e.attr(attr) {
                let handler = self.message(a)?;
                code += &format!("__ui_el=__ui_el.{method}({handler});");
            }
        }
        Ok(code)
    }
    fn finish_component(
        &mut self,
        e: &Element,
        kind: Kind,
        id: &str,
        constructor: &str,
    ) -> Result<String> {
        // `onclick` d'un composant est une prop : ne pas l'appliquer au nœud.
        let mut props = String::new();
        props += &self.classes(e)?;
        // `style="…"` et `style:prop={…}` s'appliquent à l'élément racine du composant,
        // comme les classes ; `style={…}` reste une prop.
        if matches!(e.attr("style").map(|a| &a.value), Some(Value::Text(_))) {
            props += &self.style_attribute(e)?;
        }
        props += &self.style_directives(e)?;
        let _ = kind;
        let mut post = self.listeners(e)?;
        if let Some(a) = e.attr("data-exit") {
            post += &format!("__ui_el=__ui_el.exit_animation({});", self.exit(a)?);
        }
        if let Some(a) = e.attr("tabindex")
            && matches!(&a.value, Value::Text(t) if t == "-1")
        {
            post += "__ui_el=__ui_el.tab_stop(false);";
        }
        Ok(format!(
            "{{let __ui_id={id};#[allow(unused_mut)] let __ui_node={constructor};{props}#[allow(unused_mut)] let mut __ui_el=__ui_node.build();{post}__ui_el}}"
        ))
    }
}
