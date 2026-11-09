//! Lecture d'une feuille stricte et de ses `@import` : règles à plat (requêtes
//! `@media`/`@container` imbriquées comprises), `@keyframes` et directives Tailwind.
use crate::css::{media, parser};
use crate::{Error, Result, input::Reader};
use proc_macro::Span;
use std::sync::Arc;
pub(super) struct Rule {
    pub(super) media: Vec<String>,
    pub(super) keyframes: Option<(String, Vec<parser::Item>)>,
    pub(super) selectors: Vec<String>,
    pub(super) declarations: Vec<parser::Declaration>,
    pub(super) source: String,
    pub(super) text: Arc<str>,
    pub(super) at: usize,
}
pub(super) fn diagnostic(
    file: &str,
    text: &str,
    at: usize,
    message: impl std::fmt::Display,
    span: Span,
) -> Error {
    let prefix = &text[..at.min(text.len())];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    Error::new(format!("{file}:{line}:{column}: {message}"), span)
}
/// Directives Tailwind d'une feuille (`@import "tailwindcss"`, `@theme`,
/// `@custom-variant`), transmises aux templates qui l'utilisent.
#[derive(Default)]
pub(super) struct TailwindDirectives {
    pub(super) enabled: bool,
    pub(super) theme: Vec<(String, String)>,
    pub(super) keyframes: Vec<(String, String)>,
    pub(super) variants: Vec<(String, String)>,
}
pub(super) fn load(
    reader: &mut Reader,
    path: &str,
    span: Span,
    rules: &mut Vec<Rule>,
    tailwind: &mut TailwindDirectives,
) -> Result<()> {
    let (text, source, absolute) = reader.text(path, span)?;
    let parsed = parser::parse(&text)
        .map_err(|e| diagnostic(&source.display, &text, e.at, e.message, span))?;
    let text: Arc<str> = text.into();
    reader.stack.push(absolute);
    let mut body = false;
    for item in parsed {
        match item {
            parser::Item::Media { query, items, at } => {
                body = true;
                append_media(items, vec![query], &source.display, &text, at, rules)
                    .map_err(|m| diagnostic(&source.display, &text, at, m, span))?;
            }
            parser::Item::Keyframes { name, frames, at } => {
                body = true;
                rules.push(Rule {
                    media: Vec::new(),
                    keyframes: Some((name, frames)),
                    selectors: Vec::new(),
                    declarations: Vec::new(),
                    source: source.display.clone(),
                    text: text.clone(),
                    at,
                });
            }
            parser::Item::Import { path, at } => {
                if body {
                    return Err(diagnostic(
                        &source.display,
                        &text,
                        at,
                        "@import must precede style rules",
                        span,
                    ));
                }
                if path == "tailwindcss" {
                    // Utilitaires Tailwind compilés à la demande par les templates.
                    tailwind.enabled = true;
                    continue;
                }
                if path.starts_with("tailwindcss/") {
                    return Err(diagnostic(
                        &source.display,
                        &text,
                        at,
                        "Use @import \"tailwindcss\"; the partial Tailwind imports are not implemented",
                        span,
                    ));
                }
                reader.source = source.clone();
                load(reader, &path, span, rules, tailwind)?;
            }
            parser::Item::Theme {
                declarations,
                keyframes,
                ..
            } => {
                tailwind
                    .theme
                    .extend(declarations.into_iter().map(|d| (d.name, d.value)));
                tailwind.keyframes.extend(keyframes);
            }
            parser::Item::CustomVariant { name, selector, .. } => {
                tailwind.variants.push((name, selector));
            }
            parser::Item::Rule {
                selectors,
                declarations,
                at,
            } => {
                body = true;
                rules.push(Rule {
                    media: Vec::new(),
                    keyframes: None,
                    selectors,
                    declarations,
                    source: source.display.clone(),
                    text: text.clone(),
                    at,
                });
                if rules.len() > 4096 {
                    return Err(Error::new("Stylesheet exceeds 4096 rules", span));
                }
            }
        }
    }
    reader.stack.pop();
    Ok(())
}
pub(super) fn append_media(
    items: Vec<parser::Item>,
    queries: Vec<String>,
    source: &str,
    text: &Arc<str>,
    at: usize,
    rules: &mut Vec<Rule>,
) -> std::result::Result<(), String> {
    for query in &queries {
        media::condition(query)?;
    }
    for item in items {
        match item {
            parser::Item::Rule {
                selectors,
                declarations,
                at,
            } => {
                rules.push(Rule {
                    media: queries.clone(),
                    keyframes: None,
                    selectors,
                    declarations,
                    source: source.into(),
                    text: Arc::clone(text),
                    at,
                });
            }
            parser::Item::Media { query, items, at } => {
                let mut queries = queries.clone();
                queries.push(query);
                append_media(items, queries, source, text, at, rules)?;
            }
            _ => {
                return Err(format!(
                    "Imports, keyframe definitions and Tailwind directives are not allowed inside native media or container queries (offset {at})"
                ));
            }
        }
    }
    Ok(())
}
