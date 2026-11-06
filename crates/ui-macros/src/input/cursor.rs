//! Lecture de jetons Rust pour les arguments de macro, `styles!` et `css_file!`.
use super::Source;
use crate::{Error, Result};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
pub struct Cursor<'a> {
    pub tokens: Vec<TokenTree>,
    pub at: usize,
    pub source: &'a Source,
}
impl<'a> Cursor<'a> {
    pub fn new(tokens: TokenStream, source: &'a Source) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            at: 0,
            source,
        }
    }
    pub fn done(&self) -> bool {
        self.at == self.tokens.len()
    }
    pub fn peek(&self, c: char) -> bool {
        matches!(self.tokens.get(self.at),Some(TokenTree::Punct(p)) if p.as_char()==c)
    }
    pub fn next(&mut self) -> Result<TokenTree> {
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or_else(|| self.error("Unexpected end of input"))?;
        self.at += 1;
        Ok(token)
    }
    pub fn span(&self) -> Span {
        self.tokens
            .get(self.at)
            .map_or(Span::call_site(), TokenTree::span)
    }
    pub fn error(&self, message: &str) -> Error {
        Error::new(format!("{}: {message}", self.source.display), self.span())
    }
    pub fn expect(&mut self, c: char) -> Result<()> {
        if !self.peek(c) {
            return Err(self.error(&format!("Expected '{c}'")));
        }
        self.at += 1;
        Ok(())
    }
    pub fn ident(&mut self) -> Result<String> {
        match self.next()? {
            TokenTree::Ident(i) => Ok(i.to_string()),
            _ => Err(self.error("Expected a name")),
        }
    }
    pub fn string(&mut self) -> Result<String> {
        match self.next()? {
            TokenTree::Literal(l) => {
                super::files::string(&l.to_string()).map_err(|m| self.error(&m))
            }
            _ => Err(self.error("Expected a string literal")),
        }
    }
    pub fn value(&mut self) -> Result<TokenStream> {
        let token = self.next()?;
        if let TokenTree::Group(g) = &token {
            if g.stream().is_empty() {
                return Err(self.error("An expression cannot be empty"));
            }
            return Ok(if g.delimiter() == Delimiter::Brace {
                TokenStream::from(TokenTree::Group(proc_macro::Group::new(
                    Delimiter::Parenthesis,
                    g.stream(),
                )))
            } else {
                TokenStream::from(token)
            });
        }
        if matches!(&token,TokenTree::Punct(p) if p.as_char()=='-') {
            return Ok([token, self.next()?].into_iter().collect());
        }
        Ok(TokenStream::from(token))
    }
}
