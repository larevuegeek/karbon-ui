//! Point d'entrée de `css_file!` : arguments de la macro, puis module généré pour une
//! feuille stricte ([`super::sheet`]) ou de framework ([`super::framework`]).
use super::{framework, keyframes_symbol, sheet};
use crate::{
    Error, Result,
    codegen::Output,
    input::{Cursor, Reader, Source},
};
use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree};

/// `css_file!(pub mod nom, "feuille.css" [, framework] [, report [= "all"]])`.
pub fn compile(root: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let source = Source::call(&input);
    let mut c = Cursor::new(input, &source);
    let mut visibility = String::new();
    let mut keyword = c.ident()?;
    if keyword == "pub" {
        visibility = "pub ".into();
        if let Some(TokenTree::Group(g)) = c.tokens.get(c.at)
            && g.delimiter() == Delimiter::Parenthesis
        {
            visibility += &format!("{g} ");
            c.at += 1;
        }
        keyword = c.ident()?;
    }
    if keyword != "mod" {
        return Err(c.error("Use css_file!(pub mod styles, \"styles.css\")"));
    }
    let module = match c.next()? {
        TokenTree::Ident(i) => i,
        _ => return Err(c.error("Expected a module name")),
    };
    c.expect(',')?;
    let span = c.span();
    let path = c.string()?;
    // Options : `framework`, `report`, `report = "all"`.
    let mut framework_mode = false;
    let mut report = None;
    while c.peek(',') {
        c.at += 1;
        if c.done() {
            break;
        }
        let option = c.ident()?;
        match option.as_str() {
            "framework" => framework_mode = true,
            "report" => {
                report = Some(false);
                if c.peek('=') {
                    c.at += 1;
                    match c.string()?.as_str() {
                        "all" => report = Some(true),
                        "summary" => report = Some(false),
                        _ => {
                            return Err(
                                c.error("Use report, report = \"summary\" or report = \"all\"")
                            );
                        }
                    }
                }
            }
            _ => {
                return Err(c.error("Unknown css_file! option; expected framework or report"));
            }
        }
    }
    if !c.done() {
        return Err(c.error("Unexpected css_file! argument"));
    }
    if report.is_some() && !framework_mode {
        return Err(Error::new(
            "report is available with framework stylesheets: css_file!(mod bs, \"bootstrap.css\", framework, report)",
            span,
        ));
    }
    if framework_mode {
        let body = framework_module(root, source, &path, report, span)?;
        return Ok(module_tokens(&visibility, module, body));
    }
    let body = sheet::compile(root, source, &path, span)?;
    Ok(module_tokens(&visibility, module, body))
}
/// `{visibility}mod {module} { body }`.
fn module_tokens(visibility: &str, module: proc_macro::Ident, body: TokenStream) -> TokenStream {
    let mut result: TokenStream = format!("{visibility}mod").parse().unwrap();
    result.extend([
        TokenTree::Ident(module),
        TokenTree::Group(Group::new(Delimiter::Brace, body)),
    ]);
    result
}
/// Macro `__karbon_classes!` du module généré : rappelle `__compile_classes!` avec la
/// description `meta` de la feuille et les listes de classes du template.
pub(super) fn classes_macro(meta: &str) -> String {
    format!(
        "#[allow(unused_macros)] macro_rules! __karbon_classes{{(($($r:tt)*) $($t:tt)*)=>{{$($r)*::__compile_classes!{{($($r)*) {meta:?} $($t)*}}}};}}#[allow(unused_imports)] pub(crate) use __karbon_classes;"
    )
}
/// Module d'une feuille de framework : variables globales, rapport de compatibilité,
/// animations et macro de résolution des templates (cascade compilée par élément).
fn framework_module(
    root: TokenStream,
    source: Source,
    path: &str,
    report: Option<bool>,
    span: Span,
) -> Result<TokenStream> {
    let mut reader = Reader::new(source);
    let (_, source, absolute) = reader.text(path, span)?;
    let absolute = absolute.to_string_lossy().into_owned();
    let sheet = framework::load(&absolute).map_err(|m| Error::new(m, span))?;
    let r = &sheet.report;
    if let Some(all) = report {
        let mut text = sheet
            .report
            .summary(&source.display, if all { usize::MAX } else { 25 });
        if all {
            for (at, category, snippet) in &sheet.report.details {
                let (line, column) = framework::syntax::line_column(&sheet.text, *at);
                text += &format!(
                    "  {}:{line}:{column}: {category}: {snippet}\n",
                    source.display
                );
            }
        }
        eprint!("{text}");
    }
    let mut body = reader
        .dependencies
        .iter()
        .map(|p| crate::input::files::dependency(p))
        .collect::<String>();
    body += &format!(
        "#[doc(hidden)] pub static __KARBON_REGISTRY:__ui_root::ui::styling::tokens::Registry={};",
        sheet.registry_code()
    );
    body += &format!(
        "/// Compatibility of the stylesheet with the native engine, measured at compile time.\n#[allow(dead_code)] pub const COMPATIBILITY:__ui_root::ui::styling::tokens::Compatibility=__ui_root::ui::styling::tokens::Compatibility{{rules:{},rules_applied:{},rules_full:{},selectors:{},selectors_accepted:{},declarations:{},declarations_accepted:{},declarations_dynamic:{},at_rules:{},at_rules_accepted:{},rules_inactive:{}}};",
        r.rules,
        r.rules_applied,
        r.rules_full,
        r.selectors,
        r.selectors_accepted,
        r.declarations,
        r.declarations_accepted,
        r.declarations_dynamic,
        r.at_rules,
        r.at_rules_accepted,
        r.rules_inactive
    );
    body += &format!(
        "/// Compatibility summary and most frequent skipped categories.\n#[allow(dead_code)] pub const REPORT:&str={:?};",
        sheet.report.summary(&source.display, 25)
    );
    let mut keyframes = String::new();
    for (name, frames) in sheet.animations() {
        if let Ok(symbol) = keyframes_symbol(&name) {
            keyframes += &format!(
                "#[allow(dead_code)] pub const {symbol}:&[__ui_root::ui::animation::Keyframe]={frames};"
            );
        }
    }
    body += &format!("#[allow(dead_code,non_upper_case_globals)] pub mod keyframes{{{keyframes}}}");
    let meta = format!("karbon-sheet 1\ntailwind 0\nframework {absolute}\n");
    body += &classes_macro(&meta);
    Output::new(root).finish(&body)
}
