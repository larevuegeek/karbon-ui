//! Évaluation à trois valeurs d'un sélecteur contre les faits statiques d'un élément de
//! template : vrai, faux, ou condition `StyleState` évaluée à l'exécution (états,
//! classes et attributs dynamiques, ancêtres et frères inconnus).
use super::{AttrOp, AttrSel, Combinator, Complex, Compound, Pseudo};
use std::collections::{BTreeMap, BTreeSet};

const STATE: &str = "__ui_root::ui::styling::StyleState";

/// Résultat d'une évaluation statique.
#[derive(Clone, Debug, PartialEq)]
pub enum Tri {
    True,
    False,
    /// Condition à évaluer à l'exécution (code `StyleState`).
    Runtime(String),
}
fn all(parts: Vec<Tri>) -> Tri {
    let mut runtime = Vec::new();
    for p in parts {
        match p {
            Tri::False => return Tri::False,
            Tri::True => {}
            Tri::Runtime(code) => runtime.push(code),
        }
    }
    match runtime.len() {
        0 => Tri::True,
        1 => Tri::Runtime(runtime.remove(0)),
        _ => Tri::Runtime(format!("{STATE}::All(&[{}])", runtime.join(","))),
    }
}
fn any(parts: Vec<Tri>) -> Tri {
    let mut runtime = Vec::new();
    for p in parts {
        match p {
            Tri::True => return Tri::True,
            Tri::False => {}
            Tri::Runtime(code) => runtime.push(code),
        }
    }
    match runtime.len() {
        0 => Tri::False,
        1 => Tri::Runtime(runtime.remove(0)),
        _ => Tri::Runtime(format!("{STATE}::Any(&[{}])", runtime.join(","))),
    }
}
fn not(t: Tri) -> Tri {
    match t {
        Tri::True => Tri::False,
        Tri::False => Tri::True,
        Tri::Runtime(code) => Tri::Runtime(format!("{STATE}::Not(&{code})")),
    }
}
/// Faits connus à la compilation d'un élément de template. `None` : rien n'est connu
/// (ancêtres, frères) et tout est évalué à l'exécution.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub tag: String,
    /// Classes toujours présentes.
    pub classes: BTreeSet<String>,
    /// Classes posées à l'exécution (`class:x={…}`, comportements Bootstrap).
    pub dynamic_classes: BTreeSet<String>,
    /// Attributs littéraux.
    pub attrs: BTreeMap<String, String>,
    /// Attributs dont la valeur n'est connue qu'à l'exécution.
    pub dynamic_attrs: BTreeSet<String>,
    /// Feuille (texte, contrôle) : les déclarations de modèle de conteneur sont ignorées.
    pub leaf: bool,
    /// Ancêtres écrits dans le même template, du parent vers la racine. Leur liste peut
    /// être incomplète (template inclus ailleurs) : elle prouve une relation, jamais son
    /// absence, sauf pour le parent direct (`parent_known`).
    pub ancestors: Vec<Facts>,
    /// Le parent à l'exécution est `ancestors[0]` (l'élément n'est pas la racine du
    /// template).
    pub parent_known: bool,
}
fn attr_matches(sel: &AttrSel, value: &str) -> bool {
    let (v, w) = if sel.insensitive {
        (value.to_ascii_lowercase(), sel.value.to_ascii_lowercase())
    } else {
        (value.to_owned(), sel.value.clone())
    };
    match sel.op {
        AttrOp::Exists => true,
        AttrOp::Equals => v == w,
        AttrOp::Includes => v.split_whitespace().any(|p| p == w),
        AttrOp::Dash => v == w || v.starts_with(&format!("{w}-")),
        AttrOp::Prefix => !w.is_empty() && v.starts_with(&w),
        AttrOp::Suffix => !w.is_empty() && v.ends_with(&w),
        AttrOp::Substring => !w.is_empty() && v.contains(&w),
    }
}
fn attr_code(sel: &AttrSel) -> String {
    let op = match sel.op {
        AttrOp::Exists => "Exists",
        AttrOp::Equals => "Equals",
        AttrOp::Includes => "Includes",
        AttrOp::Dash => "DashMatch",
        AttrOp::Prefix => "Prefix",
        AttrOp::Suffix => "Suffix",
        AttrOp::Substring => "Substring",
    };
    format!(
        "{STATE}::Attribute(&__ui_root::ui::styling::AttributeMatch{{name:{:?},operator:__ui_root::ui::styling::AttributeOperator::{op},value:{:?},case_insensitive:{}}})",
        sel.name, sel.value, sel.insensitive
    )
}
/// Évalue un composé ; `facts` à `None` rend tout dynamique.
fn compound(c: &Compound, facts: Option<&Facts>) -> Tri {
    let mut parts = Vec::new();
    if let Some(tag) = &c.tag {
        parts.push(match facts {
            Some(f) if f.tag == *tag => Tri::True,
            Some(_) => Tri::False,
            None => Tri::Runtime(format!("{STATE}::Tag({tag:?})")),
        });
    }
    for id in &c.ids {
        parts.push(match facts {
            Some(f) => match f.attrs.get("id") {
                Some(v) if v == id => Tri::True,
                Some(_) => Tri::False,
                None if f.dynamic_attrs.contains("id") => Tri::Runtime(attr_code(&AttrSel {
                    name: "id".into(),
                    op: AttrOp::Equals,
                    value: id.clone(),
                    insensitive: false,
                })),
                None => Tri::False,
            },
            None => Tri::Runtime(attr_code(&AttrSel {
                name: "id".into(),
                op: AttrOp::Equals,
                value: id.clone(),
                insensitive: false,
            })),
        });
    }
    for class in &c.classes {
        parts.push(match facts {
            Some(f) if f.classes.contains(class) => Tri::True,
            Some(f) if !f.dynamic_classes.contains(class) && !f.dynamic_classes.contains("*") => {
                Tri::False
            }
            _ => Tri::Runtime(format!("{STATE}::Class({class:?})")),
        });
    }
    for sel in &c.attrs {
        parts.push(match facts {
            Some(f) => match f.attrs.get(&sel.name) {
                Some(v) if attr_matches(sel, v) => Tri::True,
                Some(_) => Tri::False,
                None if f.dynamic_attrs.contains(&sel.name) => Tri::Runtime(attr_code(sel)),
                None => Tri::False,
            },
            None => Tri::Runtime(attr_code(sel)),
        });
    }
    for p in &c.pseudos {
        parts.push(match p {
            Pseudo::State(v) => Tri::Runtime(format!("{STATE}::{v}")),
            Pseudo::Nth { variant, a, b } => Tri::Runtime(format!("{STATE}::{variant}({a},{b})")),
            Pseudo::Root => Tri::Runtime(format!("{STATE}::Root")),
            Pseudo::Never => Tri::False,
            Pseudo::Not(list) => not(any(list.iter().map(|x| complex(x, facts)).collect())),
            Pseudo::Is(list) | Pseudo::Where(list) => {
                any(list.iter().map(|x| complex(x, facts)).collect())
            }
        });
    }
    all(parts)
}
/// Évalue un sélecteur complexe dont `facts` décrit le sujet.
fn complex(x: &Complex, facts: Option<&Facts>) -> Tri {
    if x.element.is_some() {
        // Un pseudo-élément dans `:is()` / `:not()` ne désigne jamais l'élément.
        return Tri::False;
    }
    subject_and_context(x, facts)
}
/// Sujet évalué contre `facts`, ancêtres et frères à l'exécution.
pub fn subject_and_context(x: &Complex, facts: Option<&Facts>) -> Tri {
    let (last, left) = x.parts.split_last().unwrap();
    let subject = compound(&last.1, facts);
    if left.is_empty() || subject == Tri::False {
        return subject;
    }
    // Contexte prouvé par les ancêtres du template : la relation devient statique.
    if let Some(f) = facts {
        match statically(left, last.0, &f.ancestors, f.parent_known) {
            Some(true) => return subject,
            Some(false) => return Tri::False,
            None => {}
        }
    }
    // Contexte gauche : P(0) = composé 0 ; P(i) = composé i ∧ relation(P(i-1)).
    let mut context: Option<Tri> = None;
    for (index, (_, c)) in left.iter().enumerate() {
        let mut here = compound(c, None);
        if let Some(previous) = context.take() {
            let relation = relation(left[index].0, previous);
            here = all(vec![here, relation]);
        }
        context = Some(here);
    }
    let relation = relation(last.0, context.unwrap());
    all(vec![subject, relation])
}
/// Le contexte gauche `parts` (relié au sujet par `combinator`) est-il prouvé (`true`)
/// ou exclu (`false`) par les ancêtres connus ? `None` : à évaluer à l'exécution.
fn statically(
    parts: &[(Combinator, Compound)],
    combinator: Combinator,
    ancestors: &[Facts],
    parent_known: bool,
) -> Option<bool> {
    let ((before, compound_), rest) = parts.split_last()?;
    match combinator {
        Combinator::Child => {
            if !parent_known || ancestors.is_empty() {
                return None;
            }
            let parent = &ancestors[0];
            match compound(compound_, Some(parent)) {
                Tri::False => Some(false),
                Tri::Runtime(_) => None,
                Tri::True if rest.is_empty() => Some(true),
                Tri::True => {
                    statically(rest, *before, &ancestors[1..], parent.parent_known).filter(|v| *v)
                }
            }
        }
        Combinator::Descendant => {
            for (k, ancestor) in ancestors.iter().enumerate() {
                if compound(compound_, Some(ancestor)) == Tri::True
                    && (rest.is_empty()
                        || statically(rest, *before, &ancestors[k + 1..], ancestor.parent_known)
                            == Some(true))
                {
                    return Some(true);
                }
            }
            None
        }
        // Frères : évalués à l'exécution.
        _ => None,
    }
}
fn relation(combinator: Combinator, inner: Tri) -> Tri {
    let code = match inner {
        Tri::False => return Tri::False,
        Tri::True => format!("{STATE}::All(&[])"),
        Tri::Runtime(code) => code,
    };
    let variant = match combinator {
        Combinator::Child => "Parent",
        Combinator::Next => "PreviousSibling",
        Combinator::Subsequent => "AnyPreviousSibling",
        Combinator::Descendant => "Ancestor",
    };
    Tri::Runtime(format!("{STATE}::{variant}(&{code})"))
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use super::*;
    #[test]
    fn static_evaluation() {
        let facts = Facts {
            tag: "button".into(),
            classes: ["btn".to_string(), "btn-primary".into()].into(),
            attrs: [("type".to_string(), "button".to_string())].into(),
            ..Default::default()
        };
        let eval = |t: &str| subject_and_context(&parse(t).unwrap(), Some(&facts));
        assert_eq!(eval(".btn.btn-primary"), Tri::True);
        assert_eq!(
            eval("button:not(:disabled)"),
            Tri::Runtime(format!("{STATE}::Not(&{STATE}::Disabled)"))
        );
        assert_eq!(eval("[type=checkbox]"), Tri::False);
        assert_eq!(
            eval(".btn-check + .btn").clone(),
            Tri::Runtime(format!(
                "{STATE}::PreviousSibling(&{STATE}::Class(\"btn-check\"))"
            ))
        );
        assert_eq!(
            eval(".nav .btn:not(.x)"),
            Tri::Runtime(format!("{STATE}::Ancestor(&{STATE}::Class(\"nav\"))"))
        );
    }
}
