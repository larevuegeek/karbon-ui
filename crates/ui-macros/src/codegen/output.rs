//! Assemblage du code généré : le texte Rust produit par les compilateurs est relu en
//! jetons, les valeurs de l'appelant y sont réinsérées avec leurs spans d'origine et
//! `__ui_root` devient le chemin de la crate `karbon_ui`.
use crate::{Error, Result};
use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree};

pub struct Output {
    root: TokenStream,
    values: Vec<TokenStream>,
    pub user_span: Option<Span>,
}
impl Output {
    pub fn new(root: TokenStream) -> Self {
        Self {
            root,
            values: Vec::new(),
            user_span: None,
        }
    }
    /// Chemin de la crate `karbon_ui` (jetons insérés à la place de `__ui_root`).
    pub fn root(&self) -> TokenStream {
        self.root.clone()
    }
    pub fn value(&mut self, value: &TokenStream) -> String {
        let n = self.values.len();
        self.values.push(value.clone());
        format!("__ui_value_{n}")
    }
    /// Comme [`Output::value`], mais les jetons sont insérés tels quels, sans groupe
    /// invisible (chemins de module dans `use` et dans l'appel d'une macro).
    pub fn raw(&mut self, value: &TokenStream) -> String {
        fn flatten(tokens: TokenStream) -> TokenStream {
            tokens
                .into_iter()
                .flat_map(|t| match t {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::None => flatten(g.stream()),
                    TokenTree::Group(g) => {
                        let mut next = Group::new(g.delimiter(), flatten(g.stream()));
                        next.set_span(g.span());
                        TokenStream::from(TokenTree::Group(next))
                    }
                    other => TokenStream::from(other),
                })
                .collect()
        }
        let n = self.values.len();
        self.values.push(flatten(value.clone()));
        format!("__ui_raw_{n}")
    }
    pub fn finish(&self, code: &str) -> Result<TokenStream> {
        fn replace(tokens: TokenStream, out: &Output) -> TokenStream {
            let mut result = TokenStream::new();
            for token in tokens {
                match token {
                    TokenTree::Ident(i) if i.to_string() == "__ui_root" => {
                        result.extend(out.root.clone())
                    }
                    TokenTree::Ident(i) if i.to_string().starts_with("__ui_raw_") => {
                        let n = i.to_string()[9..].parse::<usize>().unwrap();
                        result.extend(out.values[n].clone());
                    }
                    TokenTree::Ident(i) if i.to_string().starts_with("__ui_value_") => {
                        let n = i.to_string()[11..].parse::<usize>().unwrap();
                        result.extend([TokenTree::Group(Group::new(
                            Delimiter::None,
                            out.values[n].clone(),
                        ))]);
                    }
                    TokenTree::Ident(mut i) => {
                        if i.to_string().starts_with("__ui_") {
                            i.set_span(Span::mixed_site());
                        } else if i.to_string() == "cx"
                            && let Some(span) = out.user_span
                        {
                            i.set_span(span);
                        }
                        result.extend([TokenTree::Ident(i)]);
                    }
                    TokenTree::Group(g) => {
                        let mut next = Group::new(g.delimiter(), replace(g.stream(), out));
                        next.set_span(g.span());
                        result.extend([TokenTree::Group(next)]);
                    }
                    other => result.extend([other]),
                }
            }
            result
        }
        let parsed = code.parse().map_err(|e| {
            Error::new(
                format!("Internal template code generation error: {e}"),
                Span::call_site(),
            )
        })?;
        Ok(replace(parsed, self))
    }
}
