//! Point d'entrée de `icons_dir!` : un dossier d'icônes SVG (`<jeu>/<nom>.svg`, ou
//! `<nom>.svg` à la racine) devient un module avec `svg(cx, "jeu:nom")` et `NAMES`.
//! Chaque fichier est compilé comme `<svg src="…">` dans un template : mêmes contrôles,
//! mêmes diagnostics, et Cargo suit chaque fichier lu.
use crate::{
    Error, Result,
    codegen::Output,
    css::module_tokens,
    input::{Cursor, Source},
    template::{self, markup},
};
use proc_macro::{Span, TokenStream};
use std::path::{Path, PathBuf};

/// `icons_dir!(pub mod nom, "dossier")`.
pub fn compile(root: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let source = Source::call(&input);
    let mut c = Cursor::new(input, &source);
    let (visibility, module) = c.module_declaration("icons_dir!(pub mod icons, \"icons\")")?;
    c.expect(',')?;
    let span = c.span();
    let dir = c.string()?;
    if c.peek(',') {
        c.at += 1;
    }
    if !c.done() {
        return Err(c.error("Unexpected icons_dir! argument"));
    }
    let icons = scan(&source, &dir, span)?;
    if icons.is_empty() {
        return Err(Error::new(
            format!("{}: no .svg file in {dir:?}", source.display),
            span,
        ));
    }
    let mut out = Output::new(root.clone());
    let mut arms = String::new();
    for icon in &icons {
        let block = compile_icon(root.clone(), &source, icon, span)?;
        arms += &format!("{:?}=>{},", icon.name, out.value(&block));
    }
    let names = icons
        .iter()
        .map(|i| format!("{:?}", i.name))
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        "/// Names of the icons of the folder, sorted: `\"set:name\"` for `set/name.svg`, `\"name\"` for a file at its root.\n\
         #[allow(dead_code)] pub const NAMES:&[&str]=&[{names}];\n\
         /// The icon `name` compiled to an `<svg>` element drawn with `currentColor`, or `None` when the folder has no such file.\n\
         #[allow(dead_code)] pub fn svg<R>(cx:&__ui_root::ui::template::TemplateContext<'_,R>,name:&str)->Option<__ui_root::ui::element::Element>{{\
         Some(match name{{{arms}_=>return None,}})}}"
    );
    Ok(module_tokens(&visibility, module, out.finish(&body)?))
}

/// Icône trouvée : nom public et chemin relatif au dossier de l'appelant.
struct Icon {
    name: String,
    path: String,
}

/// Fichiers `.svg` du dossier (un niveau de jeux au plus), triés par nom ; les autres
/// fichiers (licences, notes) et les entrées cachées sont ignorés.
fn scan(source: &Source, dir: &str, span: Span) -> Result<Vec<Icon>> {
    let base = source.dir.join(dir);
    let error = |message: String| Error::new(format!("{}: {message}", source.display), span);
    let entries = |path: &Path| -> Result<Vec<(String, PathBuf, bool)>> {
        let read = std::fs::read_dir(path)
            .map_err(|e| error(format!("cannot read icon folder {}: {e}", path.display())))?;
        let mut list = Vec::new();
        for entry in read {
            let entry = entry.map_err(|e| error(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = entry
                .file_type()
                .map_err(|e| error(e.to_string()))?
                .is_dir();
            list.push((name, entry.path(), is_dir));
        }
        Ok(list)
    };
    let mut icons = Vec::new();
    for (set, path, is_dir) in entries(&base)? {
        if !is_dir {
            if let Some(name) = set.strip_suffix(".svg") {
                icons.push(icon(name.to_owned(), format!("{dir}/{set}"), &error)?);
            }
            continue;
        }
        for (file, _, nested) in entries(&path)? {
            if nested {
                return Err(error(format!(
                    "{dir}/{set}/{file}: icon folders hold <set>/<name>.svg, one level deep"
                )));
            }
            if let Some(name) = file.strip_suffix(".svg") {
                icons.push(icon(
                    format!("{set}:{name}"),
                    format!("{dir}/{set}/{file}"),
                    &error,
                )?);
            }
        }
    }
    icons.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(icons)
}

/// Noms limités aux caractères des noms Iconify (minuscules, chiffres, `-`, `_`, `.`) :
/// un nom tapé dans un template se retrouve sans ambiguïté.
fn icon(name: String, path: String, error: &dyn Fn(String) -> Error) -> Result<Icon> {
    let valid = name.split(':').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_.".contains(c))
    });
    if !valid {
        return Err(error(format!(
            "{path}: icon names use lowercase letters, digits, '-', '_' and '.'"
        )));
    }
    Ok(Icon { name, path })
}

/// `<svg src="…" />` compilé comme dans un template, avec `cx` pour contexte. Le nom de
/// l'icône distingue la graine des identifiants de chaque branche.
fn compile_icon(
    root: TokenStream,
    source: &Source,
    icon: &Icon,
    span: Span,
) -> Result<TokenStream> {
    let display = format!("{}#{}", source.display, icon.name);
    let text = format!("<svg src={:?} />", icon.path);
    let nodes = markup::parse_text(&text, &display, span)?;
    let source = Source {
        display: display.clone(),
        ..source.clone()
    };
    let document = template::Document {
        nodes,
        context: Some("cx".parse().unwrap()),
        scope: None,
        slots: None,
        styles: None,
        span,
        display,
    };
    template::generate::compile(root, document, source, None)
}
