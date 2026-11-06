//! Compilateur des templates natifs, volontairement limité à `proc_macro` et `std`.
//!
//! Détail d'implémentation de `karbon-ui` : utiliser ses macros `ui!`, `ui_file!`,
//! `styles!` et `css_file!` plutôt que cette crate. Chaque macro attend en tête le
//! chemin de la crate `karbon_ui` entre parenthèses, fourni par ces enveloppes.
//!
//! Organisation (voir `README.md`) : `input` (arguments et fichiers), `codegen`
//! (assemblage du code produit), `template` (`ui!`, `ui_file!`), `styles` (`styles!`),
//! `css` (`css_file!`), `classes` (`__karbon_classes!`), `tailwind`, `bootstrap`, `svg`.
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
            "Use the public ui!, ui_file! or styles! macro",
            Span::call_site(),
        )),
    }
}
/// Moteur de `karbon_ui::ui!` : compile le balisage écrit dans l'appel en constructeurs
/// d'`Element`.
#[proc_macro]
pub fn compile_ui(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| template::compile_inline(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Moteur de `karbon_ui::ui_file!` : lit un fichier `.ui` relatif au fichier source de
/// l'appel et le compile comme `ui!`.
#[proc_macro]
pub fn compile_ui_file(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| template::compile_file(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Moteur de `karbon_ui::styles!` : compile des recettes de styles en fonctions de
/// style typées.
#[proc_macro]
pub fn compile_styles(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| styles::compile(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Moteur de `karbon_ui::css_file!` : compile une feuille locale en module de fonctions
/// de style typées.
#[proc_macro]
pub fn compile_css_file(input: TokenStream) -> TokenStream {
    root(input)
        .and_then(|(root, input)| css::compile(root, input))
        .unwrap_or_else(Error::tokens)
}
/// Moteur de la macro `__karbon_classes!` générée dans chaque module `css_file!` :
/// résout les listes de classes littérales d'un template contre la feuille, puis
/// contre les utilitaires Tailwind quand la feuille importe `tailwindcss`.
#[proc_macro]
pub fn compile_classes(input: TokenStream) -> TokenStream {
    classes::compile(input).unwrap_or_else(Error::tokens)
}
