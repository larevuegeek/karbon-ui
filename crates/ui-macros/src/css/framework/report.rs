//! Rapport de compatibilité d'une feuille de framework : compteurs des règles,
//! sélecteurs, déclarations et at-rules acceptés, catégories des éléments sautés.
//! Il alimente les constantes `COMPATIBILITY` / `REPORT` du module généré et
//! l'option `report` de `css_file!`.
use std::collections::BTreeMap;

/// Compteurs du rapport de compatibilité.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub rules: u32,
    pub rules_full: u32,
    pub rules_applied: u32,
    pub selectors: u32,
    pub selectors_accepted: u32,
    pub declarations: u32,
    pub declarations_accepted: u32,
    /// Déclarations acceptées évaluées à l'exécution (var(), em…).
    pub declarations_dynamic: u32,
    pub at_rules: u32,
    pub at_rules_accepted: u32,
    /// Règles écartées par une condition fausse à la compilation (`@supports`,
    /// `@media print`…) : ni acceptées ni refusées.
    pub rules_inactive: u32,
    /// Catégories des éléments sautés et leur nombre.
    pub skipped: BTreeMap<String, u32>,
    /// Détail (position, catégorie, texte) quand la liste est demandée.
    pub details: Vec<(usize, String, String)>,
}
fn percent(a: u32, b: u32) -> f64 {
    if b == 0 {
        100.
    } else {
        a as f64 * 100. / b as f64
    }
}
impl Report {
    pub(super) fn skip(&mut self, at: usize, category: impl Into<String>, text: &str) {
        let category = category.into();
        *self.skipped.entry(category.clone()).or_default() += 1;
        if self.details.len() < 20_000 {
            self.details
                .push((at, category, text.chars().take(160).collect()));
        }
    }
    /// Résumé lisible : pourcentages et principales catégories sautées.
    pub fn summary(&self, name: &str, top: usize) -> String {
        let mut out = format!(
            "{name}: rules {}/{} applied ({:.1}%), {:.1}% fully; selectors {}/{} ({:.1}%); declarations {}/{} ({:.1}%, {} evaluated at run time); at-rules {}/{} ({:.1}%); {} rules inactive (false @supports/@media)\n",
            self.rules_applied,
            self.rules,
            percent(self.rules_applied, self.rules),
            percent(self.rules_full, self.rules),
            self.selectors_accepted,
            self.selectors,
            percent(self.selectors_accepted, self.selectors),
            self.declarations_accepted,
            self.declarations,
            percent(self.declarations_accepted, self.declarations),
            self.declarations_dynamic,
            self.at_rules_accepted,
            self.at_rules,
            percent(self.at_rules_accepted, self.at_rules),
            self.rules_inactive,
        );
        let mut categories = self.skipped.iter().collect::<Vec<_>>();
        categories.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (category, count) in categories.into_iter().take(top) {
            out += &format!("  {count:>5}  {category}\n");
        }
        out
    }
}
