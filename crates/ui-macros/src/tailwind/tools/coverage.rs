//! Outil de couverture : classes Tailwind de RustMusic acceptées par le compilateur.
//!
//! ```text
//! cargo test -p karbon-ui-macros --lib tailwind::tools::coverage -- --ignored --nocapture
//! ```
//!
//! Variables : `RUSTMUSIC` (racine du projet, par défaut `J:/wamp/www/__rust/rustmusic`),
//! `TAILWIND_COVERAGE_OUT` (rapport Markdown, par défaut `target/tailwind-coverage.md`).
//! Les jetons des attributs `class="…"` des fichiers `.svelte` sont extraits (les parties
//! `{…}` sont ignorées) ; ceux que Tailwind a réellement générés (présents dans la feuille
//! compilée de RustMusic) et qui ne sont pas des classes de `app.css` forment le
//! dénominateur. Le test est ignoré quand le projet est absent.
use crate::tailwind::{Config, candidate, compile_set};
use std::{collections::BTreeMap, path::Path};

/// Jetons des attributs `class="…"` d'un fichier Svelte, expressions `{…}` exclues.
fn svelte_tokens(text: &str, out: &mut BTreeMap<String, usize>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(at) = text[i..].find("class=\"") {
        let start = i + at;
        let before = start.checked_sub(1).map(|j| bytes[j]);
        i = start + 7;
        if before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b':' || b == b'-' || b == b'_')
        {
            continue;
        }
        let mut depth = 0;
        let mut current = String::new();
        let mut words = Vec::new();
        while i < bytes.len() {
            let c = bytes[i] as char;
            if depth == 0 && c == '"' {
                break;
            }
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 && c != '}' && !c.is_whitespace() {
                current.push(c);
            } else if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            i += 1;
        }
        if !current.is_empty() {
            words.push(current);
        }
        for w in words {
            // Octets UTF-8 reconstitués : les jetons sont ASCII en pratique.
            if w.is_ascii() {
                *out.entry(w).or_default() += 1;
            }
        }
        i += 1;
    }
}
pub(super) fn walk_svelte(dir: &Path, out: &mut BTreeMap<String, usize>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_svelte(&path, out);
        } else if path.extension().is_some_and(|e| e == "svelte")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            svelte_tokens(&text, out);
        }
    }
}
/// Classes définies par les blocs `<style>` des composants Svelte (CSS propre, pas
/// Tailwind), écartées comme celles d'`app.css`.
fn svelte_style_classes(dir: &Path, out: &mut std::collections::BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            svelte_style_classes(&path, out);
        } else if path.extension().is_some_and(|e| e == "svelte")
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            let mut rest = text.as_str();
            while let Some(at) = rest.find("<style") {
                let body = &rest[at..];
                let Some(open) = body.find('>') else { break };
                let end = body.find("</style>").unwrap_or(body.len());
                out.extend(app_css(&body[open + 1..end.max(open + 1)]).1);
                rest = &body[end..];
                if end == body.len() {
                    break;
                }
            }
        }
    }
}
/// Sélecteurs de classe d'une feuille générée par Tailwind, échappements retirés.
fn generated_classes(css: &str) -> std::collections::BTreeSet<String> {
    let mut set = std::collections::BTreeSet::new();
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.' && i + 1 < bytes.len() && !bytes[i - 1.min(i)].is_ascii_digit() {
            let mut name = String::new();
            let mut j = i + 1;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c == '\\' && j + 1 < bytes.len() {
                    name.push(bytes[j + 1] as char);
                    j += 2;
                    continue;
                }
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    name.push(c);
                    j += 1;
                } else {
                    break;
                }
            }
            if !name.is_empty() {
                set.insert(name);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    set
}
/// Variables (`--nom: valeur`) déclarées n'importe où dans `app.css` (`:root`, `.dark`…).
pub(super) fn app_vars(text: &str) -> BTreeMap<String, String> {
    let text = strip_comments(text);
    let mut vars = BTreeMap::new();
    for part in text.split(['{', '}', ';']) {
        if let Some((name, value)) = part.split_once(':')
            && name.trim().starts_with("--")
            && !value.contains('@')
        {
            vars.insert(name.trim().to_owned(), value.trim().to_owned());
        }
    }
    // Couleurs d'accent choisies par l'utilisateur (--acc-*) : repli du thème.
    for (name, value) in app_css(&text).0 {
        if let Some(inner) = value.strip_prefix("var(").and_then(|v| v.strip_suffix(')'))
            && let Some((var, fallback)) = inner.split_once(',')
        {
            vars.entry(var.trim().to_owned())
                .or_insert_with(|| fallback.trim().to_owned());
        }
        let _ = name;
    }
    vars
}

