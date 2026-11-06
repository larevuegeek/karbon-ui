//! Ensemble d'utilitaires Tailwind d'un élément compilé en une classe : groupes de
//! déclarations résolues (voir [`crate::tailwind`]) ajoutés comme des règles.
use super::{Class, add_rule, render_class, state_properties};
use crate::css::{animation, media, parser, selector, values};
use crate::{Result, codegen::Output};
use proc_macro::Span;
use std::collections::BTreeMap;
/// Ensemble d'utilitaires Tailwind d'un élément, compilé en une classe (sans jetons).
pub(crate) struct UtilityClass {
    class: Class,
    animations: BTreeMap<String, String>,
}
/// Construit la classe d'un ensemble d'utilitaires à partir de ses groupes de
/// déclarations résolues (voir [`crate::tailwind`]).
pub(crate) fn utility_class(
    groups: &[crate::tailwind::Group],
    markers: Vec<String>,
    vars: &BTreeMap<String, String>,
) -> std::result::Result<UtilityClass, String> {
    let mut class = Class {
        markers,
        ..Default::default()
    };
    let mut animations = BTreeMap::new();
    for group in groups {
        let mut conditions = group
            .media
            .iter()
            .map(|q| media::condition(q))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if let Some(child) = &group.child {
            // Enfants qui vérifient `child` ; les états de l'élément deviennent ceux du
            // parent.
            const STATE: &str = "__ui_root::ui::styling::StyleState";
            let mut predicate = vec![child.clone()];
            predicate.extend(conditions);
            if !group.states.is_empty() {
                predicate.push(format!(
                    "{STATE}::Parent(&{STATE}::All(&[{}]))",
                    group.states.join(",")
                ));
            }
            let mut props = Vec::new();
            for (name, value) in &group.declarations {
                props.extend(values::declaration(name, value, vars)?);
            }
            state_properties(&props)?;
            class.child_props.push((
                format!("{STATE}::All(&[{}])", predicate.join(",")),
                props,
                group.order,
            ));
            continue;
        }
        conditions.extend(group.states.iter().cloned());
        let predicate = match conditions.len() {
            0 => None,
            1 => conditions.pop(),
            _ => Some(format!(
                "__ui_root::ui::styling::StyleState::All(&[{}])",
                conditions.join(",")
            )),
        };
        let selected = selector::Selector {
            class: String::new(),
            classes: Vec::new(),
            state: "combined".into(),
            predicate,
            pseudo: group.pseudo,
        };
        let declarations = group
            .declarations
            .iter()
            .map(|(name, value)| parser::Declaration {
                name: name.clone(),
                value: value.clone(),
                at: 0,
            })
            .collect::<Vec<_>>();
        let mut order = group.order;
        add_rule(
            &mut class,
            &selected,
            !group.media.is_empty(),
            &declarations,
            vars,
            &mut order,
        )
        .map_err(|(_, m)| m)?;
        for text in &group.keyframes {
            for item in parser::parse(text).map_err(|e| e.message)? {
                if let parser::Item::Keyframes { name, frames, .. } = item {
                    animations.insert(name, animation::frames(&frames, vars)?);
                }
            }
        }
    }
    if !class.animation.is_empty() {
        animation::instance(&class.animation, &animations)?;
    }
    for (_, props, _) in class
        .state_props
        .iter()
        .chain(class.pseudos.values().flat_map(|p| &p.state_props))
    {
        state_properties(props)?;
    }
    Ok(UtilityClass { class, animations })
}
impl UtilityClass {
    /// `display` de base de l'ensemble (`-1` : non déclaré), lu par `blockifies`.
    pub(crate) fn display_kind(&self) -> i8 {
        self.class.display_kind.unwrap_or(-1)
    }
    /// Fonction de style `name` de l'ensemble.
    pub(crate) fn render(
        self,
        name: &str,
        out: &mut Output,
        literals: &Output,
        span: Span,
    ) -> Result<String> {
        render_class(name, self.class, &self.animations, out, literals, span)
    }
}
