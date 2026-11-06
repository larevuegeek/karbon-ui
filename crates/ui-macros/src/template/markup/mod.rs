//! Lecture du balisage HTML des templates : texte libre, balises, attributs,
//! entités, expressions Rust entre accolades et blocs `{#if}` / `{#each}`.
//!
//! Deux sources produisent la même suite de « pièces » : les jetons d'un `ui!`
//! (dont on reconstitue le texte et les espaces grâce aux positions des jetons) et
//! le texte brut d'un fichier `.ui`. Les accolades deviennent des blocs de jetons
//! Rust ; tout le reste est lu caractère par caractère.
//!
//! Organisation : `location` (positions et fichiers pour les diagnostics), `ast`
//! (nœuds produits), `pieces` (découpage des deux sources), `entities` (entités
//! HTML), `parser` (balises, attributs, texte) et `blocks` (`{#if}`, `{#each}`,
//! `{@…}`).
mod ast;
mod blocks;
mod entities;
mod location;
mod parser;
mod pieces;
use crate::Result;
pub use ast::{Attr, Each, Element, Node, Value};
pub use location::{Loc, register_file};
use parser::{Parser, Stop};
use pieces::{Piece, text_pieces, token_pieces};
use proc_macro::{Span, TokenStream};

fn finish(pieces: Vec<Piece>, end: Loc) -> Result<Vec<Node>> {
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
