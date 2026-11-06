//! Cascade d'un élément de template contre une feuille de framework.
//!
//! Le compilateur connaît la balise, les classes et les attributs littéraux de
//! l'élément : il retient les règles dont le sujet peut correspondre, trie leurs
//! déclarations par rang de cascade (importance, couche, spécificité, ordre), résout
//! les variables que l'élément déclare lui-même, puis élimine les déclarations
//! masquées par une déclaration inconditionnelle de rang supérieur. Le reste devient :
//! appels de nœud (base), règles d'état ordonnées par rang (`StyleState::Where`, sans
//! spécificité propre) et déclarations dynamiques évaluées à la résolution des styles.
use super::selectors::{self, Facts, Key, PseudoElement, Tri};
use super::values::{self, Kind};
use super::{Decl, Sheet};
use crate::css::{Class, add_rule, parser, selector};
use std::collections::{BTreeMap, BTreeSet};

const STATE: &str = "__ui_root::ui::styling::StyleState";
const ALWAYS: &str = "__ui_root::ui::styling::StyleState::All(&[])";

/// Balises en ligne par défaut (feuille de style des navigateurs).
const INLINE_TAGS: &[&str] = &[
    "button", "input", "textarea", "select", "img", "span", "a", "label", "small", "strong", "b",
    "i", "em", "code", "svg",
];
/// Rang de cascade d'une déclaration (croissant : la plus forte en dernier).
type Rank = (bool, u32, selectors::Specificity, u32, u32);
struct Entry<'a> {
    rank: Rank,
    rule: usize,
    predicate: Option<String>,
    decl: &'a Decl,
    kind: Kind,
    value: String,
}
/// Déclarations retenues pour une boîte (l'élément ou un pseudo-élément).
fn cascade<'a>(entries: &mut Vec<Entry<'a>>) {
    entries.sort_by_key(|e| e.rank);
    // Élimination : du rang le plus fort au plus faible, une déclaration dont tous les
    // longhands sont déjà posés par une déclaration inconditionnelle (ou de même
    // condition) de rang supérieur ne peut jamais l'emporter.
    let mut keep = vec![true; entries.len()];
    let mut always: BTreeSet<String> = BTreeSet::new();
    let mut same: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for i in (0..entries.len()).rev() {
        let e = &entries[i];
        if matches!(e.kind, Kind::Skip(_) | Kind::Ignored) {
            keep[i] = false;
            continue;
        }
        let longhands = values::coverage(&e.decl.name);
        let covered = longhands.iter().all(|l| {
            always.contains(l)
                || e.predicate
                    .as_deref()
                    .is_some_and(|p| same.get(p).is_some_and(|s| s.contains(l)))
        });
        if covered {
            keep[i] = false;
            continue;
        }
        match &e.predicate {
            None => always.extend(longhands),
            Some(p) => same.entry(p.as_str()).or_default().extend(longhands),
        }
    }
    let mut i = 0;
    entries.retain(|_| {
        i += 1;
        keep[i - 1]
    });
}
/// Variables déclarées inconditionnellement par la boîte et jamais remplacées par une
/// règle conditionnelle de rang supérieur : leur valeur est connue à la compilation.
fn own_variables(entries: &[Entry]) -> BTreeMap<String, String> {
    let mut known = BTreeMap::new();
    let mut blocked = BTreeSet::new();
    for e in entries.iter().rev() {
        if !e.decl.name.starts_with("--") || blocked.contains(&e.decl.name) {
            continue;
        }
        if e.predicate.is_some() {
            blocked.insert(e.decl.name.clone());
            continue;
        }
        known.entry(e.decl.name.clone()).or_insert(e.value.clone());
        blocked.insert(e.decl.name.clone());
    }
    known
}
/// Compile la cascade de l'élément décrit par `facts` dans `class` ; `true` quand la
/// feuille lui génère une boîte `::before` / `::after` (avec `content`), même si
/// l'élément est une feuille qui ne peut pas la porter.
pub(crate) fn compile(
    sheet: &Sheet,
    facts: &Facts,
    module: &str,
    class: &mut Class,
    notes: &mut Vec<String>,
) -> bool {
    let mut keys = vec![Key::Universal, Key::Tag(facts.tag.clone())];
    for c in facts.classes.iter().chain(&facts.dynamic_classes) {
        keys.push(Key::Class(c.clone()));
    }
    for a in facts.attrs.keys().chain(&facts.dynamic_attrs) {
        keys.push(Key::Attr(a.clone()));
    }
    if let Some(id) = facts.attrs.get("id") {
        keys.push(Key::Id(id.clone()));
    }
    let mut candidates = keys
        .iter()
        .filter_map(|k| sheet.index.get(k))
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    let mut boxes: BTreeMap<Option<PseudoElement>, Vec<Entry>> = BTreeMap::new();
    for index in candidates {
        let rule = &sheet.rules[index];
        let tri = if rule.root {
            if facts.tag == "body" {
                Tri::True
            } else {
                Tri::False
            }
        } else {
            selectors::subject_and_context(&rule.complex, Some(facts))
        };
        let mut conditions = rule.conditions.clone();
        match tri {
            Tri::False => continue,
            Tri::True => {}
            Tri::Runtime(code) => conditions.push(code),
        }
        let predicate = match conditions.len() {
            0 => None,
            1 => Some(conditions.remove(0)),
            _ => Some(format!("{STATE}::All(&[{}])", conditions.join(","))),
        };
        // Les déclarations de `html` / `:root` sont héritées par `<body>` : sous toutes
        // ses propres règles.
        let (layer, specificity) = if rule.root {
            (0, (0, 0, 0))
        } else {
            (rule.layer, rule.specificity)
        };
        for (i, decl) in rule.declarations.iter().enumerate() {
            let important_layer = if decl.important {
                u32::MAX - layer
            } else {
                layer
            };
            boxes.entry(rule.complex.element).or_default().push(Entry {
                rank: (
                    decl.important,
                    important_layer,
                    specificity,
                    rule.order,
                    i as u32,
                ),
                rule: index,
                predicate: predicate.clone(),
                decl,
                kind: decl.kind.clone(),
                value: decl.value.clone(),
            });
        }
    }
    let mut host_vars = BTreeMap::new();
    let mut order: u32 = 0;
    for (pseudo, mut entries) in boxes {
        entries.sort_by_key(|e| e.rank);
        let mut own = own_variables(&entries);
        if pseudo.is_none() {
            host_vars = own.clone();
        } else {
            for (k, v) in &host_vars {
                own.entry(k.clone()).or_insert(v.clone());
            }
        }
        // Variables connues : celles de la boîte (résolues entre elles), puis les
        // constantes de la feuille.
        let resolved: BTreeMap<String, String> = own
            .iter()
            .map(|(k, v)| {
                let known = |n: &str| {
                    if n == k {
                        None
                    } else {
                        own.get(n).or(sheet.constants.get(n)).cloned()
                    }
                };
                (k.clone(), values::substitute(v, &known, 0))
            })
            .collect();
        let known = |n: &str| resolved.get(n).or(sheet.constants.get(n)).cloned();
        for e in &mut entries {
            if e.decl.name == "content" {
                e.value = content_value(&e.value, &known);
                e.kind = Kind::Static(e.value.clone());
                continue;
            }
            if matches!(e.kind, Kind::Skip(_) | Kind::Ignored) && !e.decl.value.contains("var(") {
                continue;
            }
            let value = values::substitute(&e.value, &known, 0);
            if value != e.value || matches!(e.kind, Kind::Skip(_)) {
                e.kind = values::classify(&e.decl.name, &value, &sheet.constants);
                e.value = value;
            }
            // Propriété personnalisée lue nulle part : inutile au runtime.
            if matches!(e.kind, Kind::Custom(_)) && !sheet.referenced.contains(&e.decl.name) {
                e.kind = Kind::Ignored;
            }
        }
        if pseudo.is_none() {
            // Boîte en ligne : `display` gagnant (inconditionnel), sinon la balise.
            let display = entries
                .iter()
                .rev()
                .find(|e| e.decl.name == "display" && e.predicate.is_none())
                .map(|e| e.value.trim().to_ascii_lowercase());
            let inline = match display.as_deref() {
                Some(d) => d.starts_with("inline"),
                None => INLINE_TAGS.contains(&facts.tag.as_str()),
            };
            if inline {
                class.props.push(("inline_level".into(), "true".into()));
            }
        }
        if let Some(part @ (PseudoElement::SliderThumb | PseudoElement::SliderTrack)) = pseudo {
            slider_part(&mut entries, part == PseudoElement::SliderThumb);
        }
        cascade(&mut entries);
        emit_box(class, pseudo, &entries, &mut order, notes);
    }
    let generated = class
        .pseudos
        .values()
        .any(|p| p.content.as_deref().is_some_and(|c| c != "none"));
    if facts.leaf {
        crate::css::strip_container(class);
        class.pseudos.clear();
    }
    class.markers = facts.classes.iter().cloned().collect();
    // Faits de l'élément et variables globales de la feuille.
    let attrs = facts
        .attrs
        .iter()
        .map(|(k, v)| format!("({k:?},{v:?})"))
        .collect::<Vec<_>>()
        .join(",");
    class.prelude = format!(
        "let __ui_node=__ui_node.html_facts({:?},&[{attrs}]).style_globals(&{module}::__KARBON_REGISTRY,cx);",
        facts.tag
    );
    generated
}
/// Pose sur `class` la révélation de hauteur au minutage de la transition de hauteur
/// de la classe `name` (`data-reveal=".collapsing"`) ; `false` quand la feuille n'en
/// déclare pas.
pub(crate) fn reveal(sheet: &Sheet, name: &str, class: &mut Class) -> bool {
    match class_transition(sheet, name, "height") {
        Some(code) => {
            class.props.push(("reveal".into(), code));
            true
        }
        None => false,
    }
}
/// Minutage (code `Transition`) de la transition de `property` déclarée par la règle
/// inconditionnelle la plus forte de la classe `name` seule (`.collapsing { transition:
/// height .35s ease }`) ; `None` sans telle déclaration.
fn class_transition(sheet: &Sheet, name: &str, property: &str) -> Option<String> {
    let facts = Facts {
        tag: "div".into(),
        classes: [name.to_owned()].into_iter().collect(),
        ..Default::default()
    };
    let mut best: Option<(Rank, &str)> = None;
    for &index in sheet.index.get(&Key::Class(name.to_owned()))? {
        let rule = &sheet.rules[index];
        if !rule.conditions.is_empty()
            || rule.complex.element.is_some()
            || selectors::subject_and_context(&rule.complex, Some(&facts)) != Tri::True
        {
            continue;
        }
        for (i, decl) in rule.declarations.iter().enumerate() {
            let rank = (
                decl.important,
                rule.layer,
                rule.specificity,
                rule.order,
                i as u32,
            );
            if decl.name == "transition" && best.is_none_or(|(r, _)| rank > r) {
                best = Some((rank, decl.value.as_str()));
            }
        }
    }
    let (_, value) = best?;
    // Élément de la liste qui anime `property` (ou `all`), compilé comme en mode strict.
    let item = value.split(',').find(|item| {
        item.split_whitespace()
            .any(|word| word == property || word == "all")
    })?;
    let props =
        crate::css::values::declaration("transition", item.trim(), &sheet.constants).ok()?;
    props
        .into_iter()
        .find(|(n, _)| n == "transition")
        .map(|(_, code)| code)
}
/// Déclarations d'un pseudo-élément de curseur (`::-webkit-slider-thumb`,
/// `::-webkit-slider-runnable-track`) traduites en propriétés de pièce de l'hôte ; les
/// autres sont sans effet.
fn slider_part(entries: &mut [Entry], thumb: bool) {
    for e in entries {
        if matches!(e.kind, Kind::Skip(_)) && e.value.contains("var(") {
            continue;
        }
        let variant = match (thumb, e.decl.name.as_str()) {
            (true, "background-color" | "background") => "ThumbBackground",
            (true, "width") => "ThumbWidth",
            (true, "height") => "ThumbHeight",
            (true, "border-radius") => "ThumbRadius",
            (true, "border") => "ThumbBorder",
            (true, "box-shadow") => "ThumbShadow",
            (false, "background-color" | "background") => "TrackBackground",
            (false, "height") => "TrackHeight",
            (false, "border-radius") => "TrackRadius",
            (false, "border") => "TrackBorder",
            _ => {
                e.kind = Kind::Ignored;
                continue;
            }
        };
        e.kind = match values::tokenize(&e.value) {
            Ok(tokens) => Kind::Dynamic(variant.into(), tokens),
            Err(m) => Kind::Skip(format!("{}: {m}", e.decl.name)),
        };
    }
}
/// `content` : variables connues substituées, inconnues remplacées par leur repli.
fn content_value(value: &str, known: &dyn Fn(&str) -> Option<String>) -> String {
    let resolved = values::substitute(value, known, 0);
    let fallback = |_: &str| None;
    // Repli des variables restantes : `var(--x, "/")` → `"/"`.
    let mut out = resolved.clone();
    while let Some(start) = out.find("var(") {
        let inner = &out[start + 4..];
        let Some(end) = inner.rfind(')') else { break };
        let args = &inner[..end];
        let replacement = args.split_once(',').map_or(String::new(), |(_, f)| {
            values::substitute(f.trim(), &fallback, 0)
        });
        out = format!("{}{}{}", &out[..start], replacement, &inner[end + 1..]);
    }
    if out.trim().is_empty() {
        "none".into()
    } else {
        out
    }
}
/// Émet les déclarations retenues d'une boîte dans `class`.
fn emit_box(
    class: &mut Class,
    pseudo: Option<PseudoElement>,
    entries: &[Entry],
    order: &mut u32,
    notes: &mut Vec<String>,
) {
    let kind = match pseudo {
        None => None,
        Some(PseudoElement::Before) => Some("before"),
        Some(PseudoElement::After) => Some("after"),
        // Pièces de curseur : propriétés de l'hôte (`ControlPart`), déjà dynamiques.
        Some(PseudoElement::SliderThumb | PseudoElement::SliderTrack) => None,
        Some(PseudoElement::Placeholder) => {
            if !entries.is_empty() {
                notes.push("::placeholder".into());
            }
            return;
        }
    };
    // Une déclaration statique inconditionnelle qui recouvre en partie une déclaration
    // d'exécution de rang inférieur est aussi posée comme règle toujours vraie à son rang.
    let runtime_below = |i: usize| -> BTreeSet<String> {
        entries[..i]
            .iter()
            .filter(|e| e.predicate.is_some() || matches!(e.kind, Kind::Dynamic(..)))
            .flat_map(|e| values::coverage(&e.decl.name))
            .collect()
    };
    let mut i = 0;
    while i < entries.len() {
        // Groupe : déclarations consécutives de la même règle et de même importance.
        let start = i;
        while i < entries.len()
            && entries[i].rule == entries[start].rule
            && entries[i].rank.0 == entries[start].rank.0
        {
            i += 1;
        }
        let group = &entries[start..i];
        *order += 1;
        let rank_order = *order * 64;
        let predicate = group[0]
            .predicate
            .as_ref()
            .map(|p| format!("{STATE}::Where(&{p})"));
        let mut dynamic = Vec::new();
        let mut customs = Vec::new();
        for (j, e) in group.iter().enumerate() {
            match &e.kind {
                Kind::Dynamic(variant, tokens) => dynamic.push(format!(
                    "__ui_root::ui::styling::tokens::DynamicDeclaration{{property:__ui_root::ui::styling::tokens::DynamicProperty::{variant},value:{}}}",
                    values::token_slice(tokens)
                )),
                Kind::Custom(tokens) => customs.push(format!(
                    "({:?},__ui_root::ui::styling::VariableValue::Static({}))",
                    e.decl.name,
                    values::token_slice(tokens)
                )),
                Kind::Static(value) => {
                    // `background-image: none` retire aussi les couches SVG (évaluées à
                    // l'exécution) posées par une règle plus faible.
                    if e.decl.name == "background-image" && value.trim() == "none" {
                        dynamic.push(
                            "__ui_root::ui::styling::tokens::DynamicDeclaration{property:__ui_root::ui::styling::tokens::DynamicProperty::BackgroundImage,value:&[__ui_root::ui::styling::tokens::Token::Ident(\"none\")]}".to_string(),
                        );
                    }
                    let decl = parser::Declaration {
                        name: e.decl.name.clone(),
                        value: value.clone(),
                        at: 0,
                    };
                    let mut local = rank_order + j as u32;
                    let selected = |predicate: Option<String>| selector::Selector {
                        class: String::new(),
                        classes: Vec::new(),
                        state: "combined".into(),
                        predicate,
                        pseudo: kind,
                    };
                    let conditional = predicate.is_some();
                    if let Err((_, m)) = add_rule(
                        class,
                        &selected(predicate.clone()),
                        conditional,
                        std::slice::from_ref(&decl),
                        &Default::default(),
                        &mut local,
                    ) {
                        notes.push(format!("{}: {m}", e.decl.name));
                        continue;
                    }
                    if !conditional {
                        let overlap = values::coverage(&e.decl.name);
                        let below = runtime_below(start + j);
                        if overlap.iter().any(|l| below.contains(l)) {
                            let mut local = rank_order + j as u32;
                            let _ = add_rule(
                                class,
                                &selected(Some(format!("{STATE}::Where(&{ALWAYS})"))),
                                true,
                                std::slice::from_ref(&decl),
                                &Default::default(),
                                &mut local,
                            );
                        }
                    }
                }
                Kind::Ignored | Kind::Skip(_) => {}
            }
        }
        let mut props = Vec::new();
        if !customs.is_empty() {
            if predicate.is_none() && kind.is_none() {
                for c in customs {
                    class.props.push(("custom_property".into(), c));
                }
            } else {
                for c in customs {
                    props.push(("custom_property".to_string(), c));
                }
            }
        }
        if !dynamic.is_empty() {
            props.push((
                "dynamic_declarations".to_string(),
                format!("&[{}]", dynamic.join(",")),
            ));
        }
        if props.is_empty() {
            continue;
        }
        let state = predicate.unwrap_or_else(|| format!("{STATE}::Where(&{ALWAYS})"));
        match kind {
            None => class.state_props.push((state, props, rank_order)),
            Some(kind) => class.pseudos.entry(kind).or_default().state_props.push((
                format!("{STATE}::Parent(&{state})"),
                props,
                rank_order,
            )),
        }
    }
}
