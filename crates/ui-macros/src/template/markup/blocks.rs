//! Blocs `{#if}`, `{#each}` et directives `{@…}` du balisage.
use super::{Each, Loc, Node};
use crate::Result;
use proc_macro::{Delimiter, TokenStream, TokenTree};

/// Premier identifiant d'une suite de jetons, et le reste.
pub(super) fn split_word(tokens: TokenStream) -> (Option<String>, TokenStream) {
    let mut iter = tokens.into_iter();
    match iter.next() {
        Some(TokenTree::Ident(i)) => (Some(i.to_string()), iter.collect()),
        Some(other) => (
            None,
            std::iter::once(other).chain(iter).collect::<TokenStream>(),
        ),
        None => (None, TokenStream::new()),
    }
}
pub(super) fn directive(rest: TokenStream, loc: Loc) -> Result<Node> {
    let (word, tail) = split_word(rest);
    match word.as_deref() {
        Some("include") => {
            let tokens = tail.into_iter().collect::<Vec<_>>();
            let path = match tokens.first() {
                Some(TokenTree::Literal(l)) => {
                    crate::input::files::string(&l.to_string()).map_err(|m| loc.error(m))?
                }
                _ => return Err(loc.error("Expected {@include \"file.ui\"}")),
            };
            let slot = match &tokens[1..] {
                [] => None,
                [
                    TokenTree::Ident(i),
                    TokenTree::Punct(p),
                    TokenTree::Literal(l),
                ] if i.to_string() == "slot" && p.as_char() == '=' => {
                    Some(crate::input::files::string(&l.to_string()).map_err(|m| loc.error(m))?)
                }
                _ => {
                    return Err(loc.error(
                        "Expected {@include \"file.ui\"} or {@include \"file.ui\" slot=\"name\"}",
                    ));
                }
            };
            Ok(Node::Include { path, slot, loc })
        }
        Some("element") => {
            if tail.is_empty() {
                return Err(loc.error("Expected {@element expression}"));
            }
            Ok(Node::Embed(tail, loc))
        }
        Some(other) => Err(loc.error(format!(
            "Unknown directive {{@{other}}}; supported: {{@include \"file.ui\"}}, {{@element expr}}"
        ))),
        None => Err(loc.error("Expected {@include \"file.ui\"} or {@element expr}")),
    }
}
pub(super) fn parse_each(tokens: TokenStream, loc: Loc) -> Result<Each> {
    let usage = "Expected {#each items as item (key)} or {#each items as (index, item) (key)}";
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let key = match tokens.last() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => g.stream(),
        _ => {
            return Err(loc.error(
                "{#each} requires a stable key in parentheses: {#each items as item (item.id)}",
            ));
        }
    };
    if key.is_empty() {
        return Err(loc.error("The {#each} key cannot be empty"));
    }
    let body = &tokens[..tokens.len() - 1];
    let at = body
        .iter()
        .rposition(|t| matches!(t, TokenTree::Ident(i) if i.to_string() == "as"))
        .ok_or_else(|| loc.error(usage))?;
    let iterable = body[..at].iter().cloned().collect::<TokenStream>();
    let pattern = body[at + 1..].to_vec();
    if iterable.is_empty() || pattern.is_empty() {
        return Err(loc.error(usage));
    }
    let (index, pattern) = match pattern.as_slice() {
        [TokenTree::Group(g)] if g.delimiter() == Delimiter::Parenthesis => {
            let parts = g.stream().into_iter().collect::<Vec<_>>();
            let comma = parts
                .iter()
                .position(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','));
            match comma {
                Some(c) if c > 0 && c + 1 < parts.len() => (
                    Some(parts[..c].iter().cloned().collect::<TokenStream>()),
                    parts[c + 1..].iter().cloned().collect::<TokenStream>(),
                ),
                _ => (None, pattern.iter().cloned().collect()),
            }
        }
        _ => (None, pattern.iter().cloned().collect()),
    };
    Ok(Each {
        iterable,
        pattern,
        index,
        key,
        body: Vec::new(),
        otherwise: None,
        loc,
    })
}
