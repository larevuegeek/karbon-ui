//! Classe compilée : déclarations accumulées par les règles qui la visent (base, états,
//! pseudo-éléments, enfants, animation) avant son rendu en fonction de style.
mod render;
mod rules;
mod states;
mod utilities;
use super::values;
use crate::codegen;
pub(crate) use render::render_framework_class;
pub(in crate::css) use render::{emit, render_class};
pub(in crate::css) use rules::add_rule;
use states::state_properties;
use std::collections::BTreeMap;
pub(crate) use utilities::{UtilityClass, utility_class};
/// Applique `display` (flex ou grille) aux propriétés compilées, comme pour une classe.
pub(in crate::css) fn display(props: &mut Vec<(String, String)>, display: &str) {
    // A declaration belonging to the other layout model is inapplicable.
    if display == "flex" {
        props.retain(|(name, _)| {
            !["columns", "grid_tracks", "grid_auto_repeat"].contains(&name.as_str())
        });
        props.insert(0, ("flex_direction".into(), "\"row\"".into()));
        props.insert(1, ("css_flex".into(), "true".into()));
    } else {
        props.retain(|(name, _)| name != "flex_direction");
        props.insert(0, ("columns".into(), "1".into()));
    }
}
/// `display` d'une règle de base : modèle flex/grille, ou `None` pour un bloc,
/// `-webkit-box` (avec line-clamp) et `none` (masqué).
pub(in crate::css) fn base_display(value: &str) -> std::result::Result<Option<String>, String> {
    match value {
        "flex" | "inline-flex" => Ok(Some("flex".into())),
        "grid" | "inline-grid" => Ok(Some("grid".into())),
        // `inline` : un élément du moteur est une boîte (le texte en ligne est porté
        // par les paragraphes) ; comme dans un conteneur flex ou grille, il est traité en
        // bloc. `inline-block` garde sa largeur naturelle dans un bloc (`inline_level`).
        "block" | "-webkit-box" | "none" | "inline" | "inline-block" => Ok(None),
        _ => Err(format!(
            "display: {value} is not implemented; use block, flex, inline-flex, grid, inline-grid or none"
        )),
    }
}
/// Règles d'un pseudo-élément `::before` / `::after` d'une classe.
#[derive(Default)]
pub(in crate::css) struct Pseudo {
    pub(in crate::css) content: Option<String>,
    pub(in crate::css) display: Option<String>,
    pub(in crate::css) props: Vec<(String, String)>,
    pub(in crate::css) state_props: Vec<ConditionalProperties>,
}
#[derive(Default)]
pub(crate) struct Class {
    pub(in crate::css) original: String,
    /// Autres noms de classe portés par l'élément (utilitaires Tailwind fusionnés).
    pub(in crate::css) markers: Vec<String>,
    pub(in crate::css) pseudos: BTreeMap<&'static str, Pseudo>,
    pub(in crate::css) props: Vec<(String, String)>,
    pub(in crate::css) hover: BTreeMap<String, String>,
    pub(in crate::css) active: BTreeMap<String, String>,
    pub(in crate::css) state_props: Vec<ConditionalProperties>,
    /// Déclarations sous `@media` de thème ou de contraste que le moteur ne sait pas
    /// résoudre par règle d'état (dégradés) : appliquées à la construction de la vue si
    /// le test (code booléen) réussit, la vue étant refaite quand ces préférences changent.
    pub(in crate::css) view_rules: Vec<(String, values::Properties)>,
    /// Règles posées sur les enfants (prédicat évalué sur chaque enfant).
    pub(in crate::css) child_props: Vec<ConditionalProperties>,
    pub(in crate::css) display: Option<String>,
    /// `display` de la règle de base : 0 bloc, 1 flex, 2 grille (`None` : absent).
    pub(in crate::css) display_kind: Option<i8>,
    pub(in crate::css) animation: BTreeMap<String, String>,
    /// Appels posés avant les propriétés (faits HTML d'un élément de framework).
    pub(in crate::css) prelude: String,
}
pub(in crate::css) type ConditionalProperties = (String, values::Properties, u32);
/// Ordre de déclaration des règles d'une feuille : au-dessus de celui des utilitaires
/// Tailwind (`tailwind::ORDER_LIMIT`), comme une feuille hors `@layer` l'emporte sur la
/// couche `utilities`.
pub(in crate::css) const SHEET_ORDER: u32 = 1 << 24;
/// Classe vide recevant la cascade d'un élément de framework.
pub(crate) fn framework_class() -> Class {
    Class::default()
}
/// Retire les déclarations de modèle de conteneur (flex, grille) d'une classe posée sur
/// une feuille (texte, contrôle), qui n'a pas d'enfants à disposer.
pub(crate) fn strip_container(class: &mut Class) {
    const CONTAINER: [&str; 6] = [
        "grid_auto_repeat",
        "flex_direction",
        "columns",
        "grid_tracks",
        "grid_rows",
        "css_flex",
    ];
    class.display = None;
    class
        .props
        .retain(|(name, _)| !CONTAINER.contains(&codegen::canonical(name)));
    for (_, props, _) in class
        .state_props
        .iter_mut()
        .chain(class.child_props.iter_mut())
    {
        props.retain(|(name, _)| !CONTAINER.contains(&codegen::canonical(name)));
    }
    class.state_props.retain(|(_, props, _)| !props.is_empty());
}
/// `display` de base d'une classe compilée (`-1` : non déclaré).
pub(crate) fn display_kind(class: &Class) -> i8 {
    class.display_kind.unwrap_or(-1)
}
/// Opacité déclarée d'un élément de framework animé par `@keyframes` (valeur des étapes
/// extrêmes qui l'omettent), `None` sans animation ou sans opacité propre.
pub(crate) fn framework_animated_opacity(class: &Class) -> Option<String> {
    class.animation.get("animation-name")?;
    class
        .props
        .iter()
        .rev()
        .find(|(n, _)| n == "opacity")
        .map(|(_, v)| v.clone())
}
/// Animation infinie qui pilote l'opacité : en CSS elle remplace l'opacité de l'élément,
/// alors que le moteur compose l'animation avec la base. L'opacité propre est retirée
/// (`.spinner-grow`, `opacity: 0` hors de son étape `50%`).
pub(crate) fn framework_drop_animated_opacity(
    class: &mut Class,
    animations: &BTreeMap<String, String>,
) {
    let infinite = class
        .animation
        .get("animation-iteration-count")
        .is_some_and(|c| c == "infinite");
    let animates_opacity = class
        .animation
        .get("animation-name")
        .and_then(|n| animations.get(n))
        .is_some_and(|frames| frames.contains(".opacity("));
    if infinite && animates_opacity {
        class.props.retain(|(n, _)| n != "opacity");
    }
}
