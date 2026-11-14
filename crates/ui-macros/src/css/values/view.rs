//! Couleurs des dégradés et des ombres qui lisent des variables : la valeur de `:root`
//! compilée sert de repli, les variables du contexte (thème de l'application) sont lues à
//! la construction de la vue.
use super::*;
use std::cell::RefCell;
thread_local! {
    static VARS: RefCell<Option<BTreeMap<String, String>>> = const { RefCell::new(None) };
}
/// Exécute `f`, les couleurs de peinture (arrêts de dégradé, ombres) gardant leurs `var()`.
pub(super) fn with_vars<T>(vars: &BTreeMap<String, String>, f: impl FnOnce() -> T) -> T {
    VARS.with(|v| *v.borrow_mut() = Some(vars.clone()));
    let result = f();
    VARS.with(|v| *v.borrow_mut() = None);
    result
}
/// Exécute `f` sans les variables de [`with_vars`] (valeur déjà résolue).
pub(super) fn without_vars<T>(f: impl FnOnce() -> T) -> T {
    let saved = VARS.with(|v| v.borrow_mut().take());
    let result = f();
    VARS.with(|v| *v.borrow_mut() = saved);
    result
}
/// Variables actives ([`with_vars`]).
pub(super) fn variables() -> BTreeMap<String, String> {
    VARS.with(|v| v.borrow().clone()).unwrap_or_default()
}
/// Variables actives si `word` utilise `var()`.
fn active(word: &str) -> Option<BTreeMap<String, String>> {
    if !word.contains("var(") {
        return None;
    }
    VARS.with(|v| v.borrow().clone())
}
/// Code d'une couleur de peinture : constante, ou lue dans le contexte à la construction.
pub(super) fn paint_color(word: &str) -> Result<String> {
    let Some(vars) = active(word) else {
        return color(word);
    };
    let fixed = color(&resolve(word, &vars)?)?;
    Ok(
        match colors::color_property(word, Some(&vars))?.strip_prefix("dynamic:") {
            Some(expression) => format!("cx.variable_expression({expression},{fixed})"),
            None => fixed,
        },
    )
}
/// Vrai pendant [`with_vars`] pour une valeur utilisant `var()`.
pub(super) fn reads_variables(value: &str) -> bool {
    active(value).is_some()
}
