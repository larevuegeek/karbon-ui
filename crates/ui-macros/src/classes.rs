//! Résolution des classes littérales des templates (`class="…"`, `class:nom={…}`).
//!
//! `css_file!` génère dans chaque module de styles une macro `__karbon_classes!` qui
//! rappelle ce compilateur avec la description de la feuille (classes, variables
//! `:root`, directives Tailwind). Le template lui confie ses listes de classes ; chaque
//! liste devient des fonctions de style locales `__ui_cls_<liste>_<i>` appliquées dans
//! l'ordre : utilitaires Tailwind fusionnés (couche `utilities`), classes de la feuille
//! dans l'ordre de l'attribut, utilitaires `!important`, puis fonctions neutres.
use crate::{Error, Result, codegen::Output, css, input::files, tailwind, template::html};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Rc,
};

/// Feuille décrite par `__karbon_classes!`.
pub(crate) struct Sheet {
    pub classes: BTreeSet<String>,
    pub vars: BTreeMap<String, String>,
    pub tailwind: Option<tailwind::Config>,
    /// Feuille de framework (`css_file!(…, framework)`) : cascade compilée par élément.
    pub framework: Option<Rc<css::framework::Sheet>>,
    /// Une règle de la feuille lit les balises ou les attributs des éléments.
    pub facts: bool,
}
impl Sheet {
    /// Lit la description produite par `css_file!` (une entrée par ligne).
    pub(crate) fn parse(meta: &str) -> std::result::Result<Self, String> {
        let mut lines = meta.lines();
        if lines.next() != Some("karbon-sheet 1") {
            return Err("Stylesheet module compiled by another version of karbon-ui".into());
        }
        let mut enabled = false;
        let mut classes = BTreeSet::new();
        let mut vars = BTreeMap::new();
        let (mut theme, mut keyframes, mut variants) = (Vec::new(), Vec::new(), Vec::new());
        let mut framework = None;
        let mut facts = false;
        for line in lines {
            let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
            let pair = || {
                let (name, value) = rest.split_once('\t').unwrap_or((rest, ""));
                (name.to_owned(), value.to_owned())
            };
            match kind {
                "tailwind" => enabled = rest == "1",
                "facts" => facts = rest == "1",
                "class" => {
                    classes.insert(rest.to_owned());
                }
                "var" => {
                    let (name, value) = pair();
                    vars.insert(name, value);
                }
                "theme" => theme.push(pair()),
                "keyframes" => keyframes.push(pair()),
                "variant" => variants.push(pair()),
                "framework" => framework = Some(css::framework::load(rest)?),
                _ => {}
            }
        }
        let tailwind = if enabled {
            Some(tailwind::Config::new(&theme, &keyframes, &variants)?)
        } else {
            None
        };
        if let Some(sheet) = &framework {
            classes.extend(sheet.classes.iter().cloned());
        }
        Ok(Self {
            classes,
            vars,
            tailwind,
            framework,
            facts,
        })
    }
}
thread_local! {
    /// Feuilles déjà lues pendant la compilation de la crate (une par module de styles).
    static SHEETS: RefCell<HashMap<String, Rc<Sheet>>> = RefCell::new(HashMap::new());
}
fn sheet(meta: &str) -> std::result::Result<Rc<Sheet>, String> {
    if let Some(sheet) = SHEETS.with(|s| s.borrow().get(meta).cloned())
        && fresh(&sheet, meta)
    {
        return Ok(sheet);
    }
    let sheet = Rc::new(Sheet::parse(meta)?);
    SHEETS.with(|s| s.borrow_mut().insert(meta.to_owned(), sheet.clone()));
    Ok(sheet)
}
/// `false` quand la feuille de framework en cache a changé sur le disque : la description
/// ne nomme que son chemin, et un processus qui dure (rust-analyzer) garderait sinon la
/// cascade de la première lecture. `framework::load` compare taille et date du fichier.
fn fresh(sheet: &Sheet, meta: &str) -> bool {
    let path = meta
        .lines()
        .find_map(|line| line.strip_prefix("framework "));
    match (&sheet.framework, path) {
        (Some(cached), Some(path)) => {
            css::framework::load(path).is_ok_and(|now| Rc::ptr_eq(&now, cached))
        }
        _ => true,
    }
}
/// Littéral chaîne, éventuellement dans un groupe invisible (valeur transmise par
/// macro), avec son span.
fn string(token: Option<TokenTree>) -> Result<(String, Span)> {
    match token {
        Some(TokenTree::Literal(l)) => files::string(&l.to_string())
            .map(|s| (s, l.span()))
            .map_err(|m| Error::new(m, l.span())),
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::None => {
            string(g.stream().into_iter().next())
        }
        Some(t) => Err(Error::new("Expected a string literal", t.span())),
        None => Err(Error::new(
            "Unexpected end of __karbon_classes! input",
            Span::call_site(),
        )),
    }
}
/// Rôle d'une position d'une liste compilée.
enum Slot {
    Sheet(String),
    Merged(Box<css::UtilityClass>),
    Neutral,
}
/// Classes de la feuille et utilitaires d'une liste, dans l'ordre d'application.
pub(crate) struct Resolved {
    pub sheet: Vec<String>,
    pub normal: Option<css::UtilityClass>,
    pub important: Option<css::UtilityClass>,
}
/// Résout une liste de classes ; l'erreur est un message sans position.
pub(crate) fn resolve(text: &str, sheet: &Sheet) -> std::result::Result<Resolved, String> {
    let mut sheet_classes = Vec::new();
    let mut utilities = Vec::new();
    for token in text.split_whitespace() {
        if sheet.classes.contains(token) {
            sheet_classes.push(token.to_owned());
        } else if sheet.tailwind.is_some() {
            utilities.push(token);
        } else {
            return Err(unknown(token, sheet));
        }
    }
    let mut resolved = Resolved {
        sheet: sheet_classes,
        normal: None,
        important: None,
    };
    if let Some(config) = &sheet.tailwind
        && !utilities.is_empty()
    {
        let set = tailwind::compile_set(&utilities, config).map_err(|(index, message)| {
            let token = utilities[index];
            let mut message = format!("Unknown class '{token}': {message}");
            let names = sheet.classes.iter().map(String::as_str).collect::<Vec<_>>();
            if let Some(s) = html::suggest(token, &names) {
                message += &format!(" (stylesheet class '{s}'?)");
            }
            message
        })?;
        // Une fonction par couche présente (au plus une par utilitaire) ; la première
        // porte les noms des classes de l'élément.
        let mut markers = utilities
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>();
        if !set.normal.is_empty() || set.important.is_empty() {
            let normal =
                css::utility_class(&set.normal, std::mem::take(&mut markers), &sheet.vars)?;
            resolved.normal = Some(normal);
        }
        if !set.important.is_empty() {
            resolved.important = Some(css::utility_class(&set.important, markers, &sheet.vars)?);
        }
    }
    Ok(resolved)
}
/// Faits statiques d'un élément décrit par le générateur de templates :
/// `balise ␝ classes ␝ classes dynamiques ␝ attributs (nom␟valeur␞…) ␝ attributs dynamiques`.
fn element_facts(signature: &str) -> css::framework::selectors::Facts {
    let mut parts = signature.split('\u{1d}');
    let own = parts.by_ref().take(5).collect::<Vec<_>>().join("\u{1d}");
    let leaf = parts.next() == Some("true");
    let chain = parts.next().unwrap_or("");
    let mut facts = own_facts(&own);
    facts.leaf = leaf;
    let ancestors = chain
        .split('\u{1c}')
        .filter(|a| !a.is_empty())
        .map(|a| own_facts(&a.replace('\u{1b}', "\u{1d}")))
        .collect::<Vec<_>>();
    facts.parent_known = !ancestors.is_empty();
    // Chaque ancêtre connaît son parent s'il en a un dans la chaîne.
    let count = ancestors.len();
    facts.ancestors = ancestors
        .into_iter()
        .enumerate()
        .map(|(i, mut a)| {
            a.parent_known = i + 1 < count;
            a
        })
        .collect();
    facts
}
/// Faits propres : `balise ␝ classes ␝ classes dynamiques ␝ attributs (nom␟valeur␞…) ␝
/// attributs dynamiques`.
fn own_facts(text: &str) -> css::framework::selectors::Facts {
    let mut parts = text.split('\u{1d}');
    let mut next = || parts.next().unwrap_or("");
    let tag = next().to_owned();
    let classes = next().split_whitespace().map(str::to_owned).collect();
    let dynamic_classes = next().split_whitespace().map(str::to_owned).collect();
    let attrs = next()
        .split('\u{1e}')
        .filter(|a| !a.is_empty())
        .map(|a| {
            let (k, v) = a.split_once('\u{1f}').unwrap_or((a, ""));
            (k.to_owned(), v.to_owned())
        })
        .collect();
    let dynamic_attrs = next().split_whitespace().map(str::to_owned).collect();
    css::framework::selectors::Facts {
        tag,
        classes,
        dynamic_classes,
        attrs,
        dynamic_attrs,
        ..Default::default()
    }
}
/// `display` d'une liste de classes d'une feuille de framework (sur un `<div>`), lu par
/// le générateur pour savoir si un conteneur est flex ou grille.
fn framework_display(sheet: &Sheet, classes: &[String]) -> i8 {
    let Some(framework) = &sheet.framework else {
        return -1;
    };
    let facts = css::framework::selectors::Facts {
        tag: "div".into(),
        classes: classes.iter().cloned().collect(),
        ..Default::default()
    };
    let mut class = css::framework_class();
    css::framework::element::compile(framework, &facts, "", &mut class, &mut Vec::new());
    css::display_kind(&class)
}
/// Diagnostic d'une classe absente d'une feuille sans Tailwind.
fn unknown(token: &str, sheet: &Sheet) -> String {
    let names = sheet.classes.iter().map(String::as_str).collect::<Vec<_>>();
    let mut message = format!("Unknown class '{token}': the stylesheet does not declare .{token}");
    if let Some(s) = html::suggest(token, &names) {
        message += &format!("; did you mean '{s}'?");
    } else if let Ok(config) = tailwind::Config::new(&[], &[], &[])
        && tailwind::compile_set(&[token], &config).is_ok()
    {
        message += "; it is a Tailwind utility: add @import \"tailwindcss\"; at the top of the stylesheet to enable them";
    }
    message
}
/// Backend de `__karbon_classes!` :
/// `(racine) "description" (module) id "préfixe" "classes" id "préfixe" "classes"…`.
pub fn compile(input: TokenStream) -> Result<TokenStream> {
    let mut tokens = input.into_iter();
    let group = |t: Option<TokenTree>| match t {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => Ok(g.stream()),
        _ => Err(Error::new(
            "Use class=\"…\" in ui!/ui_file! templates",
            Span::call_site(),
        )),
    };
    let root = group(tokens.next())?;
    let (meta, meta_span) = string(tokens.next())?;
    let module = group(tokens.next())?;
    let sheet = sheet(&meta).map_err(|m| Error::new(m, meta_span))?;
    let mut out = Output::new(root.clone());
    let literals = Output::new(root);
    let module = out.raw(&module);
    let mut code = String::new();
    // Diagnostics des listes refusées ; leurs fonctions restent neutres pour éviter
    // les erreurs en cascade.
    let mut errors = TokenStream::new();
    // Animations de la feuille de framework, compilées une fois par appel.
    let animations = sheet
        .framework
        .as_ref()
        .map(|f| f.animations())
        .unwrap_or_default();
    while let Some(id) = tokens.next() {
        if let TokenTree::Ident(kind) = &id
            && kind.to_string() == "elem"
        {
            // Élément d'un template : `elem <n> "balise\x1dclasses\x1d…"`.
            let id = match tokens.next() {
                Some(TokenTree::Literal(l)) => l.to_string(),
                _ => return Err(Error::new("Expected an element number", kind.span())),
            };
            let (signature, span) = string(tokens.next())?;
            let name = format!("__ui_elm_{id}");
            // `data-reveal=".classe"` : minutage de la transition de hauteur de la classe ;
            // `data-bs-reveal` (cibles `collapse` réécrites) : de même, sans erreur quand la
            // feuille n'en déclare pas.
            let own = own_facts(&signature);
            let (reveal, strict) = match (
                own.attrs
                    .get("data-reveal")
                    .and_then(|v| v.trim().strip_prefix('.')),
                own.attrs.get("data-bs-reveal"),
            ) {
                (Some(class), _) => (Some(class.to_owned()), true),
                (None, Some(class)) => (Some(class.trim().to_owned()), false),
                (None, None) => (None, false),
            };
            let mut reveal_error = |message: String| {
                if strict {
                    errors.extend(Error::new(message, span).tokens());
                    errors.extend(";".parse::<TokenStream>().unwrap());
                }
            };
            if let Some(class) = &reveal
                && sheet.framework.is_none()
            {
                reveal_error(format!(
                    "data-reveal=\".{class}\" reads the height transition of a class: it needs a framework stylesheet (css_file!(…, framework))"
                ));
            }
            match &sheet.framework {
                Some(framework) => {
                    let facts = element_facts(&signature);
                    let mut class = css::framework_class();
                    let mut notes = Vec::new();
                    let generated = css::framework::element::compile(
                        framework, &facts, &module, &mut class, &mut notes,
                    );
                    if let Some(name) = &reveal
                        && !css::framework::element::reveal(framework, name, &mut class)
                    {
                        reveal_error(format!(
                            "data-reveal=\".{name}\": the stylesheet declares no `transition` of height for .{name}"
                        ));
                    }
                    code += &format!(
                        "#[allow(dead_code,non_upper_case_globals)] const __ui_elmg_{id}:bool={generated};"
                    );
                    // Animation dont une étape extrême omet l'opacité : celle de l'élément.
                    let own = css::framework_animated_opacity(&class)
                        .filter(|o| o.trim() != "1")
                        .map(|o| framework.animations_with(o.trim()));
                    let frames = own.as_ref().unwrap_or(&animations);
                    css::framework_drop_animated_opacity(&mut class, frames);
                    code += &css::render_framework_class(
                        &name, class, frames, &mut out, &literals, span,
                    )?;
                }
                // Une liste de ce template lit les balises ou les attributs (`data-[x]:`,
                // `[a+&]:`, `.carte > div`) : chaque élément les enregistre.
                None if sheet.facts
                    || code.contains("StyleState::Tag(")
                    || code.contains("StyleState::Attribute(") =>
                {
                    let facts = element_facts(&signature);
                    let attrs = facts
                        .attrs
                        .iter()
                        .map(|(k, v)| format!("({k:?},{v:?})"))
                        .collect::<Vec<_>>()
                        .join(",");
                    code += &format!(
                        "#[allow(non_snake_case,dead_code)] fn {name}<R,W:__ui_root::ui::element::Widget+'static>(__ui_node:__ui_root::ui::template::Node<W>,_:&__ui_root::ui::template::TemplateContext<'_,R>)->__ui_root::ui::template::Node<W>{{__ui_node.html_facts({:?},&[{attrs}])}}#[allow(dead_code,non_upper_case_globals)] const __ui_elmg_{id}:bool=false;",
                        facts.tag
                    );
                }
                None => {
                    code += &format!(
                        "#[allow(unused_imports)] use __ui_root::ui::template::unstyled as {name};#[allow(dead_code,non_upper_case_globals)] const __ui_elmg_{id}:bool=false;"
                    );
                }
            }
            continue;
        }
        let id = match id {
            TokenTree::Literal(l) => l.to_string(),
            other => return Err(Error::new("Expected a class list number", other.span())),
        };
        let (prefix, _) = string(tokens.next())?;
        let (text, span) = string(tokens.next())?;
        let count = text.split_whitespace().count();
        let resolved = match resolve(&text, &sheet) {
            Ok(resolved) => resolved,
            Err(m) => {
                errors.extend(Error::new(format!("{prefix}{m}"), span).tokens());
                errors.extend(";".parse::<TokenStream>().unwrap());
                Resolved {
                    sheet: Vec::new(),
                    normal: None,
                    important: None,
                }
            }
        };
        let mut slots = Vec::new();
        if sheet.framework.is_some() {
            // La cascade est compilée par élément (`__ui_elm_*`) : les listes posent
            // seulement les noms de classes, lus par les sélecteurs à l'exécution.
            for (i, class) in resolved.sheet.iter().enumerate() {
                let display = if i == 0 {
                    framework_display(&sheet, &resolved.sheet)
                } else {
                    -1
                };
                code += &format!(
                    "#[allow(non_snake_case,unused_variables,dead_code)] fn __ui_cls_{id}_{i}<R,W:__ui_root::ui::element::Widget+'static>(__ui_node:__ui_root::ui::template::Node<W>,cx:&__ui_root::ui::template::TemplateContext<'_,R>)->__ui_root::ui::template::Node<W>{{__ui_node.css_class({class:?})}}#[allow(dead_code,non_upper_case_globals)] const __ui_clsd_{id}_{i}:i8={display};"
                );
            }
            for i in resolved.sheet.len()..count {
                code += &format!(
                    "#[allow(unused_imports)] use __ui_root::ui::template::unstyled as __ui_cls_{id}_{i};#[allow(dead_code,non_upper_case_globals)] const __ui_clsd_{id}_{i}:i8=-1;"
                );
            }
            continue;
        }
        if let Some(normal) = resolved.normal {
            slots.push(Slot::Merged(Box::new(normal)));
        }
        slots.extend(resolved.sheet.into_iter().map(Slot::Sheet));
        if let Some(important) = resolved.important {
            slots.push(Slot::Merged(Box::new(important)));
        }
        while slots.len() < count {
            slots.push(Slot::Neutral);
        }
        for (i, slot) in slots.into_iter().enumerate() {
            let name = format!("__ui_cls_{id}_{i}");
            let display = format!("__ui_clsd_{id}_{i}");
            match slot {
                Slot::Sheet(class) => {
                    let symbol = css::class_symbol(&class).map_err(|m| Error::new(m, span))?;
                    let shown = symbol.replacen("__class_", "__display_", 1);
                    code += &format!(
                        "#[allow(unused_imports)] use {module}::{symbol} as {name};#[allow(unused_imports)] use {module}::{shown} as {display};"
                    );
                }
                Slot::Merged(class) => {
                    let kind = class.display_kind();
                    code += &class.render(&name, &mut out, &literals, span)?;
                    code += &format!(
                        "#[allow(dead_code,non_upper_case_globals)] const {display}:i8={kind};"
                    );
                }
                Slot::Neutral => {
                    code += &format!(
                        "#[allow(unused_imports)] use __ui_root::ui::template::unstyled as {name};#[allow(dead_code,non_upper_case_globals)] const {display}:i8=-1;"
                    );
                }
            }
        }
    }
    let mut result = out.finish(&code)?;
    result.extend(errors);
    Ok(result)
}

#[cfg(test)]
mod tests {
    /// Une feuille de framework modifiée est relue : la cascade en cache suivait son
    /// chemin seul.
    #[test]
    fn a_modified_framework_sheet_is_read_again() {
        let path = std::env::temp_dir().join(format!("karbon-classes-{}.css", std::process::id()));
        let meta = format!("karbon-sheet 1\ntailwind 0\nframework {}\n", path.display());
        std::fs::write(&path, ".first { color: red; }").unwrap();
        assert!(super::sheet(&meta).unwrap().classes.contains("first"));
        std::fs::write(&path, ".second-name { color: blue; }").unwrap();
        let sheet = super::sheet(&meta).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(sheet.classes.contains("second-name") && !sheet.classes.contains("first"));
    }
}
