//! Oracle : les déclarations produites pour des utilitaires sont comparées à celles de
//! la feuille que Tailwind 4.3.3 a réellement générée pour RustMusic.
//!
//! Les deux côtés passent par la même normalisation : variables résolues (thème et
//! valeurs initiales `@property` de la feuille générée pour l'oracle), `rem` en px,
//! propriétés logiques (`padding-inline`…) en côtés physiques, propriétés composées
//! (`translate`, `transition-*`, `box-shadow`) en propriétés du moteur, puis
//! traduction par le compilateur CSS du moteur ; les nombres du code obtenu sont
//! comparés à 1 % près. `oracle.css` reprend un extrait de la feuille générée (thème,
//! `@property` et règles des utilitaires testés) ; la feuille complète, si elle est
//! présente, sert au relevé de `tools::oracle` sur tous les jetons acceptés.
mod tokens;
use crate::tailwind::{Config, compile_set, resolve::resolve_group, theme, utilities};
use std::collections::BTreeMap;
use tokens::TOKENS;

const FIXTURE: &str = include_str!("oracle.css");

/// Feuille générée par Tailwind : variables du thème et valeurs initiales.
pub(in crate::tailwind) struct Oracle {
    css: String,
    vars: BTreeMap<String, String>,
}
/// Échappement d'un nom de classe dans un sélecteur (`top-1/2` → `top-1\/2`).
fn escape(token: &str) -> String {
    token
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_string()
            } else {
                format!("\\{c}")
            }
        })
        .collect()
}
/// Déclarations `nom:valeur` d'un bloc minifié.
fn declarations(block: &str) -> Vec<(String, String)> {
    crate::css::parser::split(block, ';')
        .unwrap_or_default()
        .into_iter()
        .filter_map(|d| {
            let (n, v) = d.split_once(':')?;
            let v = v.trim().trim_end_matches("!important").trim();
            Some((n.trim().to_owned(), v.to_owned()))
        })
        .filter(|(n, v)| !n.is_empty() && !v.is_empty())
        .collect()
}
/// Bloc `{…}` qui commence à `open`.
fn block(css: &str, open: usize) -> Option<&str> {
    let close = theme::matching(&css[open + 1..].replace(['{'], "(").replace('}', ")"))?;
    Some(&css[open + 1..open + 1 + close])
}
impl Oracle {
    pub(crate) fn new(css: &str) -> Self {
        let mut vars = BTreeMap::new();
        if let Some(at) = css.find("@layer theme{:root,:host{") {
            let open = at + "@layer theme{:root,:host".len();
            if let Some(b) = block(css, open) {
                vars.extend(declarations(b));
            }
        }
        let mut rest = css;
        while let Some(at) = rest.find("@property ") {
            let tail = &rest[at + 10..];
            let Some(open) = tail.find('{') else { break };
            let name = tail[..open].trim().to_owned();
            let body = block(tail, open).unwrap_or("");
            if let Some((_, v)) = declarations(body)
                .into_iter()
                .find(|(n, _)| n == "initial-value")
            {
                vars.entry(name).or_insert(v);
            }
            rest = &tail[open..];
        }
        Self {
            css: css.to_owned(),
            vars,
        }
    }
    /// Déclarations de `token` dans l'ordre de la feuille (repli puis `@supports`).
    pub(crate) fn rule(&self, token: &str) -> Option<Vec<(String, String)>> {
        let needle = format!(".{}", escape(token));
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(found) = self.css[at..].find(&needle) {
            let start = at + found + needle.len();
            at = start;
            let next = self.css[start..].chars().next().unwrap_or(' ');
            if next.is_ascii_alphanumeric() || next == '-' || next == '_' || next == '\\' {
                continue;
            }
            // Début d'un sélecteur seulement (pas `.parent .token`).
            let before = self.css[..start - needle.len()]
                .chars()
                .last()
                .unwrap_or('}');
            if !matches!(before, '}' | '{' | ';' | ',' | '\n') {
                continue;
            }
            let Some(open) = self.css[start..].find('{').map(|o| start + o) else {
                break;
            };
            // Liste de sélecteurs (`.token,.autre`) : hors sujet ; `:where(a,b)` gardé.
            if crate::css::parser::split(&self.css[start..open], ',').is_ok_and(|p| p.len() > 1) {
                continue;
            }
            if let Some(b) = block(&self.css, open) {
                out.extend(declarations(b));
            }
        }
        (!out.is_empty()).then_some(out)
    }
    /// Déclarations normalisées d'une règle de la feuille générée.
    pub(crate) fn normalized(&self, decls: &[(String, String)]) -> Vec<(String, String)> {
        let mut vars = self.vars.clone();
        for (n, v) in decls {
            if n.starts_with("--") {
                vars.insert(n.clone(), v.clone());
            }
        }
        let mut raw = Vec::new();
        for (name, value) in decls {
            if name.starts_with("--")
                || name.starts_with("-webkit-")
                    && name != "-webkit-line-clamp"
                    && name != "-webkit-box-orient"
            {
                continue;
            }
            let value = resolve(value, &vars, 0);
            let value = utilities::spaced(&value).replace(" in oklab", "");
            let sides = |prefix: &str, a: &str, b: &str| {
                vec![
                    (format!("{prefix}-{a}"), value.clone()),
                    (format!("{prefix}-{b}"), value.clone()),
                ]
            };
            raw.extend(match name.as_str() {
                "padding-inline" => sides("padding", "left", "right"),
                "padding-block" => sides("padding", "top", "bottom"),
                "margin-inline" => sides("margin", "left", "right"),
                "margin-block" => sides("margin", "top", "bottom"),
                "inset-inline" => vec![
                    ("left".into(), value.clone()),
                    ("right".into(), value.clone()),
                ],
                "inset-block" => vec![
                    ("top".into(), value.clone()),
                    ("bottom".into(), value.clone()),
                ],
                "padding-inline-start" => vec![("padding-left".into(), value.clone())],
                "padding-inline-end" => vec![("padding-right".into(), value.clone())],
                "margin-inline-start" => vec![("margin-left".into(), value.clone())],
                "margin-inline-end" => vec![("margin-right".into(), value.clone())],
                "border-inline-width" => sides("border", "left-width", "right-width"),
                "border-block-width" => sides("border", "top-width", "bottom-width"),
                "border-radius" if utilities::px_length(&value).is_some_and(|n| n >= 9999.) => {
                    vec![(name.clone(), "9999px".into())]
                }
                // `span 2/span 2` : le moteur lit la forme courte équivalente.
                "grid-column" | "grid-row" if value.starts_with("span ") => {
                    vec![(
                        name.clone(),
                        value.split('/').next().unwrap_or("").trim().to_owned(),
                    )]
                }
                _ => vec![(name.clone(), value)],
            });
        }
        normalize(&raw)
    }
}
/// Remplace toutes les variables connues (les autres restent pour l'exécution).
fn resolve(value: &str, vars: &BTreeMap<String, String>, depth: usize) -> String {
    if depth > 16 {
        return value.to_owned();
    }
    let mut out = String::new();
    let mut rest = value;
    while let Some(at) = rest.find("var(--") {
        out += &rest[..at];
        let tail = &rest[at + 4..];
        let Some(close) = theme::matching(tail) else {
            return out + &rest[at..];
        };
        let inner = &tail[..close];
        let (name, fallback) = match crate::css::parser::split(inner, ',')
            .ok()
            .filter(|p| p.len() > 1)
        {
            Some(_) => {
                let (n, f) = inner.split_once(',').unwrap();
                (n.trim(), Some(f.trim()))
            }
            None => (inner.trim(), None),
        };
        match vars.get(name).filter(|v| v.as_str() != "initial") {
            Some(v) => out += &resolve(v, vars, depth + 1),
            None => match fallback {
                Some(f) => out += &resolve(f, vars, depth + 1),
                None if name.starts_with("--tw-") => {}
                None => out += &format!("var({inner})"),
            },
        }
        rest = &tail[close + 1..];
    }
    out + rest
}
/// Normalisation commune : propriétés composées puis `rem` en px et `calc()` évalués.
fn normalize(raw: &[(String, String)]) -> Vec<(String, String)> {
    resolve_group(raw, None, &BTreeMap::new())
        .unwrap_or_default()
        .into_iter()
        .map(|(n, v)| (n, utilities::rem(&v)))
        .collect()
}
/// Code du moteur d'une déclaration, réduit à son squelette et à ses nombres.
fn engine(
    name: &str,
    value: &str,
    vars: &BTreeMap<String, String>,
) -> Result<(String, Vec<f64>), String> {
    let props = crate::css::values_declaration(name, value, vars)?;
    let code = props
        .iter()
        .map(|(n, v)| format!("{n}={v}"))
        .collect::<Vec<_>>()
        .join(";");
    let mut skeleton = String::new();
    let mut numbers = Vec::new();
    let mut current = String::new();
    for c in code.chars().chain([' ']) {
        if c.is_ascii_digit()
            || c == '.'
            || (c == '-'
                && current.is_empty()
                && !skeleton.ends_with(|c: char| c.is_ascii_alphanumeric()))
        {
            current.push(c);
            continue;
        }
        if !current.is_empty() {
            if let Ok(n) = current.parse::<f64>() {
                numbers.push(n);
                skeleton.push('#');
            } else {
                skeleton += &current;
            }
            current.clear();
        }
        skeleton.push(c);
    }
    // Suffixes numériques (`f32`, `u16`) sans incidence.
    let skeleton = skeleton
        .replace("#f32", "#")
        .replace("#u16", "#")
        .replace("#u8", "#");
    Ok((skeleton, numbers))
}
/// Écart entre les déclarations d'un utilitaire et celles de l'oracle, `None` si
/// équivalentes.
pub(in crate::tailwind) fn difference(
    mine: &[(String, String)],
    oracle: &[(String, String)],
    vars: &BTreeMap<String, String>,
) -> Option<String> {
    let mine = normalize(mine);
    for (name, expected) in oracle {
        if name.starts_with("animation") || name == "content" {
            // Animations comparées par le compilateur de keyframes, pas ici.
            continue;
        }
        // Variables de l'application remplacées des deux côtés par la même valeur.
        let expected = &resolve(expected, vars, 0);
        let Some((_, got)) = mine.iter().rev().find(|(n, _)| n == name) else {
            if engine(name, expected, vars).is_err() {
                // Déclaration invalide pour Tailwind aussi (dégradé sans couleurs…).
                continue;
            }
            return Some(format!("{name}: missing (oracle {expected})"));
        };
        let got = &resolve(got, vars, 0);
        let a = engine(name, got, vars);
        let b = engine(name, expected, vars);
        match (a, b) {
            (Ok((sa, na)), Ok((sb, nb))) => {
                let close = na.len() == nb.len()
                    && na
                        .iter()
                        .zip(&nb)
                        .all(|(x, y)| (x - y).abs() <= 0.01 * x.abs().max(y.abs()).max(1.));
                if sa != sb || !close {
                    return Some(format!("{name}: {got} != oracle {expected}"));
                }
            }
            (Err(_), Err(_)) if got == expected => {}
            (a, b) => {
                return Some(format!(
                    "{name}: {got} ({}) vs oracle {expected} ({})",
                    a.err().unwrap_or_default(),
                    b.err().unwrap_or_default()
                ));
            }
        }
    }
    None
}
/// Thème de RustMusic (`@theme` d'`app.css`).
pub(in crate::tailwind) fn rustmusic_theme() -> Config {
    let theme = [
        ("--font-sans", "'Inter Variable', system-ui, sans-serif"),
        ("--color-accent-500", "var(--acc-500, #22c55e)"),
        ("--color-accent-600", "var(--acc-600, #16a34a)"),
        ("--color-accent-700", "var(--acc-700, #15803d)"),
    ];
    Config::new(
        &theme.map(|(n, v)| (n.to_owned(), v.to_owned())),
        &[],
        &[("dark".into(), "&:where(.dark, .dark *)".into())],
    )
    .unwrap()
}
/// Déclarations du groupe d'un utilitaire seul (la condition de ses variantes).
pub(in crate::tailwind) fn mine(
    token: &str,
    cfg: &Config,
) -> Result<Vec<(String, String)>, String> {
    let set = compile_set(&[token], cfg).map_err(|(_, m)| m)?;
    let groups = if set.normal.is_empty() {
        set.important
    } else {
        set.normal
    };
    // Groupe de l'utilitaire (pseudo-élément compris), sans le `content` ajouté.
    Ok(groups
        .into_iter()
        .filter(|g| g.declarations.iter().any(|(n, _)| n != "content"))
        .flat_map(|g| g.declarations)
        .filter(|(n, _)| n != "content")
        .collect())
}

#[test]
fn utilities_match_the_declarations_generated_by_tailwind() {
    let oracle = Oracle::new(FIXTURE);
    assert!(oracle.vars.contains_key("--spacing"), "theme layer");
    let cfg = rustmusic_theme();
    let vars = BTreeMap::from([
        ("--rg-mu".into(), "#8a948f".into()),
        ("--rg-tx".into(), "#ffffff".into()),
        ("--rg-s2".into(), "#1b211e".into()),
        ("--rg-bd".into(), "#1f2522".into()),
        ("--acc-600".into(), "#16a34a".into()),
    ]);
    let mut failures = Vec::new();
    for token in TOKENS {
        let Some(expected) = oracle.rule(token) else {
            failures.push(format!("{token}: absent from oracle.css"));
            continue;
        };
        let expected = oracle.normalized(&expected);
        match mine(token, &cfg) {
            Ok(got) => {
                if let Some(d) = difference(&got, &expected, &vars) {
                    failures.push(format!("{token}: {d}"));
                }
            }
            Err(m) => failures.push(format!("{token}: rejected: {m}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
