use super::*;
// Pour `framework` : ajout d'une règle à une classe, comme dans une feuille stricte.
pub(super) use super::class::add_rule;

/// Couleur d'un attribut SVG : constante (sRGB 0..1 et alpha), ou `None` pour
/// `currentColor` ; les variables et mélanges dépendant de l'exécution sont refusés.
pub fn svg_color(value: &str) -> std::result::Result<Option<[f32; 4]>, String> {
    values::constant_color(value)
}
/// Traduction d'une déclaration par le compilateur CSS (propriétés du moteur, code).
#[cfg(test)]
pub(crate) fn values_declaration(
    name: &str,
    value: &str,
    vars: &std::collections::BTreeMap<String, String>,
) -> std::result::Result<values::Properties, String> {
    values::declaration(name, value, vars)
}
/// Mots d'une valeur CSS, hors fonctions et chaînes.
pub(crate) fn values_words(value: &str) -> Vec<String> {
    values::words(value).unwrap_or_default()
}
/// Condition compilée d'une requête `@media` (ou `@container …`).
pub(crate) fn media_condition(query: &str) -> std::result::Result<String, String> {
    media::condition(query)
}
/// Prédicat d'un sélecteur dont le sujet (dernier composé) est la classe `subject`,
/// et son pseudo-élément éventuel : variantes arbitraires de Tailwind (`[.a+&]`).
pub(crate) fn subject_predicate(
    css: &str,
    subject: &str,
) -> std::result::Result<(Option<String>, Option<&'static str>), String> {
    let selected = selector::parse(css)?;
    if selected.class != subject {
        return Err("& must be the subject: the first class of the last compound".into());
    }
    if selected.pseudo == Some("scrollbar") {
        return Err("::-webkit-scrollbar is not available as a variant".into());
    }
    Ok((selected.predicate, selected.pseudo))
}
/// Prédicat d'un composé simple (`div:last-child`, `.ligne`, `a[href]`, `*`).
pub(crate) fn compound_predicate(text: &str) -> std::result::Result<String, String> {
    selector::compound_predicate(text)
}
/// Prédicat d'un sélecteur d'attribut sans crochets (`data-state=open`).
pub(crate) fn attribute_predicate(selector: &str) -> std::result::Result<String, String> {
    selector::attribute(selector)
}
/// Symbole Rust d'une animation `@keyframes` dans le module `keyframes` généré.
pub fn keyframes_symbol(name: &str) -> std::result::Result<String, String> {
    parser::class_name(name)
        .map_err(|_| format!("Unsupported @keyframes name '{name}'; use letters, digits, _ and -"))
}