/// `@keyframes` d'`app.css` (texte CSS complet).
fn app_keyframes(text: &str) -> Vec<(String, String)> {
    let text = strip_comments(text);
    let mut out = Vec::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("@keyframes ") {
        let tail = &rest[at..];
        let Some(open) = tail.find('{') else { break };
        let name = tail[11..open].trim().to_owned();
        let mut depth = 0;
        let mut end = open;
        for (i, c) in tail[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push((name, tail[..=end].to_owned()));
        rest = &tail[end + 1..];
    }
    out
}
/// Thème (`@theme`) et classes propres d'`app.css`.
fn app_css(text: &str) -> (Vec<(String, String)>, std::collections::BTreeSet<String>) {
    let mut theme = Vec::new();
    if let Some(at) = text.find("@theme {")
        && let Some(end) = text[at..].find('}')
    {
        let block = &text[at + 8..at + end];
        let block = strip_comments(block);
        for d in block.split(';') {
            if let Some((n, v)) = d.split_once(':') {
                theme.push((n.trim().to_owned(), v.trim().to_owned()));
            }
        }
    }
    let text = strip_comments(text);
    let mut classes = std::collections::BTreeSet::new();
    // Sélecteurs : texte hors des blocs de déclarations.
    let mut depth = 0;
    let mut selector = String::new();
    for c in text.chars() {
        match c {
            '{' => {
                for part in selector.split(|c: char| {
                    !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                }) {
                    for name in part.split('.').skip(1) {
                        if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
                            classes.insert(name.to_owned());
                        }
                    }
                }
                selector.clear();
                depth += 1;
            }
            '}' => {
                depth -= 1;
                selector.clear();
            }
            ';' => selector.clear(),
            c => selector.push(c),
        }
    }
    let _ = depth;
    (theme, classes)
}
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("/*") {
        out += &rest[..at];
        match rest[at..].find("*/") {
            Some(end) => rest = &rest[at + end + 2..],
            None => return out,
        }
    }
    out + rest
}
/// Famille d'un utilitaire : racine sans valeur (`bg`, `text`, `rounded-t`, `gap-x`).
fn family(utility: &str) -> String {
    let base = utility.trim_start_matches('!').trim_start_matches('-');
    if base.starts_with('[') {
        return "[property:value]".into();
    }
    let base = base.split('/').next().unwrap_or(base);
    let stop = base.find(['[', '(']).unwrap_or(base.len());
    let head = base[..stop].trim_end_matches('-');
    let parts = head.split('-').collect::<Vec<_>>();
    let value_like = |p: &str| {
        p.is_empty()
            || p.chars().next().is_some_and(|c| c.is_ascii_digit())
            || [
                "xs",
                "sm",
                "md",
                "lg",
                "xl",
                "full",
                "auto",
                "px",
                "screen",
                "none",
                "white",
                "black",
                "transparent",
                "current",
                "tight",
                "snug",
                "normal",
                "relaxed",
                "loose",
                "wide",
                "wider",
                "widest",
                "tighter",
                "thin",
                "light",
                "medium",
                "semibold",
                "bold",
                "extrabold",
                "black",
                "fit",
                "min",
                "max",
                "dvh",
                "svh",
                "square",
                "video",
                "center",
                "start",
                "end",
                "between",
                "inherit",
                "base",
            ]
            .contains(&p)
            || [
                "red", "orange", "amber", "yellow", "lime", "green", "emerald", "teal", "cyan",
                "sky", "blue", "indigo", "violet", "purple", "fuchsia", "pink", "rose", "slate",
                "gray", "zinc", "neutral", "stone", "accent",
            ]
            .contains(&p)
    };
    let mut keep = parts.len();
    while keep > 1 && value_like(parts[keep - 1]) {
        keep -= 1;
    }
    if stop < base.len() {
        keep = parts.len();
    }
    parts[..keep].join("-")
}

