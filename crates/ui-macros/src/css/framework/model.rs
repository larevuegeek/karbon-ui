pub(super) use super::selectors::{Complex, Key, Specificity};
use super::*;
pub(super) use std::collections::{BTreeMap, BTreeSet, HashMap};
pub(super) use std::rc::Rc;

/// Déclaration d'une règle aplatie.
#[derive(Clone, Debug)]
pub struct Decl {
    pub name: String,
    /// Valeur, constantes de la feuille substituées.
    pub value: String,
    pub important: bool,
    /// Classement hors contexte (rapport ; l'élément reclasse après ses variables).
    pub kind: values::Kind,
}
/// Une alternative d'un sélecteur de règle, avec les déclarations de la règle.
#[derive(Clone, Debug)]
pub struct Rule {
    pub complex: Complex,
    pub specificity: Specificity,
    /// Rang de couche pour les déclarations normales (non stratifiées : le plus haut).
    pub layer: u32,
    /// Ordre de la règle dans la feuille.
    pub order: u32,
    /// Conditions `@media` / `@container` évaluées à l'exécution (code `StyleState`).
    pub conditions: Vec<String>,
    pub declarations: Rc<Vec<Decl>>,
    /// Sujet `:root` / `html` / `:host` : appliqué à l'élément `<body>`.
    pub root: bool,
}
/// Feuille de framework compilée.
pub struct Sheet {
    pub rules: Vec<Rule>,
    pub index: HashMap<Key, Vec<usize>>,
    /// Propriétés personnalisées de `:root` et valeurs initiales de `@property`.
    pub globals: BTreeMap<String, String>,
    /// Propriétés enregistrées non héritées (`@property … { inherits: false }`).
    pub non_inherited: BTreeSet<String>,
    /// Variables de valeur unique dans toute la feuille, substituées à la compilation.
    pub constants: BTreeMap<String, String>,
    /// Variables lues par au moins un `var()`.
    pub referenced: BTreeSet<String>,
    pub classes: BTreeSet<String>,
    /// `@keyframes` : nom et texte source.
    pub keyframes: Vec<(String, String)>,
    pub report: Report,
    /// Texte source (positions du rapport).
    pub text: String,
}

/// Feuille en cache : taille et date du fichier, feuille compilée.
pub(super) type Cached = (u64, Option<std::time::SystemTime>, Rc<Sheet>);
thread_local! {
    /// Feuilles déjà compilées pendant la compilation de la crate (chemin, taille, date).
    pub(super) static SHEETS: std::cell::RefCell<HashMap<String, Cached>> =
        std::cell::RefCell::new(HashMap::new());
}
/// Feuille du fichier `path` (absolu), compilée une fois par processus tant qu'elle ne
/// change pas.
pub fn load(path: &str) -> Result<Rc<Sheet>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("Cannot read {path}: {e}"))?;
    let (len, modified) = (meta.len(), meta.modified().ok());
    if let Some(sheet) = SHEETS.with(|s| {
        s.borrow()
            .get(path)
            .filter(|(l, m, _)| *l == len && *m == modified)
            .map(|(_, _, sheet)| sheet.clone())
    }) {
        return Ok(sheet);
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("Cannot read {path}: {e}"))?;
    if text.len() > 8 * 1024 * 1024 {
        return Err("Framework stylesheets are limited to 8 MiB".into());
    }
    // Sans la marque d'ordre des octets, qui deviendrait le nom de balise du premier
    // sélecteur (`:root` n'y serait plus reconnu).
    let sheet = Rc::new(Sheet::compile(
        text.strip_prefix('\u{feff}').unwrap_or(&text),
    ));
    SHEETS.with(|s| {
        s.borrow_mut()
            .insert(path.to_owned(), (len, modified, sheet.clone()))
    });
    Ok(sheet)
}
