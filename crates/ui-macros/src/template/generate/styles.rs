//! Styles d'un élément : classes (résolues par `__karbon_classes!`), faits de
//! cascade des feuilles de framework, `style="…"` et directives `style:…`.
use super::{Generator, R};
use crate::{
    Result,
    codegen::{self, StyleProp},
    css,
    template::{
        html::{self, Kind},
        markup::{Element, Loc, Value},
    },
};
use proc_macro::TokenStream;

impl Generator {
    pub(super) fn classes(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        if let Some(class) = e.attr("class") {
            let list: Vec<String> = match &class.value {
                Value::Text(names) => {
                    let module = self.module(class.loc)?;
                    let (id, count) = self.class_set(module, names, class.loc);
                    (0..count).map(|i| format!("__ui_cls_{id}_{i}")).collect()
                }
                Value::Expr(t) => codegen::classes(t)
                    .iter()
                    .map(|v| self.out.value(v))
                    .collect(),
                Value::Bool => return Err(class.loc.error("class needs a value")),
            };
            for class in list {
                code += &format!(
                    "let __ui_node={R}::ui::template::style(__ui_node,&__ui_context,{class});"
                );
            }
        }
        for a in &e.attrs {
            if let Some(name) = a.name.strip_prefix("class:") {
                let module = self.module(a.loc)?;
                let (id, count) = self.class_set(module, name, a.loc);
                if count != 1 {
                    return Err(a.loc.error("class:name={condition} takes one class name"));
                }
                let cond = self.boolean(a)?;
                code += &format!(
                    "let __ui_node=if {cond}{{{R}::ui::template::style(__ui_node,&__ui_context,__ui_cls_{id}_0)}}else{{__ui_node}};"
                );
            }
        }
        Ok(code)
    }
    /// Numéro et nombre de fonctions de la liste de classes `text` du module `module`.
    pub(super) fn class_set(&mut self, module: String, text: &str, loc: Loc) -> (usize, usize) {
        let count = text.split_whitespace().count();
        if let Some(id) = self
            .sets
            .iter()
            .position(|(m, t, _)| *m == module && t == text)
        {
            return (id, count);
        }
        self.sets.push((module, text.to_owned(), loc));
        (self.sets.len() - 1, count)
    }
    /// Appels `module::__karbon_classes!{…}` qui définissent les fonctions des listes.
    pub(super) fn class_macros(&mut self) -> String {
        let mut modules: Vec<&String> = Vec::new();
        for module in self
            .sets
            .iter()
            .map(|(m, _, _)| m)
            .chain(self.elements.iter().map(|(m, _)| m))
        {
            if !modules.contains(&module) {
                modules.push(module);
            }
        }
        let modules = modules.into_iter().cloned().collect::<Vec<_>>();
        let mut code = String::new();
        for module in modules {
            let mut args = String::new();
            let sets = self.sets.clone();
            for (id, (m, text, loc)) in sets.iter().enumerate() {
                if *m != module {
                    continue;
                }
                let prefix = match loc.error("").message.strip_suffix(": ") {
                    Some(at) if loc.line != 0 => format!("{at}: "),
                    _ => String::new(),
                };
                let mut literal = proc_macro::Literal::string(text);
                literal.set_span(loc.span);
                let text = self
                    .out
                    .value(&TokenStream::from(proc_macro::TokenTree::Literal(literal)));
                args += &format!("{id} {prefix:?} {text} ");
            }
            for (id, (m, signature)) in self.elements.iter().enumerate() {
                if *m == module {
                    args += &format!("elem {id} {signature:?} ");
                }
            }
            let path = self.styles_raw.clone().unwrap_or_default();
            code += &format!("{path}::__karbon_classes!{{({R}) ({path}) {args}}}");
        }
        code
    }
    /// Balise, classes et attributs de l'élément, transmis à la feuille de styles : une
    /// feuille de framework y applique sa cascade (`__ui_elm_<n>`), une feuille stricte
    /// une fonction neutre.
    pub(super) fn element_facts(&mut self, e: &Element, kind: Kind) -> Result<String> {
        let Some(module) = self.styles.clone() else {
            return Ok(String::new());
        };
        let (own, mut code) = self.own_facts(e);
        // L'élément lui-même est au sommet de la pile quand il passe par `element`.
        let mut stack = self.ancestors.clone();
        if stack
            .last()
            .is_some_and(|f| f.as_deref() == Some(own.as_str()))
        {
            stack.pop();
        }
        let mut chain = Vec::new();
        for facts in stack.iter().rev() {
            match facts {
                Some(f) => chain.push(f.replace('\u{1d}', "\u{1b}")),
                None => break,
            }
        }
        let signature = format!(
            "{own}\u{1d}{}\u{1d}{}",
            // Feuille (texte, contrôle) : pas de modèle de conteneur.
            !matches!(
                kind,
                Kind::Container(_) | Kind::Dialog | Kind::Popover | Kind::Split | Kind::VirtualList
            ),
            chain.join("\u{1c}")
        );
        let id = match self
            .elements
            .iter()
            .position(|(m, s)| *m == module && *s == signature)
        {
            Some(id) => id,
            None => {
                self.elements.push((module, signature));
                self.elements.len() - 1
            }
        };
        code += &format!(
            "let __ui_node={R}::ui::template::style(__ui_node,&__ui_context,__ui_elm_{id});"
        );
        self.last_element = Some(id);
        Ok(code)
    }
    /// Faits propres d'un élément : `balise ␝ classes ␝ classes dynamiques ␝ attributs
    /// ␝ attributs dynamiques`, et le code qui pose ses attributs calculés.
    pub(super) fn own_facts(&mut self, e: &Element) -> (String, String) {
        let classes = match e.attr("class").map(|a| &a.value) {
            Some(Value::Text(t)) => t.clone(),
            _ => String::new(),
        };
        let mut dynamic_classes = e
            .attrs
            .iter()
            .filter_map(|a| a.name.strip_prefix("class:"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        // `class={…}` : classes inconnues à la compilation.
        if matches!(e.attr("class").map(|a| &a.value), Some(Value::Expr(_))) {
            dynamic_classes.push("*".into());
        }
        let mut attrs = String::new();
        let mut dynamic_attrs = Vec::new();
        let mut code = String::new();
        if e.name == "input" && e.attr("type").is_none() {
            attrs += "type\u{1f}text\u{1e}";
        }
        for a in &e.attrs {
            let name = a.name.as_str();
            if matches!(name, "class" | "style")
                || name.starts_with("class:")
                || name.starts_with("style:")
                || html::EVENTS.contains(&name)
            {
                continue;
            }
            match &a.value {
                Value::Text(t) => attrs += &format!("{name}\u{1f}{t}\u{1e}"),
                Value::Bool => attrs += &format!("{name}\u{1f}\u{1e}"),
                Value::Expr(t) => {
                    if name.starts_with("data-") || name.starts_with("aria-") {
                        let value = self.out.value(t);
                        code += &format!(
                            "let __ui_node=__ui_node.html_attribute({name:?},::core::option::Option::Some(::std::string::ToString::to_string(&({value}))));"
                        );
                        dynamic_attrs.push(name.to_owned());
                    }
                }
            }
        }
        let own = format!(
            "{}\u{1d}{classes}\u{1d}{}\u{1d}{attrs}\u{1d}{}",
            e.name,
            dynamic_classes.join(" "),
            dynamic_attrs.join(" "),
        );
        (own, code)
    }
    pub(super) fn module(&self, loc: Loc) -> Result<String> {
        self.styles.clone().ok_or_else(|| {
            loc.error("class=\"…\" requires @styles(module); or ui_file!(…, styles = module)")
        })
    }
    pub(super) fn styles(&mut self, e: &Element) -> Result<String> {
        let mut code = self.style_attribute(e)?;
        code += &self.style_directives(e)?;
        Ok(code)
    }
    /// `style="…"` (CSS) ou `style={…}` (classe Rust).
    pub(super) fn style_attribute(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        if let Some(style) = e.attr("style") {
            match &style.value {
                Value::Text(text) => {
                    code += &css::inline(text, &mut self.out, "__ui_context", "__ui_node")
                        .map_err(|m| style.loc.error(m))?;
                }
                Value::Expr(t) => {
                    let value = self.out.value(t);
                    code += &format!("let __ui_node=__ui_node.style({value});");
                }
                Value::Bool => return Err(style.loc.error("style needs declarations")),
            }
        }
        Ok(code)
    }
    /// `style:prop={valeur}`.
    pub(super) fn style_directives(&mut self, e: &Element) -> Result<String> {
        let mut code = String::new();
        for a in &e.attrs {
            if let Some(name) = a.name.strip_prefix("style:") {
                let Value::Expr(t) = &a.value else {
                    return Err(a.loc.error(format!(
                        "style:{name} expects a Rust value; use style=\"{name}: …\" for literal CSS"
                    )));
                };
                if name.starts_with("--") {
                    // Propriété personnalisée héritée (style:--h={expr}).
                    let value = self.out.value(t);
                    code += &format!("let __ui_node=__ui_node.custom_property({name:?},{value});");
                    continue;
                }
                // Décalages CSS (`style:left={…}`) : propriétés offset_* du moteur.
                let name = match name {
                    "top" | "right" | "bottom" | "left" => format!("offset_{name}"),
                    _ => name.replace('-', "_"),
                };
                let attr = StyleProp {
                    name,
                    value: t.clone(),
                    span: a.loc.span,
                };
                code += &codegen::property(&attr, &mut self.out, "__ui_context", "__ui_node")
                    .map_err(|err| a.loc.error(err.message))?;
            }
        }
        Ok(code)
    }
}
