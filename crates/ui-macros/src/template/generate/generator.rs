pub(super) use super::super::Document;
pub(super) use super::inputs::{collect_label_toggles, collect_labels};
pub(super) use crate::{
    Result,
    codegen::Output,
    input::{Reader, Source, files},
    template::markup::{Attr, Loc, Node, Value},
};
pub(super) use proc_macro::TokenStream;
pub(super) use std::{collections::HashMap, path::PathBuf};

pub(super) const R: &str = "__ui_root";

/// Compile un template analysé en un bloc Rust qui construit son `Element` racine.
pub fn compile(
    root: TokenStream,
    doc: Document,
    source: Source,
    path: Option<PathBuf>,
) -> Result<TokenStream> {
    let mut g = Generator {
        out: Output::new(root),
        reader: Reader::new(source.clone()),
        counter: 0,
        slots: doc.slots.is_some(),
        styles: None,
        styles_raw: None,
        labels: HashMap::new(),
        label_toggles: HashMap::new(),
        forms: Vec::new(),
        flex: Vec::new(),
        sets: Vec::new(),
        elements: Vec::new(),
        ancestors: Vec::new(),
        last_element: None,
        inline_row: None,
    };
    if let Some(path) = path {
        g.reader.stack.push(path.clone());
        g.reader.dependencies.push(path);
    }
    let mut prefix = String::new();
    if let Some(context) = &doc.context {
        let context = g.out.value(context);
        prefix += &format!("let __ui_context = {R}::ui::template::copy_context(&({context}));");
    } else {
        prefix += &format!("let __ui_context = {R}::ui::template::TemplateContext::default();");
    }
    if let Some(scope) = &doc.scope {
        let scope = g.out.value(scope);
        prefix += &format!("let __ui_context=__ui_context.with_id({scope});");
    }
    if let Some(styles) = &doc.styles {
        g.styles = Some(g.out.value(styles));
        g.styles_raw = Some(g.out.raw(styles));
    }
    let mut doc = doc;
    let nodes = g.expand(std::mem::take(&mut doc.nodes), &source)?;
    let nodes = crate::bootstrap::rewrite(nodes, g.out.root());
    let first_line = nodes
        .iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e.loc.span.line()),
            _ => None,
        })
        .unwrap_or(0);
    let seed = format!("{}:{}", source.display, first_line);
    prefix += &format!("let __ui_scope=__ui_context.id.child({seed:?});");
    if let Some(slots) = &doc.slots {
        let slots = g.out.value(slots);
        prefix += &format!("let __ui_slots=&mut ({slots});");
    }
    collect_labels(&nodes, &mut g.labels);
    collect_label_toggles(&nodes, &mut g.label_toggles);
    let element = g.root(&nodes, &doc)?;
    let dependencies = g
        .reader
        .dependencies
        .iter()
        .map(|p| files::dependency(p))
        .collect::<String>();
    let classes = g.class_macros();
    g.out
        .finish(&format!("{{{dependencies}{classes}{prefix}{element}}}"))
}

