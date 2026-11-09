//! Typed application-controlled variables, shared by templates without runtime CSS parsing.
use crate::ui::{
    element::{Insets, Length},
    theme::Color,
};
use std::{borrow::Cow, collections::HashMap, sync::Arc};
/// Typed value of a CSS custom property (`--name`).
#[derive(Clone, Debug, PartialEq)]
pub enum VariableValue {
    /// A color.
    Color(Color),
    /// A length (pixels, percentage, computed expression…).
    Length(Length),
    /// A unitless number.
    Number(f32),
    /// A font family name.
    Family(&'static str),
    /// Edge insets (padding, margin…).
    Insets(Insets),
    /// Pre-tokenized value declared by a framework stylesheet; its `var()` groups are
    /// substituted when the element declaring it resolves its styles.
    Static(&'static [super::tokens::Token]),
    /// Computed value of a framework custom property (`var()` already substituted).
    Tokens(Arc<[super::tokens::Token]>),
}
pub(crate) type VariableValues = HashMap<Cow<'static, str>, VariableValue>;
impl From<f32> for VariableValue {
    fn from(value: f32) -> Self {
        Self::Number(value)
    }
}
impl From<f64> for VariableValue {
    fn from(value: f64) -> Self {
        Self::Number(value as f32)
    }
}
impl From<i32> for VariableValue {
    fn from(value: i32) -> Self {
        Self::Number(value as f32)
    }
}
impl From<u32> for VariableValue {
    fn from(value: u32) -> Self {
        Self::Number(value as f32)
    }
}
impl From<usize> for VariableValue {
    fn from(value: usize) -> Self {
        Self::Number(value as f32)
    }
}
impl From<Color> for VariableValue {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}
impl From<Length> for VariableValue {
    fn from(value: Length) -> Self {
        Self::Length(value)
    }
}
impl From<Insets> for VariableValue {
    fn from(value: Insets) -> Self {
        Self::Insets(value)
    }
}
impl From<&'static str> for VariableValue {
    fn from(value: &'static str) -> Self {
        Self::Family(value)
    }
}
/// Set of CSS custom properties provided by the application.
///
/// Cloning is cheap: values are shared and copied on write.
#[derive(Clone, Default)]
pub struct CssVariables {
    pub(crate) values: Arc<VariableValues>,
    /// Valeurs de plus faible priorité (variables de l'application sous les propriétés
    /// héritées des ancêtres).
    pub(crate) fallback: Option<Arc<VariableValues>>,
    /// Portée de l'ancêtre (propriétés héritées), lue après `values` : un élément qui
    /// déclare des propriétés ne recopie pas celles de ses ancêtres.
    pub(crate) parent: Option<Arc<CssVariables>>,
    /// Profondeur de la chaîne `parent`.
    pub(crate) depth: u16,
}
impl CssVariables {
    /// Sets variable `name` and returns its previous value, if any.
    ///
    /// # Panics
    ///
    /// Panics if `name` does not start with `--` or is just `--`.
    pub fn set(&mut self, name: impl Into<String>, value: VariableValue) -> Option<VariableValue> {
        let name = name.into();
        assert!(
            name.starts_with("--") && name.len() > 2,
            "CSS variable names start with --"
        );
        Arc::make_mut(&mut self.values).insert(Cow::Owned(name), value)
    }
    /// Pose une propriété sans allouer son nom (déclarations compilées).
    pub(crate) fn set_static(&mut self, name: &'static str, value: VariableValue) {
        Arc::make_mut(&mut self.values).insert(Cow::Borrowed(name), value);
    }
    /// Pose une propriété déjà nommée (fusion de deux tables).
    pub(crate) fn set_named(&mut self, name: Cow<'static, str>, value: VariableValue) {
        Arc::make_mut(&mut self.values).insert(name, value);
    }
    /// Portée enfant vide au-dessus de `parent` ; la chaîne est aplatie au-delà de
    /// 16 niveaux pour borner le coût des lectures.
    pub(crate) fn child(parent: Option<&Self>) -> Self {
        let Some(parent) = parent else {
            return Self::default();
        };
        if parent.depth >= 16 {
            let mut flat = VariableValues::new();
            parent.collect(&mut flat);
            return Self {
                values: Arc::new(VariableValues::new()),
                fallback: parent.fallback.clone(),
                parent: Some(Arc::new(Self {
                    values: Arc::new(flat),
                    fallback: None,
                    parent: None,
                    depth: 0,
                })),
                depth: 1,
            };
        }
        Self {
            values: Arc::new(VariableValues::new()),
            fallback: parent.fallback.clone(),
            parent: Some(Arc::new(parent.clone())),
            depth: parent.depth + 1,
        }
    }
    /// Valeurs visibles de la chaîne (hors repli), la plus proche l'emportant.
    fn collect(&self, out: &mut VariableValues) {
        if let Some(parent) = &self.parent {
            parent.collect(out);
        }
        for (k, v) in self.values.iter() {
            out.insert(k.clone(), v.clone());
        }
    }
    /// Sets a color variable. Panics on an invalid name, like [`set`](Self::set).
    pub fn set_color(&mut self, name: impl Into<String>, value: Color) {
        self.set(name, VariableValue::Color(value));
    }
    /// Sets a length variable. Panics on an invalid name, like [`set`](Self::set).
    pub fn set_length(&mut self, name: impl Into<String>, value: impl Into<Length>) {
        self.set(name, VariableValue::Length(value.into()));
    }
    /// Sets a number variable. Panics on an invalid name, like [`set`](Self::set).
    pub fn set_number(&mut self, name: impl Into<String>, value: f32) {
        self.set(name, VariableValue::Number(value));
    }
    /// Sets a font family variable. Panics on an invalid name, like [`set`](Self::set).
    pub fn set_family(&mut self, name: impl Into<String>, value: &'static str) {
        self.set(name, VariableValue::Family(value));
    }
    /// Sets an insets variable. Panics on an invalid name, like [`set`](Self::set).
    pub fn set_insets(&mut self, name: impl Into<String>, value: impl Into<Insets>) {
        self.set(name, VariableValue::Insets(value.into()));
    }
    /// Removes variable `name` and returns its value, if it was set.
    pub fn remove(&mut self, name: &str) -> Option<VariableValue> {
        Arc::make_mut(&mut self.values).remove(name)
    }
    /// Returns the value of variable `name`, if set.
    pub fn get(&self, name: &str) -> Option<&VariableValue> {
        self.declared(name)
            .or_else(|| self.fallback.as_ref()?.get(name))
    }
    /// Valeur de la chaîne des propriétés déclarées, sans le repli.
    fn declared(&self, name: &str) -> Option<&VariableValue> {
        let mut scope = self;
        loop {
            if let Some(v) = scope.values.get(name) {
                return Some(v);
            }
            scope = scope.parent.as_deref()?;
        }
    }
    /// Portée d'un élément : `declared` (propriétés héritées) au-dessus de `app`.
    pub(crate) fn scope(declared: &Self, app: Option<&Self>) -> Self {
        Self {
            values: Arc::clone(&declared.values),
            fallback: app.map(|a| Arc::clone(&a.values)),
            parent: declared.parent.clone(),
            depth: declared.depth,
        }
    }
    /// Même contenu visible (valeurs et repli).
    pub(crate) fn same(&self, other: &Self) -> bool {
        let eq = |a: &Arc<VariableValues>, b: &Arc<VariableValues>| Arc::ptr_eq(a, b) || a == b;
        eq(&self.values, &other.values)
            && match (&self.fallback, &other.fallback) {
                (Some(a), Some(b)) => eq(a, b),
                (None, None) => true,
                _ => false,
            }
            && match (&self.parent, &other.parent) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a.same(b),
                (None, None) => true,
                _ => false,
            }
    }
}