/// Jetons d'une famille : total, acceptés, refus (jeton, message).
type Family = (usize, usize, Vec<(String, String)>);
#[test]
#[ignore = "outil de mesure : lit le projet RustMusic ; lancer avec --ignored --nocapture"]
fn rustmusic_coverage() {
    let root = std::env::var("RUSTMUSIC").unwrap_or_else(|_| "J:/wamp/www/__rust/rustmusic".into());
    let root = Path::new(&root);
    if !root.join("src").is_dir() {
        eprintln!("RustMusic not found at {}: skipped", root.display());
        return;
    }
    let mut tokens = BTreeMap::new();
    walk_svelte(&root.join("src"), &mut tokens);
    let app = std::fs::read_to_string(root.join("src/app.css")).unwrap_or_default();
    let (theme, mut custom) = app_css(&app);
    svelte_style_classes(&root.join("src"), &mut custom);
    let generated = std::fs::read_dir(root.join("build/_app/immutable/assets"))
        .ok()
        .and_then(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "css"))
                .max_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        })
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|css| generated_classes(&css));
    let cfg = Config::new(
        &theme,
        &app_keyframes(&app),
        &[("dark".into(), "&:where(.dark, .dark *)".into())],
    )
    .expect("RustMusic theme");
    let vars = app_vars(&app);

    let mut total = (0usize, 0usize, 0usize, 0usize);
    let mut families: BTreeMap<String, Family> = BTreeMap::new();
    let mut variants: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut errors: BTreeMap<String, usize> = BTreeMap::new();
    let mut ignored = (0usize, 0usize);
    for (token, uses) in &tokens {
        if custom.contains(token.as_str()) {
            ignored.0 += 1;
            continue;
        }
        if generated
            .as_ref()
            .is_some_and(|g| !g.contains(token.as_str()))
        {
            // Pas un utilitaire pour Tailwind non plus (classe JS, coquille, variante absente).
            ignored.1 += 1;
            continue;
        }
        let result = compile_set(&[token.as_str()], &cfg)
            .map_err(|(_, m)| m)
            .and_then(|set| {
                crate::css::utility_class(&set.normal, Vec::new(), &vars)?;
                crate::css::utility_class(&set.important, Vec::new(), &vars)?;
                Ok(())
            });
        let ok = result.is_ok();
        total.0 += 1;
        total.1 += usize::from(ok);
        total.2 += uses;
        total.3 += if ok { *uses } else { 0 };
        let parts = candidate::split_top(token, ':');
        let utility = parts.last().copied().unwrap_or(token);
        let entry = families.entry(family(utility)).or_default();
        entry.0 += 1;
        entry.1 += usize::from(ok);
        if let Err(m) = result {
            entry.2.push((token.clone(), m.clone()));
            let short = m
                .split(';')
                .next()
                .unwrap_or(&m)
                .chars()
                .take(110)
                .collect::<String>();
            *errors.entry(short).or_default() += 1;
        }
        for v in &parts[..parts.len() - 1] {
            let key = if v.starts_with('[') {
                "[arbitrary]".to_owned()
            } else {
                v.split(['[', '/'])
                    .next()
                    .unwrap_or(v)
                    .trim_end_matches('-')
                    .to_owned()
            };
            let e = variants.entry(key).or_default();
            e.0 += 1;
            e.1 += usize::from(ok);
        }
    }
    let pct = |a: usize, b: usize| {
        if b == 0 {
            100.
        } else {
            a as f64 * 100. / b as f64
        }
    };
    let mut report = format!(
        "# Couverture Tailwind sur RustMusic\n\n{} jetons distincts dans les `.svelte` ; {} classes d'`app.css` et {} jetons non générés par Tailwind écartés.\n\n**{} / {} utilitaires acceptés ({:.1} %)**, {:.1} % des occurrences ({} / {}).\n\n",
        tokens.len(),
        ignored.0,
        ignored.1,
        total.1,
        total.0,
        pct(total.1, total.0),
        pct(total.3, total.2),
        total.3,
        total.2
    );
    let mut fams = families.iter().collect::<Vec<_>>();
    fams.sort_by_key(|(_, (n, _, _))| std::cmp::Reverse(*n));
    report += "## Familles (par nombre de jetons)\n\n| Famille | Acceptés | Total | % |\n| --- | ---: | ---: | ---: |\n";
    for (name, (n, ok, _)) in &fams {
        report += &format!("| `{name}` | {ok} | {n} | {:.0} |\n", pct(*ok, *n));
    }
    let mut vars_list = variants.iter().collect::<Vec<_>>();
    vars_list.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    report +=
        "\n## Variantes\n\n| Variante | Acceptés | Total | % |\n| --- | ---: | ---: | ---: |\n";
    for (name, (n, ok)) in &vars_list {
        report += &format!("| `{name}:` | {ok} | {n} | {:.0} |\n", pct(*ok, *n));
    }
    let mut missing = fams
        .iter()
        .filter(|(_, (n, ok, _))| n > ok)
        .collect::<Vec<_>>();
    missing.sort_by_key(|(_, (n, ok, _))| std::cmp::Reverse(n - ok));
    report += "\n## Familles non couvertes (jetons refusés)\n\n| Famille | Refusés | Exemple | Raison |\n| --- | ---: | --- | --- |\n";
    for (name, (n, ok, samples)) in missing.iter().take(60) {
        let (token, message) = &samples[0];
        report += &format!(
            "| `{name}` | {} | `{token}` | {} |\n",
            n - ok,
            message.replace('|', "\\|")
        );
    }
    let mut errs = errors.iter().collect::<Vec<_>>();
    errs.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    report += "\n## Messages les plus fréquents\n\n";
    for (m, n) in errs.iter().take(30) {
        report += &format!("- {n} × {m}\n");
    }
    println!("{report}");
    // Liste complète des refus, dans le fichier seulement.
    let mut refused = families
        .values()
        .flat_map(|(_, _, samples)| samples.iter())
        .collect::<Vec<_>>();
    refused.sort();
    report += "\n## Tous les jetons refusés\n\n";
    for (token, message) in refused {
        report += &format!("- `{token}` : {}\n", message.replace('\n', " "));
    }
    let out = std::env::var("TAILWIND_COVERAGE_OUT").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/tailwind-coverage.md"
        )
        .into()
    });
    let _ = std::fs::write(&out, &report);
    eprintln!("written to {out}");
}
