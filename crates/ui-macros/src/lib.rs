//! Compiler of the `karbon-ui` templates and stylesheets, deliberately limited to
//! `proc_macro` and `std`.
//!
//! This crate is an implementation detail of
//! [`karbon-ui`](https://crates.io/crates/karbon-ui): use its `ui!`, `ui_file!`,
//! `styles!` and `css_file!` macros instead of the entry points below. Each of them
//! expects the path of the `karbon_ui` crate between parentheses as its first token,
//! which those wrappers provide; their arguments may change in any release.
// Organisation (voir `docs/compiler.md`) : `input` (arguments et fichiers), `codegen`
// (assemblage du code produit), `template` (`ui!`, `ui_file!`), `styles` (`styles!`),
// `css` (`css_file!`), `classes` (`__karbon_classes!`), `tailwind`, `bootstrap`, `svg`.
extern crate proc_macro;
mod bootstrap;
mod classes;
mod codegen;
mod css;
mod error;
mod input;
mod styles;
mod svg;
mod tailwind;
mod template;
use error::{Error, Result};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

/// Sépare le chemin de la crate `karbon_ui` (premier groupe entre parenthèses, fourni
/// par les macros publiques) du reste de l'appel.
fn root(input: TokenStream) -> Result<(TokenStream, TokenStream)> {
    let mut tokens = input.into_iter();
    match tokens.next() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
            Ok((g.stream(), tokens.collect()))
        }
        _ => Err(Error::new(
            "Use the public ui!, ui_file!, styles! or css_file! macro",
            Span::call_site(),
        )),
    }
}
/// Engine of `karbon_ui::ui!`: compiles the markup written in the call into `Element`
/// constructors.
#[proc_macro]
pub fn compile_ui(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| template::compile_inline(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Engine of `karbon_ui::ui_file!`: reads a `.ui` file relative to the calling source
/// file and compiles it like `ui!`.
#[proc_macro]
pub fn compile_ui_file(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| template::compile_file(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Engine of `karbon_ui::styles!`: compiles style recipes into typed style functions.
#[proc_macro]
pub fn compile_styles(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| styles::compile(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Engine of `karbon_ui::css_file!`: compiles a local stylesheet into a module of typed
/// style functions.
#[proc_macro]
pub fn compile_css_file(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| css::compile(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Engine of the `__karbon_classes!` macro generated in every `css_file!` module:
/// resolves the literal class lists of a template against the stylesheet, then against
/// the Tailwind utilities when the stylesheet imports `tailwindcss`.
#[proc_macro]
pub fn compile_classes(input: TokenStream) -> TokenStream {
    classes::compile(input).unwrap_or_else(Error::tokens)
}
