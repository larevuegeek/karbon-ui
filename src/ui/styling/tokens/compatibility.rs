//! Mesure de compatibilité d'une feuille de framework avec le moteur natif.
/// Compatibility of a framework stylesheet with the native engine, measured when
/// [`css_file!`](crate::css_file) compiles it in framework mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compatibility {
    /// Style rules of the sheet (outside false `@supports` / `@media` blocks).
    pub rules: u32,
    /// Rules with at least one accepted selector and one accepted declaration.
    pub rules_applied: u32,
    /// Rules whose selectors and declarations are all accepted.
    pub rules_full: u32,
    /// Selectors (each alternative of a selector list).
    pub selectors: u32,
    /// Selectors the engine can match.
    pub selectors_accepted: u32,
    /// Declarations.
    pub declarations: u32,
    /// Declarations the engine applies (at compile time or while styles resolve).
    pub declarations_accepted: u32,
    /// Accepted declarations evaluated while styles resolve (`var()`, `em`…).
    pub declarations_dynamic: u32,
    /// At-rules.
    pub at_rules: u32,
    /// At-rules evaluated or applied.
    pub at_rules_accepted: u32,
    /// Rules discarded by a condition known to be false at compile time.
    pub rules_inactive: u32,
}
impl Compatibility {
    fn percent(a: u32, b: u32) -> f32 {
        if b == 0 {
            100.
        } else {
            a as f32 * 100. / b as f32
        }
    }
    /// Percentage of rules applied.
    pub fn rules_percent(&self) -> f32 {
        Self::percent(self.rules_applied, self.rules)
    }
    /// Percentage of selectors accepted.
    pub fn selectors_percent(&self) -> f32 {
        Self::percent(self.selectors_accepted, self.selectors)
    }
    /// Percentage of declarations accepted.
    pub fn declarations_percent(&self) -> f32 {
        Self::percent(self.declarations_accepted, self.declarations)
    }
}
