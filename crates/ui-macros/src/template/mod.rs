//! Templates `.ui` : balisage HTML analysé (`markup`), réécrit pour les comportements
//! Bootstrap, puis traduit en constructeurs d'`Element` (`generate`).
pub(crate) mod generate;
pub(crate) mod html;
pub(crate) mod markup;
use crate::{
    Error, Result,
    input::{Cursor, Reader, Source},
};
use markup::Node;
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

/// Template analysé, avec ses directives `@context(...)` & co.
pub struct Document {
    pub nodes: Vec<Node>,
    pub context: Option<TokenStream>,
    pub scope: Option<TokenStream>,
    pub slots: Option<TokenStream>,
    pub styles: Option<TokenStream>,
    /// Position de l'appel (diagnostics globaux).
    pub span: Span,
    pub display: String,
}

/// `ui!` : directives et balisage écrits dans l'appel.
pub(crate) fn compile_inline(root: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let source = Source::call(&input);
    let doc = directives(input, &source)?;
    generate::compile(root, doc, source, None)
}
/// `ui_file!` : chemin du fichier `.ui` relatif à l'appelant, puis options
/// `context = …`, `scope = …`, `slots = …`, `styles = …`.
pub(crate) fn compile_file(root: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let source = Source::call(&input);
    let mut cursor = Cursor::new(input, &source);
    let span = cursor.span();
    let path = cursor.string()?;
    let mut document = Document {
        nodes: Vec::new(),
        context: None,
        scope: None,
        slots: None,
        styles: None,
        span,
        display: String::new(),
    };
    while !cursor.done() {
        cursor.expect(',')?;
        let key = cursor.ident()?;
        cursor.expect('=')?;
        let value = cursor.value()?;
        let target = match key.as_str() {
            "context" => &mut document.context,
            "scope" => &mut document.scope,
            "slots" => &mut document.slots,
            "styles" => &mut document.styles,
            _ => return Err(cursor.error("ui_file!: expected context, scope, slots or styles")),
        };
        if target.replace(value).is_some() {
            return Err(cursor.error("Duplicate ui_file! option"));
        }
    }
    let mut reader = Reader::new(source.clone());
    let (text, nested, target) = reader.text(&path, span)?;
    document.nodes = markup::parse_text(&text, &nested.display, span)?;
    document.display = nested.display.clone();
    generate::compile(root, document, nested, Some(target))
}
/// Leading `@context(...)`, `@scope(...)`, `@slots(...)` and `@styles(...)` directives
/// of an inline template, followed by its markup.
fn directives(input: TokenStream, source: &Source) -> Result<Document> {
    let tokens = input.into_iter().collect::<Vec<_>>();
    let mut at = 0;
    let mut document = Document {
        nodes: Vec::new(),
        context: None,
        scope: None,
        slots: None,
        styles: None,
        span: tokens.first().map_or(Span::call_site(), TokenTree::span),
        display: source.display.clone(),
    };
    while let Some(TokenTree::Punct(p)) = tokens.get(at)
        && p.as_char() == '@'
    {
        let span = p.span();
        let usage =
            "Expected @context(expr);, @scope(expr);, @slots(variable); or @styles(module);";
        let (Some(TokenTree::Ident(name)), Some(TokenTree::Group(g)), Some(TokenTree::Punct(end))) =
            (tokens.get(at + 1), tokens.get(at + 2), tokens.get(at + 3))
        else {
            return Err(Error::new(usage, span));
        };
        if g.delimiter() != Delimiter::Parenthesis || end.as_char() != ';' {
            return Err(Error::new(usage, span));
        }
        let target = match name.to_string().as_str() {
            "context" => &mut document.context,
            "scope" => &mut document.scope,
            "slots" => &mut document.slots,
            "styles" => &mut document.styles,
            _ => {
                return Err(Error::new(
                    format!("Unknown template directive @{name}"),
                    name.span(),
                ));
            }
        };
        if target.replace(g.stream()).is_some() {
            return Err(Error::new("Duplicate template directive", span));
        }
        at += 4;
    }
    let file = markup::register_file(&source.display);
    document.nodes = markup::parse_tokens(tokens[at..].iter().cloned().collect(), file)?;
    Ok(document)
}
