use super::*;
#[derive(Clone, PartialEq, Eq)]
/// One entry of a [`Select`](super::Select), identified by an application key.
pub struct SelectOption {
    /// Application key; must be unique within the select.
    pub id: u64,
    /// Displayed label.
    pub label: Arc<str>,
    /// Whether the option can be chosen.
    pub enabled: bool,
}
impl SelectOption {
    /// Enabled option with key `id`.
    pub fn new(id: u64, label: impl Into<Arc<str>>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
        }
    }
    /// Show the option greyed out and prevent choosing it.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}
