pub(super) use super::super::class::{Class, SHEET_ORDER, add_rule, render_class};
pub(super) use super::super::module::classes_macro;
pub(super) use super::super::{animation, keyframes_symbol, media, parser, selector, values};
pub(super) use super::load::{TailwindDirectives, diagnostic, load};
use super::*;
pub(super) use crate::{
    Error, Result,
    codegen::Output,
    input::{Reader, Source},
};
pub(super) use proc_macro::{Span, TokenStream};
pub(super) use std::collections::BTreeMap;
/// Corps du module d'une feuille stricte : dépendances, fonctions de style des classes,
/// `@keyframes` et macro `__karbon_classes!`.
pub(crate) fn compile(
    root: TokenStream,
    source: Source,
    path: &str,
    span: Span,
) -> Result<TokenStream> {
    let mut reader = Reader::new(source);
    let mut rules = Vec::new();
    let mut tailwind = TailwindDirectives::default();
    load(&mut reader, path, span, &mut rules, &mut tailwind)?;
    if !tailwind.enabled && (!tailwind.theme.is_empty() || !tailwind.variants.is_empty()) {
        return Err(Error::new(
            "@theme and @custom-variant need @import \"tailwindcss\"; at the top of the stylesheet",
            span,
        ));
    }
    let mut vars = BTreeMap::new();
    for rule in &rules {
        if rule.selectors == [":root"] && rule.media.is_empty() {
            for d in &rule.declarations {
                if !d.name.starts_with("--") {
                    return Err(diagnostic(
                        &rule.source,
                        &rule.text,
                        d.at,
                        ":root currently accepts custom properties only",
                        span,
                    ));
                }
                vars.insert(d.name.clone(), d.value.clone());
            }
        }
    }
    if tailwind.enabled {
        // `@keyframes` de la feuille, utilisables par `animate-[nom_1s_…]`.
        for rule in &rules {
            if let Some((name, _)) = &rule.keyframes
                && let Some(len) = rule.text[rule.at..]
                    .find('{')
                    .and_then(|open| block_len(&rule.text[rule.at + open..]).map(|l| open + l))
            {
                tailwind
                    .keyframes
                    .push((name.clone(), rule.text[rule.at..rule.at + len].to_owned()));
            }
        }
    }
    let mut classes = BTreeMap::<String, Class>::new();
    let mut declaration_order = SHEET_ORDER;
    let mut animations = BTreeMap::new();
    for rule in &rules {
        if let Some((name, frames)) = &rule.keyframes {
            animations.insert(
                name.clone(),
                animation::frames_at(frames, &vars).map_err(|(at, m)| {
                    diagnostic(&rule.source, &rule.text, at.unwrap_or(rule.at), m, span)
                })?,
            );
        }
    }
    for rule in &rules {
        if rule.selectors == [":root"] {
            continue;
        }
        for selector in &rule.selectors {
            let mut selected = selector::parse(selector)
                .map_err(|m| diagnostic(&rule.source, &rule.text, rule.at, m, span))?;
            if !rule.media.is_empty() {
                let mut conditions = rule
                    .media
                    .iter()
                    .map(|q| media::condition(q))
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|m| diagnostic(&rule.source, &rule.text, rule.at, m, span))?;
                if let Some(p) = selected.predicate.take() {
                    conditions.push(p);
                }
                selected.predicate = Some(format!(
                    "__ui_root::ui::styling::StyleState::All(&[{}])",
                    conditions.join(",")
                ));
            }
            for name in selected.classes.iter().chain([&selected.class]) {
                let ident = parser::class_name(name)
                    .map_err(|m| diagnostic(&rule.source, &rule.text, rule.at, m, span))?;
                let existing = classes.entry(ident).or_insert_with(|| Class {
                    original: name.clone(),
                    ..Default::default()
                });
                if existing.original != *name {
                    return Err(diagnostic(
                        &rule.source,
                        &rule.text,
                        rule.at,
                        "Class names collide after conversion to Rust identifiers",
                        span,
                    ));
                }
            }
            let ident = parser::class_name(&selected.class).unwrap();
            let class = classes.get_mut(&ident).unwrap();
            add_rule(
                class,
                &selected,
                !rule.media.is_empty(),
                &rule.declarations,
                &vars,
                &mut declaration_order,
            )
            .map_err(|(at, m)| diagnostic(&rule.source, &rule.text, at, m, span))?;
        }
        // `@keyframes` inconnu : signalé à la position de la déclaration.
        for d in &rule.declarations {
            let value = values::resolve(&d.value, &vars).unwrap_or_default();
            if let Some(name) = animation::names(&d.name, &value)
                .into_iter()
                .find(|n| !animations.contains_key(n))
            {
                return Err(diagnostic(
                    &rule.source,
                    &rule.text,
                    d.at,
                    format!("Unknown @keyframes '{name}'"),
                    span,
                ));
            }
        }
    }
    let mut out = Output::new(root.clone());
    let literals = Output::new(root);
    let mut body = reader
        .dependencies
        .iter()
        .map(|p| crate::input::files::dependency(p))
        .collect::<String>();
    let meta = sheet_meta(&classes, &vars, &tailwind);
    body += &root_variables(&rules, &vars, span)?;
    for (name, class) in classes {
        let symbol = parser::class_symbol(&class.original).map_err(|m| Error::new(m, span))?;
        let display_kind = class.display_kind.unwrap_or(-1);
        body += &render_class(&name, class, &animations, &mut out, &literals, span)?;
        body += &format!("#[allow(unused_imports)] pub use self::{name} as {symbol};");
        // `display` de base, lu par les templates : les éléments en ligne d'un conteneur
        // flex ou grille deviennent des éléments flex séparés.
        let display = symbol.replacen("__class_", "__display_", 1);
        body += &format!(
            "#[allow(dead_code,non_upper_case_globals)] pub const {display}:i8={display_kind};"
        );
    }
    // Animations nommées, utilisables par data-exit="nom" dans les templates.
    let mut keyframes = String::new();
    for (name, frames) in &animations {
        let symbol = keyframes_symbol(name).map_err(|m| Error::new(m, span))?;
        keyframes += &format!(
            "#[allow(dead_code)] pub const {symbol}:&[__ui_root::ui::animation::Keyframe]={frames};"
        );
    }
    body += &format!("#[allow(dead_code,non_upper_case_globals)] pub mod keyframes{{{keyframes}}}");
    // Résolution des classes littérales des templates (`class="…"`) : classes de la
    // feuille d'abord, puis utilitaires Tailwind si la feuille les active.
    body += &classes_macro(&meta);
    out.finish(&body)
}
/// Fonction `root_variables(dark, high_contrast)` du module généré : les variables `:root`
/// des blocs `@media (prefers-color-scheme: …)` / `(prefers-contrast: …)` de la feuille,
/// pour ce thème et ce contraste. Rien si la feuille n'en déclare pas.
pub(super) fn root_variables(
    rules: &[load::Rule],
    vars: &BTreeMap<String, String>,
    span: Span,
) -> Result<String> {
    const VARIABLES: &str = "__ui_root::ui::styling::CssVariables";
    let mut blocks = String::new();
    for rule in rules
        .iter()
        .filter(|r| r.selectors == [":root"] && !r.media.is_empty())
    {
        let fail = |m: String| diagnostic(&rule.source, &rule.text, rule.at, m, span);
        let mut wanted = (None, None);
        for query in &rule.media {
            let (dark, high) = media::color_preferences(query).map_err(fail)?;
            wanted = (wanted.0.or(dark), wanted.1.or(high));
        }
        let mut statements = String::new();
        for d in &rule.declarations {
            if !d.name.starts_with("--") {
                return Err(fail(
                    ":root currently accepts custom properties only".into(),
                ));
            }
            let code = values::resolve(&d.value, vars)
                .and_then(|value| values::custom_value(&value))
                .map_err(|m| diagnostic(&rule.source, &rule.text, d.at, m, span))?;
            statements += &format!("__ui_vars.set({:?},{code});", d.name);
        }
        blocks += &format!(
            "if {}{{{statements}}}",
            media::color_test(wanted.0, wanted.1)
        );
    }
    if blocks.is_empty() {
        return Ok(String::new());
    }
    Ok(format!(
        "/// Custom properties of the stylesheet's conditional `:root` blocks for this colour scheme and contrast, for [`TemplateContext::with_variables`].\n#[allow(dead_code)] pub fn root_variables(dark:bool,high_contrast:bool)->&'static {VARIABLES}{{static CACHE:[::std::sync::OnceLock<{VARIABLES}>;4]=[const{{::std::sync::OnceLock::new()}};4];CACHE[usize::from(dark)*2+usize::from(high_contrast)].get_or_init(||{{let __ui_scheme=(dark,high_contrast);let mut __ui_vars={VARIABLES}::default();{blocks}__ui_vars}})}}"
    ))
}
/// Longueur d'un bloc `{…}` (accolade fermante comprise), chaînes ignorées.
pub(super) fn block_len(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    for (i, c) in text.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}
/// Description d'une feuille transmise aux templates par la macro `__karbon_classes`
/// du module généré : classes, variables `:root` et directives Tailwind.
pub(super) fn sheet_meta(
    classes: &BTreeMap<String, Class>,
    vars: &BTreeMap<String, String>,
    tailwind: &TailwindDirectives,
) -> String {
    let line = |value: &str| value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut meta = format!("karbon-sheet 1\ntailwind {}\n", u8::from(tailwind.enabled));
    // Une règle lit une balise ou un attribut (`.liste > p.note`) : les templates
    // enregistrent ces faits sur leurs éléments.
    let reads_facts =
        |p: &str| p.contains("StyleState::Tag(") || p.contains("StyleState::Attribute(");
    if classes.values().any(|c| {
        c.state_props
            .iter()
            .chain(&c.child_props)
            .chain(c.pseudos.values().flat_map(|p| &p.state_props))
            .any(|(state, _, _)| reads_facts(state))
    }) {
        meta += "facts 1\n";
    }
    for class in classes.values() {
        meta += &format!("class {}\n", class.original);
    }
    for (name, value) in vars {
        meta += &format!("var {name}\t{}\n", line(value));
    }
    for (name, value) in &tailwind.theme {
        meta += &format!("theme {name}\t{}\n", line(value));
    }
    for (name, value) in &tailwind.keyframes {
        meta += &format!("keyframes {name}\t{}\n", line(value));
    }
    for (name, value) in &tailwind.variants {
        meta += &format!("variant {name}\t{}\n", line(value));
    }
    meta
}