/// État de la génération d'un template (code produit, compteurs, contexte des
/// éléments englobants).
pub(super) struct Generator {
    pub(super) out: Output,
    pub(super) reader: Reader,
    pub(super) counter: u64,
    pub(super) slots: bool,
    /// Module de styles (`@styles(css)`), sous forme de valeur générée.
    pub(super) styles: Option<String>,
    /// Même module, chemin inséré sans groupe invisible (appel de macro, `use`).
    pub(super) styles_raw: Option<String>,
    /// `<label for="id">texte</label>` : nom accessible des champs.
    pub(super) labels: HashMap<String, String>,
    /// `<label for="id">` voisin d'une case ou d'un interrupteur (même parent) : le clic
    /// sur le libellé bascule la case, comme en HTML. Valeur : `checked` et `onchange`.
    pub(super) label_toggles: HashMap<String, (Option<Attr>, Attr)>,
    /// Message `onsubmit` des formulaires englobants.
    pub(super) forms: Vec<Option<String>>,
    /// Condition Rust « le conteneur parent est flex ou grille » (`None` : bloc) : ses
    /// éléments en ligne deviennent alors des éléments séparés, comme en HTML.
    pub(super) flex: Vec<Option<String>>,
    /// Listes de classes littérales (module, texte, position) : résolues par la macro
    /// `__karbon_classes!` du module de styles, en fonctions `__ui_cls_<n>_<i>`.
    pub(super) sets: Vec<(String, String, Loc)>,
    /// Signatures des éléments (balise, classes, attributs) : cascade d'une feuille de
    /// framework compilée par `__karbon_classes!` en fonctions `__ui_elm_<n>`.
    pub(super) elements: Vec<(String, String)>,
    /// Faits des éléments englobants (feuilles de framework), du plus extérieur au
    /// parent ; `None` : frontière (composant, couche) au-delà de laquelle le parent
    /// réel n'est pas connu.
    pub(super) ancestors: Vec<Option<String>>,
    /// Numéro de la dernière signature enregistrée par `element_facts`.
    pub(super) last_element: Option<usize>,
    /// Bouton converti en rangée (boîte générée `::after`) : adresse de l'élément
    /// synthétique dont `finish` pose la rangée centrée.
    pub(super) inline_row: Option<usize>,
}
/// Destination des enfants : un `Vec<Element>` ou les slots d'un composant.
#[derive(Clone, Copy)]
pub(super) struct Dest<'a> {
    pub(super) var: &'a str,
    pub(super) slots: bool,
}
/// Élément d'une suite d'enfants après regroupement du contenu en ligne.
pub(super) enum Item<'a> {
    Block(&'a Node, Option<&'a str>),
    Inline(Vec<&'a Node>, Option<&'a str>),
}

impl Generator {
    pub(super) fn next(&mut self) -> u64 {
        let n = self.counter;
        self.counter += 1;
        n
    }
    // -- Valeurs d'attributs -------------------------------------------------
    pub(super) fn value(&mut self, a: &Attr) -> String {
        match &a.value {
            Value::Expr(t) => self.out.value(t),
            Value::Text(s) => format!("{s:?}"),
            Value::Bool => "true".into(),
        }
    }
    pub(super) fn message(&mut self, a: &Attr) -> Result<String> {
        match &a.value {
            Value::Expr(t) => Ok(self.out.value(t)),
            _ => Err(a.loc.error(format!(
                "{} expects a Rust message in braces, e.g. {}={{Msg::Save}}",
                a.name, a.name
            ))),
        }
    }
    pub(super) fn number(&mut self, a: &Attr) -> Result<String> {
        match &a.value {
            Value::Expr(t) => Ok(format!("(({}) as f64)", self.out.value(t))),
            Value::Text(s) => s
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(|n| format!("{n:?}f64"))
                .ok_or_else(|| a.loc.error(format!("{}=\"{s}\" is not a number", a.name))),
            Value::Bool => Err(a.loc.error(format!("{} needs a number", a.name))),
        }
    }
    pub(super) fn boolean(&mut self, a: &Attr) -> Result<String> {
        match &a.value {
            Value::Bool => Ok("true".into()),
            Value::Expr(t) => Ok(format!("({})", self.out.value(t))),
            Value::Text(s) if s.is_empty() || s == &a.name || s == "true" => Ok("true".into()),
            Value::Text(s) if s == "false" => Ok("false".into()),
            Value::Text(s) => Err(a.loc.error(format!(
                "Boolean attribute {}=\"{s}\": write {} or {}={{expr}}",
                a.name, a.name, a.name
            ))),
        }
    }
    pub(super) fn text_attr(&mut self, a: &Attr) -> Result<String> {
        match &a.value {
            Value::Text(s) => Ok(format!("{s:?}")),
            Value::Expr(t) => Ok(self.out.value(t)),
            Value::Bool => Err(a.loc.error(format!("{} needs a text value", a.name))),
        }
    }
    pub(super) fn literal(a: &Attr) -> Result<&str> {
        match &a.value {
            Value::Text(s) => Ok(s),
            _ => Err(a.loc.error(format!("{} must be a literal string", a.name))),
        }
    }
}
