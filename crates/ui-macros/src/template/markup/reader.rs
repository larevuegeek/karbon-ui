pub(super) use super::parser::{Parser, Stop};
pub(super) use super::pieces::{Piece, text_pieces, token_pieces};
use super::*;
pub(super) use crate::Result;
pub(super) use proc_macro::{Span, TokenStream};

pub(super) fn finish(pieces: Vec<Piece>, end: Loc) -> Result<Vec<Node>> {
    let mut parser = Parser {
        pieces,
        at: 0,
        depth: 0,
        end,
    };
    let (nodes, stop) = parser.nodes(None, end)?;
    match stop {
        Stop::Eof => Ok(nodes),
        Stop::Else(_, l) => Err(l.error("{:else} outside of {#if} or {#each}")),
        Stop::CloseBlock(w, l) => Err(l.error(format!("{{/{w}}} without a matching {{#{w}}}"))),
    }
}
/// Analyse les jetons d'un `ui!`.
pub fn parse_tokens(tokens: TokenStream, file: u16) -> Result<Vec<Node>> {
    let end = tokens
        .clone()
        .into_iter()
        .last()
        .map_or(Span::call_site(), |t| t.span());
    let mut pieces = Vec::new();
    token_pieces(tokens, file, &mut pieces, &mut None);
    finish(
        pieces,
        Loc {
            span: end,
            line: 0,
            col: 0,
            file,
        },
    )
}
/// Analyse le texte d'un fichier `.ui` ; `span` situe les erreurs dans le Rust appelant.
pub fn parse_text(text: &str, display: &str, span: Span) -> Result<Vec<Node>> {
    let file = register_file(display);
    let pieces = text_pieces(text, file, span)?;
    let lines = text.lines().count().max(1) as u32;
    finish(
        pieces,
        Loc {
            span,
            line: lines,
            col: 1,
            file,
        },
    )
}
