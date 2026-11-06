//! Compilation de la liste de classes d'un élément : chaque utilitaire est découpé, ses
//! variantes deviennent une condition, puis les déclarations sont triées dans l'ordre de
//! la feuille de Tailwind, regroupées par condition et résolues.
use super::{
    Config, candidate,
    order::property_rank,
    resolve::{complete_keyframes, initial_vars, resolve_group},
    utilities,
    variants::{self, Condition},
};
/// Déclarations résolues d'une condition (variantes) d'une liste d'utilitaires.
#[derive(Clone, Debug)]
pub struct Group {
    /// Règle des enfants de l'élément qui vérifient ce prédicat (`space-y-*` : tous sauf
    /// le dernier) ; les autres prédicats portent alors sur l'élément lui-même (parent
    /// des enfants).
    pub child: Option<String>,
    /// Prédicats `StyleState` (conjonction).
    pub states: Vec<String>,
    /// Requêtes `@media` / `@container`.
    pub media: Vec<String>,
    /// Pseudo-élément généré.
    pub pseudo: Option<&'static str>,
    /// Ordre de la règle d'état (sous celui des feuilles, `1 << 24`).
    pub order: u32,
    /// Déclarations CSS acceptées par le moteur.
    pub declarations: Vec<(String, String)>,
    /// `@keyframes` des animations utilisées (texte CSS).
    pub keyframes: Vec<String>,
}
/// Liste compilée : utilitaires normaux, puis `!important` (appliqués après les
/// classes de la feuille).
pub struct Set {
    pub normal: Vec<Group>,
    pub important: Vec<Group>,
}
/// Groupe en construction : condition, déclarations brutes, `@keyframes`.
type Draft = (Condition, Vec<(String, String)>, Vec<String>);
struct Entry {
    index: usize,
    important: bool,
    condition: Condition,
    rank: usize,
    count: usize,
    decls: Vec<(String, String)>,
    keyframes: Vec<String>,
}
/// Compile les utilitaires `tokens` d'un élément ; l'erreur désigne le jeton fautif.
pub fn compile_set(tokens: &[&str], cfg: &Config) -> Result<Set, (usize, String)> {
    let mut entries = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let error = |m: String| (index, m);
        let candidate = candidate::parse(token).map_err(error)?;
        let mut condition = variants::apply(&candidate.variants, cfg).map_err(error)?;
        let mut out = utilities::compile(&candidate, cfg).map_err(error)?;
        if out.decls.iter().any(|(n, _)| n == "--tw-child-not-last") {
            // `:where(& > :not(:last-child))` : règle posée sur les enfants.
            if condition.pseudo.is_some() || condition.child.is_some() {
                return Err(error(
                    "space-*/divide-* cannot target a pseudo-element or a child variant".into(),
                ));
            }
            out.decls.retain(|(n, _)| n != "--tw-child-not-last");
            condition.child = Some(
                "__ui_root::ui::styling::StyleState::Not(&__ui_root::ui::styling::StyleState::LastChild)"
                    .into(),
            );
        }
        let rank = out
            .decls
            .iter()
            .map(|(n, _)| property_rank(n))
            .min()
            .unwrap_or(0);
        entries.push(Entry {
            index,
            important: candidate.important,
            condition,
            rank,
            count: out.decls.len(),
            decls: out.decls,
            keyframes: out.keyframes,
        });
    }
    // Ordre de la feuille de Tailwind : variantes (rangs décroissants comparés comme
    // les masques de variantes : la plus tardive d'abord), propriétés, nombre, nom.
    entries.sort_by(|a, b| {
        a.condition
            .ranks
            .cmp(&b.condition.ranks)
            .then(a.rank.cmp(&b.rank))
            .then(b.count.cmp(&a.count))
            .then_with(|| tokens[a.index].cmp(tokens[b.index]))
    });
    let mut set = Set {
        normal: Vec::new(),
        important: Vec::new(),
    };
    // Variables de la base (sans variante), visibles de toutes les conditions.
    let mut base_vars = initial_vars(cfg);
    for entry in &entries {
        if entry.condition == Condition::default()
            || (entry.condition.states.is_empty()
                && entry.condition.media.is_empty()
                && entry.condition.pseudo.is_none())
        {
            for (n, v) in &entry.decls {
                if n.starts_with("--tw-") {
                    base_vars.insert(n.clone(), v.clone());
                }
            }
        }
    }
    for important in [false, true] {
        // Groupes par condition, dans l'ordre de la première occurrence.
        let mut groups: Vec<Draft> = Vec::new();
        for entry in entries.iter().filter(|e| e.important == important) {
            let key = entry.condition.key();
            let at = match groups.iter().position(|(c, _, _)| c.key() == key) {
                Some(at) => at,
                None => {
                    groups.push((entry.condition.clone(), Vec::new(), Vec::new()));
                    groups.len() - 1
                }
            };
            groups[at].1.extend(entry.decls.iter().cloned());
            for frames in &entry.keyframes {
                groups[at]
                    .2
                    .push(complete_keyframes(frames).map_err(|m| (entry.index, m))?);
            }
        }
        // `before:`/`after:` : la boîte générée existe hors de toute condition, avec
        // `content: var(--tw-content)` (le moteur ne rend pas `content` conditionnel).
        for kind in ["before", "after"] {
            if !groups.iter().any(|(c, _, _)| c.pseudo == Some(kind)) {
                continue;
            }
            let base = Condition {
                pseudo: Some(kind),
                ranks: vec![if kind == "before" { 13 } else { 14 }],
                ..Default::default()
            };
            let at = match groups.iter().position(|(c, _, _)| c.key() == base.key()) {
                Some(at) => at,
                None => {
                    let at = groups
                        .iter()
                        .position(|(c, _, _)| c.pseudo == Some(kind))
                        .unwrap();
                    groups.insert(at, (base, Vec::new(), Vec::new()));
                    at
                }
            };
            groups[at]
                .1
                .insert(0, ("content".into(), "var(--tw-content)".into()));
        }
        let base_raw = groups
            .iter()
            .find(|(c, _, _)| c.key() == Condition::default().key())
            .map(|(_, d, _)| d.clone())
            .unwrap_or_default();
        let target = if important {
            &mut set.important
        } else {
            &mut set.normal
        };
        for (i, (condition, raw, keyframes)) in groups.into_iter().enumerate() {
            let is_base = condition.key() == Condition::default().key();
            let mut vars = base_vars.clone();
            for (n, v) in &raw {
                if n.starts_with("--tw-") {
                    vars.insert(n.clone(), v.clone());
                }
            }
            let base = (!is_base).then_some(base_raw.as_slice());
            let declarations = resolve_group(&raw, base, &vars).map_err(|m| (0, m))?;
            target.push(Group {
                child: condition.child,
                states: condition.states,
                media: condition.media,
                pseudo: condition.pseudo,
                order: if important { 1 << 28 } else { 0 } + 1 + i as u32 * 256,
                declarations,
                keyframes,
            });
        }
    }
    Ok(set)
}
