//! Compilation d'une feuille de framework : règles aplaties découpées en alternatives
//! de sélecteur, variables globales et constantes substituées, déclarations classées,
//! index des règles par clé de sujet et registre des jetons globaux.
use super::walker::{Context, Walker};
use super::{Decl, Report, Rule, Sheet, selectors, syntax, values};
use selectors::{Complex, Key};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

/// Sujet `:root`, `html` ou `:host` seul (règles de variables globales).
fn is_root(x: &Complex) -> bool {
    x.element.is_none() && x.parts.len() == 1 && {
        let c = x.subject();
        c.classes.is_empty()
            && c.ids.is_empty()
            && c.attrs.is_empty()
            && (c.tag.as_deref() == Some("html") && c.pseudos.is_empty()
                || c.tag.is_none() && c.pseudos == [selectors::Pseudo::Root])
    }
}
/// Classes d'un sélecteur illisible, repérées dans le texte.
fn raw_classes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.'
            && chars
                .get(i + 1)
                .is_some_and(|c| c.is_alphabetic() || *c == '_' || *c == '-' || *c == '\\')
        {
            let mut name = String::new();
            i += 1;
            while i < chars.len() {
                let c = chars[i];
                if c == '\\' && i + 1 < chars.len() {
                    name.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    name.push(c);
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(name);
            continue;
        }
        i += 1;
    }
    out
}
impl Sheet {
    /// Compile le texte d'une feuille.
    pub fn compile(text: &str) -> Self {
        let nodes = syntax::parse(text);
        let mut walker = Walker {
            flats: Vec::new(),
            layers: Vec::new(),
            anonymous: 0,
            keyframes: Vec::new(),
            properties: Vec::new(),
            report: Report::default(),
            text: text.to_owned(),
        };
        walker.walk(&nodes, &Context::default());
        let Walker {
            flats,
            layers,
            keyframes,
            properties,
            mut report,
            ..
        } = walker;
        // Variables globales : `:root` inconditionnel, puis valeurs initiales enregistrées.
        let mut globals = BTreeMap::new();
        let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut referenced = BTreeSet::new();
        let mut parsed = Vec::new();
        for flat in &flats {
            let alternatives = flat
                .selectors
                .iter()
                .map(|s| (s.clone(), selectors::parse(s)))
                .collect::<Vec<_>>();
            let root_only = flat.conditions.is_empty()
                && alternatives
                    .iter()
                    .any(|(_, x)| x.as_ref().is_ok_and(is_root));
            for d in &flat.declarations {
                let mut refs = Vec::new();
                values::references(&d.value, &mut refs);
                referenced.extend(refs);
                if d.name.starts_with("--") {
                    declared
                        .entry(d.name.clone())
                        .or_default()
                        .insert(d.value.clone());
                    if root_only {
                        globals.insert(d.name.clone(), d.value.clone());
                    }
                }
            }
            parsed.push(alternatives);
        }
        let mut non_inherited = BTreeSet::new();
        for (name, initial, inherits) in &properties {
            if !inherits {
                non_inherited.insert(name.clone());
            }
            if let Some(initial) = initial {
                globals
                    .entry(name.clone())
                    .or_insert_with(|| initial.clone());
                declared
                    .entry(name.clone())
                    .or_default()
                    .insert(initial.clone());
            }
        }
        // Constantes : une seule valeur dans la feuille, définie globalement, sans var()
        // non constante.
        let mut constants: BTreeMap<String, String> = declared
            .iter()
            .filter(|(name, values)| values.len() == 1 && globals.contains_key(*name))
            .filter(|(name, _)| !non_inherited.contains(*name))
            .map(|(name, values)| (name.clone(), values.iter().next().unwrap().clone()))
            .collect();
        loop {
            let snapshot = constants.clone();
            let known = |n: &str| snapshot.get(n).cloned();
            let mut changed = false;
            let mut remove = Vec::new();
            for (name, value) in constants.iter_mut() {
                if value.contains("var(") {
                    let next = values::substitute(value, &known, 0);
                    if next.contains("var(") {
                        remove.push(name.clone());
                    } else if next != *value {
                        *value = next;
                        changed = true;
                    }
                }
            }
            for name in &remove {
                constants.remove(name);
                changed = true;
            }
            if !changed {
                break;
            }
        }
        let layer_count = layers.len() as u32;
        let layer_rank = |layer: &Option<String>| match layer {
            Some(name) => layers.iter().position(|l| l == name).unwrap_or(0) as u32 + 1,
            None => layer_count + 1,
        };
        let known = |n: &str| constants.get(n).cloned();
        let mut rules = Vec::new();
        let mut classes = BTreeSet::new();
        for (order, (flat, alternatives)) in flats.iter().zip(parsed).enumerate() {
            report.rules += 1;
            let mut decls = Vec::new();
            let mut accepted = 0;
            for d in &flat.declarations {
                report.declarations += 1;
                // Propriétés logiques : côtés physiques d'un texte horizontal de gauche
                // à droite.
                let physical = values::logical(&d.name, &d.value);
                let mut skipped = None;
                let mut dynamic = false;
                for (name, value) in physical {
                    let value = values::substitute(&value, &known, 0);
                    let kind = values::classify(&name, &value, &constants);
                    match &kind {
                        values::Kind::Skip(category) => skipped = Some(category.clone()),
                        values::Kind::Dynamic(..) => dynamic = true,
                        _ => {}
                    }
                    decls.push(Decl {
                        name,
                        value,
                        important: d.important,
                        kind,
                    });
                }
                match skipped {
                    Some(category) => {
                        report.skip(d.at, category, &format!("{}: {}", d.name, d.value));
                    }
                    None => {
                        report.declarations_accepted += 1;
                        report.declarations_dynamic += u32::from(dynamic);
                        accepted += 1;
                    }
                }
            }
            let decls = Rc::new(decls);
            let mut selectors_ok = 0;
            for (text, parsed) in &alternatives {
                report.selectors += 1;
                match parsed {
                    Ok(x) => {
                        selectors_ok += 1;
                        report.selectors_accepted += 1;
                        selectors::classes(x, &mut classes);
                        let root = is_root(x);
                        // `:root` inconditionnel : ses variables sont globales ; ses autres
                        // déclarations vont à `<body>`.
                        if root
                            && flat.conditions.is_empty()
                            && decls.iter().all(|d| d.name.starts_with("--"))
                        {
                            continue;
                        }
                        rules.push(Rule {
                            specificity: x.specificity(),
                            complex: x.clone(),
                            layer: layer_rank(&flat.layer),
                            order: order as u32,
                            conditions: flat.conditions.clone(),
                            declarations: decls.clone(),
                            root,
                        });
                    }
                    Err(category) => {
                        report.skip(flat.at, category.clone(), text);
                        // Classes nommées dans un sélecteur refusé : acceptées par les
                        // templates (sans effet).
                        classes.extend(raw_classes(text));
                    }
                }
            }
            if selectors_ok > 0 && accepted > 0 {
                report.rules_applied += 1;
            }
            if selectors_ok == alternatives.len() && accepted == flat.declarations.len() {
                report.rules_full += 1;
            }
        }
        let mut index: HashMap<Key, Vec<usize>> = HashMap::new();
        for (i, rule) in rules.iter().enumerate() {
            let key = if rule.root {
                Key::Tag("body".into())
            } else {
                selectors::key(rule.complex.subject())
            };
            index.entry(key).or_default().push(i);
        }
        Sheet {
            rules,
            index,
            globals,
            non_inherited,
            constants,
            referenced,
            classes,
            keyframes,
            report,
            text: text.to_owned(),
        }
    }
    /// Code `Registry` des variables globales (jetons), triées par nom.
    pub fn registry_code(&self) -> String {
        let known = |n: &str| self.constants.get(n).cloned();
        let mut entries = Vec::new();
        for (name, value) in &self.globals {
            if self.constants.contains_key(name) && !self.referenced.contains(name) {
                continue;
            }
            let value = values::substitute(value, &known, 0);
            if let Ok(tokens) = values::tokenize(&value) {
                entries.push(format!("({name:?},{})", values::token_slice(&tokens)));
            }
        }
        format!(
            "__ui_root::ui::styling::tokens::Registry{{values:&[{}],non_inherited:&[{}]}}",
            entries.join(","),
            self.non_inherited
                .iter()
                .map(|n| format!("{n:?}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
