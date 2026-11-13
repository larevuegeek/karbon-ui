//! Analyse des pièces : balises, attributs, texte et blocs.
use super::{
    Attr, Element, Loc, Node, Value,
    blocks::{directive, parse_each, split_word},
    entities::decode,
    pieces::{Piece, relocate},
};
use crate::{Error, Result};
use proc_macro::{TokenStream, TokenTree};

/// Éléments vides (sans balise fermante), comme en HTML.
const VOID: &[&str] = &["input", "img", "br", "hr", "meta", "link", "source", "wbr"];
/// Profondeur d'imbrication maximale (éléments et blocs `{#if}` / `{#each}`) : la
/// lecture et la génération sont récursives.
const MAX_DEPTH: usize = 128;
const NESTING: &str = "Template nesting exceeds 128 levels (elements, {#if} and {#each})";

pub(super) enum Stop {
    Eof,
    Else(Option<TokenStream>, Loc),
    CloseBlock(String, Loc),
}
pub(super) struct Parser {
    pub(super) pieces: Vec<Piece>,
    pub(super) at: usize,
    pub(super) depth: usize,
    pub(super) end: Loc,
}
fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic()
}
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}
fn is_attr_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '@')
}
impl Parser {
    fn peek(&self) -> Option<&Piece> {
        self.pieces.get(self.at)
    }
    fn char_at(&self, n: usize) -> Option<char> {
        match self.pieces.get(self.at + n) {
            Some(Piece::Char(c, _)) => Some(*c),
            _ => None,
        }
    }
    fn loc(&self) -> Loc {
        self.peek().map_or(self.end, Piece::loc)
    }
    fn error(&self, message: impl std::fmt::Display) -> Error {
        self.loc().error(message)
    }
    fn skip_ws(&mut self) {
        while self.char_at(0).is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, c: char) -> bool {
        if self.char_at(0) == Some(c) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, c: char) -> Result<()> {
        if self.eat(c) {
            Ok(())
        } else {
            Err(self.error(format!("Expected '{c}'")))
        }
    }
    /// Nom de balise : lettres, chiffres, `-`, `_`, et chemins Rust `a::B`.
    fn tag_name(&mut self) -> Result<(String, TokenStream)> {
        let loc = self.loc();
        let mut name = String::new();
        if !self.char_at(0).is_some_and(is_name_start) {
            return Err(self.error("Expected a tag name"));
        }
        loop {
            while let Some(c) = self.char_at(0).filter(|c| is_name_char(*c)) {
                name.push(c);
                self.at += 1;
            }
            if self.char_at(0) == Some(':') && self.char_at(1) == Some(':') {
                name.push_str("::");
                self.at += 2;
                continue;
            }
            break;
        }
        let mut path = TokenStream::new();
        if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) || name.contains("::") {
            path = name
                .parse::<TokenStream>()
                .map_err(|_| loc.error(format!("Invalid component path {name}")))?;
            path = relocate(path, loc.span);
        }
        Ok((name, path))
    }
    fn text_until(&mut self, quote: char) -> Result<String> {
        let start = self.loc();
        let mut raw = String::new();
        loop {
            match self.peek() {
                Some(Piece::Char(c, _)) if *c == quote => {
                    self.at += 1;
                    return decode(&raw, start);
                }
                Some(Piece::Char(c, _)) => {
                    raw.push(*c);
                    self.at += 1;
                }
                Some(Piece::Block(..)) => {
                    return Err(self.error(
                        "Braces are not interpolated inside quoted attribute values; use attr={expr}",
                    ));
                }
                None => return Err(start.error("Unclosed attribute quote")),
            }
        }
    }
    fn attribute(&mut self) -> Result<Attr> {
        let loc = self.loc();
        let mut name = String::new();
        while let Some(c) = self.char_at(0).filter(|c| is_attr_char(*c)) {
            name.push(c);
            self.at += 1;
        }
        if name.is_empty() {
            return Err(match self.peek() {
                Some(Piece::Block(..)) => {
                    self.error("Attribute shorthand {name} is not supported; write name={name}")
                }
                _ => self.error("Expected an attribute name, '>' or '/>'"),
            });
        }
        self.skip_ws();
        if !self.eat('=') {
            return Ok(Attr {
                name,
                value: Value::Bool,
                loc,
            });
        }
        self.skip_ws();
        let value = match self.peek().cloned() {
            Some(Piece::Block(tokens, l)) => {
                self.at += 1;
                if tokens.is_empty() {
                    return Err(l.error("An attribute expression cannot be empty"));
                }
                Value::Expr(tokens)
            }
            Some(Piece::Char(q @ ('"' | '\''), _)) => {
                self.at += 1;
                Value::Text(self.text_until(q)?)
            }
            Some(Piece::Char(..)) => {
                let mut raw = String::new();
                while let Some(c) = self.char_at(0).filter(|c| {
                    !c.is_whitespace() && !matches!(c, '>' | '"' | '\'' | '=' | '<' | '`')
                }) {
                    if c == '/' && self.char_at(1) == Some('>') {
                        break;
                    }
                    raw.push(c);
                    self.at += 1;
                }
                if raw.is_empty() {
                    return Err(self.error(format!("Missing value for attribute '{name}'")));
                }
                Value::Text(decode(&raw, loc)?)
            }
            None => return Err(self.error("Unexpected end of template")),
        };
        Ok(Attr { name, value, loc })
    }
    fn element(&mut self) -> Result<Element> {
        if self.depth >= MAX_DEPTH {
            return Err(self.error(NESTING));
        }
        let loc = self.loc();
        self.expect('<')?;
        let (name, path) = self.tag_name()?;
        let mut attrs: Vec<Attr> = Vec::new();
        let self_closing;
        loop {
            self.skip_ws();
            if self.char_at(0) == Some('/') && self.char_at(1) == Some('>') {
                self.at += 2;
                self_closing = true;
                break;
            }
            if self.eat('>') {
                self_closing = false;
                break;
            }
            if self.peek().is_none() {
                return Err(loc.error(format!("Unclosed tag <{name}")));
            }
            let attr = self.attribute()?;
            if attrs.iter().any(|a| a.name == attr.name) {
                return Err(attr
                    .loc
                    .error(format!("Duplicate attribute '{}' on <{name}>", attr.name)));
            }
            attrs.push(attr);
        }
        let children = if self_closing || VOID.contains(&name.as_str()) {
            Vec::new()
        } else {
            self.depth += 1;
            let children = self.nodes(Some(&name), loc)?.0;
            self.depth -= 1;
            children
        };
        Ok(Element {
            name,
            path,
            attrs,
            children,
            loc,
        })
    }
    /// Lit des nœuds jusqu'à la balise fermante `closing`, une branche de bloc ou la fin.
    pub(super) fn nodes(&mut self, closing: Option<&str>, open: Loc) -> Result<(Vec<Node>, Stop)> {
        let mut nodes = Vec::new();
        let mut text = String::new();
        let mut text_loc = None;
        macro_rules! flush {
            () => {
                if let Some(loc) = text_loc.take() {
                    nodes.push(Node::Text(decode(&std::mem::take(&mut text), loc)?, loc));
                }
            };
        }
        loop {
            match self.peek().cloned() {
                None => {
                    flush!();
                    if let Some(name) = closing {
                        return Err(open.error(format!("Missing closing tag </{name}>")));
                    }
                    return Ok((nodes, Stop::Eof));
                }
                Some(Piece::Char('<', loc)) if self.char_at(1) == Some('/') => {
                    flush!();
                    self.at += 2;
                    let (name, _) = self.tag_name()?;
                    self.skip_ws();
                    self.expect('>')?;
                    if closing != Some(name.as_str()) {
                        return Err(loc.error(match closing {
                            Some(expected) => {
                                format!("Mismatched closing tag </{name}>; expected </{expected}>")
                            }
                            None => format!("Unexpected closing tag </{name}>"),
                        }));
                    }
                    return Ok((nodes, Stop::Eof));
                }
                Some(Piece::Char('<', _)) if self.char_at(1).is_some_and(is_name_start) => {
                    flush!();
                    nodes.push(Node::Element(self.element()?));
                }
                Some(Piece::Char(c, loc)) => {
                    text_loc.get_or_insert(loc);
                    text.push(c);
                    self.at += 1;
                }
                Some(Piece::Block(tokens, loc)) => {
                    flush!();
                    self.at += 1;
                    let mut iter = tokens.clone().into_iter();
                    let first = iter.next();
                    let sigil = match &first {
                        Some(TokenTree::Punct(p)) if "#:/@".contains(p.as_char()) => {
                            Some(p.as_char())
                        }
                        _ => None,
                    };
                    let rest: TokenStream = iter.collect();
                    match sigil {
                        None => {
                            if tokens.is_empty() {
                                return Err(loc.error("Empty braces {} in text"));
                            }
                            nodes.push(Node::Expr(tokens, loc));
                        }
                        Some('#') => {
                            // Un bloc compte comme un niveau : la lecture et la génération
                            // descendent récursivement dans son corps.
                            if self.depth >= MAX_DEPTH {
                                return Err(loc.error(NESTING));
                            }
                            self.depth += 1;
                            let block = self.block(rest, loc);
                            self.depth -= 1;
                            nodes.push(block?);
                        }
                        Some(':') => {
                            let (word, tail) = split_word(rest);
                            if word.as_deref() != Some("else") {
                                return Err(loc.error("Expected {:else} or {:else if condition}"));
                            }
                            let (next, cond) = split_word(tail.clone());
                            let condition = match next.as_deref() {
                                None => None,
                                Some("if") if !cond.is_empty() => Some(cond),
                                _ => {
                                    return Err(
                                        loc.error("Expected {:else} or {:else if condition}")
                                    );
                                }
                            };
                            if let Some(name) = closing {
                                return Err(loc.error(format!(
                                    "{{:else}} cannot appear inside <{name}> before it is closed"
                                )));
                            }
                            return Ok((nodes, Stop::Else(condition, loc)));
                        }
                        Some('/') => {
                            let (word, tail) = split_word(rest);
                            if !tail.is_empty() || word.is_none() {
                                return Err(loc.error("Expected {/if} or {/each}"));
                            }
                            if let Some(name) = closing {
                                return Err(loc.error(format!(
                                    "{{/{}}} found before the closing tag </{name}>",
                                    word.unwrap()
                                )));
                            }
                            return Ok((nodes, Stop::CloseBlock(word.unwrap(), loc)));
                        }
                        _ => nodes.push(directive(rest, loc)?),
                    }
                }
            }
        }
    }
    fn block(&mut self, rest: TokenStream, loc: Loc) -> Result<Node> {
        let (word, tail) = split_word(rest);
        match word.as_deref() {
            Some("if") => {
                if tail.is_empty() {
                    return Err(loc.error("{#if} needs a condition: {#if condition}"));
                }
                let mut branches = Vec::new();
                let mut condition = tail;
                loop {
                    let (body, stop) = self.nodes(None, loc)?;
                    match stop {
                        Stop::Else(Some(next), _) => {
                            branches.push((condition, body));
                            condition = next;
                        }
                        Stop::Else(None, _) => {
                            branches.push((condition, body));
                            let (otherwise, stop) = self.nodes(None, loc)?;
                            match stop {
                                Stop::CloseBlock(w, _) if w == "if" => {}
                                Stop::CloseBlock(w, l) => {
                                    return Err(
                                        l.error(format!("Expected {{/if}}, found {{/{w}}}"))
                                    );
                                }
                                Stop::Else(_, l) => {
                                    return Err(l.error("{:else} must be the last branch of {#if}"));
                                }
                                Stop::Eof => return Err(loc.error("Missing {/if}")),
                            }
                            return Ok(Node::If {
                                branches,
                                otherwise: Some(otherwise),
                                loc,
                            });
                        }
                        Stop::CloseBlock(w, l) => {
                            if w != "if" {
                                return Err(l.error(format!("Expected {{/if}}, found {{/{w}}}")));
                            }
                            branches.push((condition, body));
                            return Ok(Node::If {
                                branches,
                                otherwise: None,
                                loc,
                            });
                        }
                        Stop::Eof => return Err(loc.error("Missing {/if}")),
                    }
                }
            }
            Some("each") => {
                let mut each = parse_each(tail, loc)?;
                let (body, stop) = self.nodes(None, loc)?;
                each.body = body;
                match stop {
                    Stop::CloseBlock(w, _) if w == "each" => {}
                    Stop::Else(None, _) => {
                        let (otherwise, stop) = self.nodes(None, loc)?;
                        match stop {
                            Stop::CloseBlock(w, _) if w == "each" => {}
                            _ => return Err(loc.error("Missing {/each}")),
                        }
                        each.otherwise = Some(otherwise);
                    }
                    Stop::CloseBlock(w, l) => {
                        return Err(l.error(format!("Expected {{/each}}, found {{/{w}}}")));
                    }
                    Stop::Else(Some(_), l) => {
                        return Err(l.error("{:else if} is not allowed in {#each}; use {:else}"));
                    }
                    Stop::Eof => return Err(loc.error("Missing {/each}")),
                }
                Ok(Node::Each(each))
            }
            Some(other) => Err(loc.error(format!(
                "Unknown block {{#{other}}}; supported blocks: {{#if}}, {{#each}}"
            ))),
            None => Err(loc.error("Expected {#if condition} or {#each items as item (key)}")),
        }
    }
}
