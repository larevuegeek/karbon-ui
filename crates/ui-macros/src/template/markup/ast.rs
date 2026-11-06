//! Nœuds produits par l'analyse du balisage.
use super::Loc;
use proc_macro::TokenStream;

/// Valeur d'attribut.
#[derive(Clone)]
pub enum Value {
    /// Attribut booléen sans valeur (`disabled`).
    Bool,
    /// Chaîne littérale, entités décodées.
    Text(String),
    /// Expression Rust `{...}`.
    Expr(TokenStream),
}
#[derive(Clone)]
pub struct Attr {
    pub name: String,
    pub value: Value,
    pub loc: Loc,
}
#[derive(Clone)]
pub struct Element {
    pub name: String,
    /// Chemin Rust d'un composant (`Card`, `widgets::Card`).
    pub path: TokenStream,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
    pub loc: Loc,
}
impl Element {
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.iter().find(|a| a.name == name)
    }
}
#[derive(Clone)]
pub struct Each {
    pub iterable: TokenStream,
    pub pattern: TokenStream,
    pub index: Option<TokenStream>,
    pub key: TokenStream,
    pub body: Vec<Node>,
    pub otherwise: Option<Vec<Node>>,
    pub loc: Loc,
}
#[derive(Clone)]
pub enum Node {
    /// Texte brut (entités décodées, blancs non encore regroupés).
    Text(String, Loc),
    /// `{expr}` affiché avec `Display`.
    Expr(TokenStream, Loc),
    Element(Element),
    /// `{@element expr}`.
    Embed(TokenStream, Loc),
    If {
        branches: Vec<(TokenStream, Vec<Node>)>,
        otherwise: Option<Vec<Node>>,
        loc: Loc,
    },
    Each(Each),
    /// `{@include "fichier.ui"}` avant expansion.
    Include {
        path: String,
        slot: Option<String>,
        loc: Loc,
    },
    /// Contenu d'un include après expansion (slot éventuellement imposé).
    Fragment {
        nodes: Vec<Node>,
        slot: Option<String>,
    },
}
impl Node {
    pub fn loc(&self) -> Option<Loc> {
        match self {
            Node::Text(_, l) | Node::Expr(_, l) | Node::Embed(_, l) => Some(*l),
            Node::Element(e) => Some(e.loc),
            Node::If { loc, .. } | Node::Include { loc, .. } => Some(*loc),
            Node::Each(e) => Some(e.loc),
            Node::Fragment { nodes, .. } => nodes.iter().find_map(Node::loc),
        }
    }
}
