//! Découpage des deux sources en « pièces » : caractères situés et blocs de jetons
//! Rust (contenu des accolades).
use super::Loc;
use crate::Result;
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

#[derive(Clone)]
pub(super) enum Piece {
    Char(char, Loc),
    Block(TokenStream, Loc),
}
impl Piece {
    pub(super) fn loc(&self) -> Loc {
        match self {
            Piece::Char(_, l) | Piece::Block(_, l) => *l,
        }
    }
}

// ---------------------------------------------------------------------------
// Pièces depuis les jetons d'un `ui!`.

fn position(span: Span) -> (usize, usize) {
    (span.line(), span.column())
}
pub(super) fn token_pieces(
    tokens: TokenStream,
    file: u16,
    out: &mut Vec<Piece>,
    previous: &mut Option<(usize, usize)>,
) {
    let loc = |span: Span| Loc {
        span,
        line: 0,
        col: 0,
        file,
    };
    let space = |start: (usize, usize),
                 span: Span,
                 out: &mut Vec<Piece>,
                 previous: &Option<(usize, usize)>| {
        if previous.is_some_and(|end| end != start) {
            out.push(Piece::Char(' ', loc(span)));
        }
    };
    for token in tokens {
        let span = token.span();
        space(position(span.start()), span, out, previous);
        match token {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                out.push(Piece::Block(g.stream(), loc(span)));
                *previous = Some(position(span.end()));
            }
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => (Some('('), Some(')')),
                    Delimiter::Bracket => (Some('['), Some(']')),
                    _ => (None, None),
                };
                if let Some(c) = open {
                    out.push(Piece::Char(c, loc(g.span_open())));
                    *previous = Some(position(g.span_open().end()));
                }
                token_pieces(g.stream(), file, out, previous);
                if let Some(c) = close {
                    space(
                        position(g.span_close().start()),
                        g.span_close(),
                        out,
                        previous,
                    );
                    out.push(Piece::Char(c, loc(g.span_close())));
                    *previous = Some(position(g.span_close().end()));
                }
            }
            other => {
                let text = span.source_text().unwrap_or_else(|| other.to_string());
                for c in text.chars() {
                    out.push(Piece::Char(c, loc(span)));
                }
                *previous = Some(position(span.end()));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pièces depuis le texte d'un fichier `.ui`.

/// Fin du bloc Rust ouvert à `start` (`{`), en ignorant chaînes, caractères et
/// commentaires. Renvoie l'index de l'accolade fermante.
fn block_end(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            '"' => {
                // Chaîne brute r#"..."# ?
                let mut hashes = 0;
                let mut j = i;
                while j > 0 && chars[j - 1] == '#' {
                    hashes += 1;
                    j -= 1;
                }
                let raw = j > 0 && chars[j - 1] == 'r';
                i += 1;
                while i < chars.len() {
                    if !raw && chars[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if chars[i] == '"' && (0..hashes).all(|h| chars.get(i + 1 + h) == Some(&'#')) {
                        i += hashes;
                        break;
                    }
                    i += 1;
                }
            }
            '\'' => {
                if chars.get(i + 1) == Some(&'\\') {
                    i += 2;
                    while i < chars.len() && chars[i] != '\'' {
                        i += 1;
                    }
                } else if chars.get(i + 2) == Some(&'\'') {
                    i += 2;
                }
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    None
}
pub(super) fn relocate(tokens: TokenStream, span: Span) -> TokenStream {
    tokens
        .into_iter()
        .map(|mut t| {
            if let TokenTree::Group(g) = t {
                let mut n = proc_macro::Group::new(g.delimiter(), relocate(g.stream(), span));
                n.set_span(span);
                t = TokenTree::Group(n);
            } else {
                t.set_span(span);
            }
            t
        })
        .collect()
}
pub(super) fn text_pieces(text: &str, file: u16, span: Span) -> Result<Vec<Piece>> {
    #[derive(PartialEq)]
    enum Mode {
        Text,
        Tag,
        Quote(char),
    }
    let chars = text.chars().collect::<Vec<_>>();
    let mut lines = Vec::with_capacity(chars.len());
    let (mut line, mut col) = (1u32, 1u32);
    for c in &chars {
        lines.push((line, col));
        if *c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    let loc = |i: usize| {
        let (line, col) = lines.get(i).copied().unwrap_or((line, col));
        Loc {
            span,
            line,
            col,
            file,
        }
    };
    let mut out = Vec::with_capacity(chars.len());
    let mut mode = Mode::Text;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match mode {
            Mode::Text if chars[i..].starts_with(&['<', '!', '-', '-']) => {
                let end = (i + 4..chars.len().saturating_sub(2))
                    .find(|&j| chars[j..].starts_with(&['-', '-', '>']))
                    .ok_or_else(|| loc(i).error("Unclosed comment <!-- … -->"))?;
                i = end + 3;
                continue;
            }
            Mode::Text
                if c == '<'
                    && chars
                        .get(i + 1)
                        .is_some_and(|n| n.is_ascii_alphabetic() || *n == '/') =>
            {
                mode = Mode::Tag
            }
            Mode::Tag if c == '"' || c == '\'' => mode = Mode::Quote(c),
            Mode::Tag if c == '>' => mode = Mode::Text,
            Mode::Quote(q) if c == q => mode = Mode::Tag,
            _ => {}
        }
        if c == '{' && mode != Mode::Quote('"') && mode != Mode::Quote('\'') {
            let end = block_end(&chars, i).ok_or_else(|| loc(i).error("Unclosed '{'"))?;
            let inner = chars[i + 1..end].iter().collect::<String>();
            let tokens = inner
                .parse::<TokenStream>()
                .map_err(|e| loc(i).error(format!("Invalid Rust tokens between braces: {e}")))?;
            out.push(Piece::Block(relocate(tokens, span), loc(i)));
            i = end + 1;
            continue;
        }
        if c == '}' && mode != Mode::Quote('"') && mode != Mode::Quote('\'') {
            return Err(loc(i).error("Unmatched '}'; write &#125; for a literal brace"));
        }
        out.push(Piece::Char(c, loc(i)));
        i += 1;
    }
    Ok(out)
}
