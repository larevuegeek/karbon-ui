//! `data-reveal=".classe"` : révélation de hauteur au minutage de la transition de
//! hauteur qu'une classe de la feuille déclare (`.collapsing` de Bootstrap).
use super::Sheet;
use super::element::Rank;
use super::selectors::{self, Facts, Key, Tri};
use crate::css::Class;

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
