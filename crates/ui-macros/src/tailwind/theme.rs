//! Thème Tailwind : valeurs par défaut de Tailwind v4 (`theme.css`), surchargées et
//! étendues par les blocs `@theme` de la feuille, et variantes `@custom-variant`.
use crate::css::parser;
use std::collections::BTreeMap;

const DEFAULT_THEME: &str = include_str!("theme.css");

/// Thème et variantes d'une feuille qui importe `tailwindcss`.
pub struct Config {
    theme: BTreeMap<String, String>,
    keyframes: BTreeMap<String, String>,
    pub(super) variants: BTreeMap<String, String>,
}
thread_local! {
    static DEFAULTS: (BTreeMap<String, String>, BTreeMap<String, String>) = defaults();
}
/// Thème par défaut, lu une fois par compilation.
fn defaults() -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let mut theme = BTreeMap::new();
    let mut keyframes = BTreeMap::new();
    for item in parser::parse(DEFAULT_THEME).expect("valid default Tailwind theme") {
        if let parser::Item::Theme {
            declarations,
            keyframes: frames,
            ..
        } = item
        {
            for d in declarations {
                if !d.value.contains("--theme(") {
                    theme.insert(d.name, d.value);
                }
            }
            keyframes.extend(frames);
        }
    }
    (theme, keyframes)
}
impl Config {
    /// Thème par défaut surchargé par `theme` (déclarations `@theme` dans l'ordre),
    /// `keyframes` (animations des blocs `@theme`) et `variants` (`@custom-variant`).
    pub fn new(
        theme: &[(String, String)],
        keyframes: &[(String, String)],
        variants: &[(String, String)],
    ) -> Result<Self, String> {
        let (mut values, mut frames) = DEFAULTS.with(|d| d.clone());
        for (name, value) in theme {
            if value == "initial" {
                // `--color-*: initial` vide un espace de noms ; `--*: initial` tout le thème.
                match name.strip_suffix('*') {
                    Some(prefix) => values.retain(|k, _| !k.starts_with(prefix)),
                    None => {
                        values.remove(name);
                    }
                }
                continue;
            }
            if name.contains('*') {
                return Err(format!("@theme: {name} only accepts the value initial"));
            }
            values.insert(name.clone(), value.clone());
        }
        frames.extend(keyframes.iter().cloned());
        let mut variant_map = BTreeMap::new();
        for (name, selector) in variants {
            variant_map.insert(name.clone(), selector.clone());
        }
        Ok(Self {
            theme: values,
            keyframes: frames,
            variants: variant_map,
        })
    }
    /// Valeur d'une variable du thème, les références à d'autres variables du thème
    /// résolues (les autres `var()` restent pour l'exécution).
    pub fn get(&self, name: &str) -> Option<String> {
        let value = self.theme.get(name)?;
        Some(self.inline(value, 0))
    }
    fn inline(&self, value: &str, depth: usize) -> String {
        if depth > 16 || !value.contains("var(--") {
            return value.to_owned();
        }
        let mut out = String::new();
        let mut rest = value;
        while let Some(at) = rest.find("var(--") {
            out += &rest[..at];
            let tail = &rest[at + 4..];
            let Some(close) = matching(tail) else {
                out += &rest[at..];
                return out;
            };
            let inner = &tail[..close];
            let name = inner.split(',').next().unwrap_or("").trim();
            match self.theme.get(name) {
                Some(v) => out += &self.inline(v, depth + 1),
                None => out += &format!("var({inner})"),
            }
            rest = &tail[close + 1..];
        }
        out + rest
    }
    /// `--{namespace}-{key}` du thème.
    pub fn lookup(&self, namespace: &str, key: &str) -> Option<String> {
        self.get(&format!("--{namespace}-{key}"))
    }
    /// Clés d'un espace de noms (`color` : `red-500`, `white`…), pour les suggestions.
    pub fn keys(&self, namespace: &str) -> Vec<String> {
        let prefix = format!("--{namespace}-");
        self.theme
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .filter(|k| !k.contains("--"))
            .map(str::to_owned)
            .collect()
    }
    /// `@keyframes` du thème (texte CSS complet).
    pub fn keyframes(&self, name: &str) -> Option<&str> {
        self.keyframes.get(name).map(String::as_str)
    }
    /// Pas de l'échelle d'espacement (`--spacing`), en px.
    pub fn spacing(&self) -> Option<f32> {
        super::utilities::px_length(&self.get("--spacing")?)
    }
}
/// Position de la parenthèse fermante qui équilibre `text` (qui suit une ouvrante).
pub(super) fn matching(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}
