use super::*;
/// Labelled on/off switch controlled by the application: it shows `checked`
/// and reports the next value, the application keeps the state.
pub struct Switch {
    /// Label drawn next to the switch, also used as the accessible name.
    pub label: Arc<str>,
    /// Displayed state.
    pub checked: bool,
    pub(crate) on_change: Handler<bool>,
    /// Whether the switch accepts input.
    pub enabled: bool,
    /// Colour overrides.
    pub style: ChoiceStyle,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
}
impl Switch {
    /// Switch reporting its next value through `on_change`.
    pub fn new<M: 'static>(
        label: impl Into<Arc<str>>,
        checked: bool,
        on_change: impl Fn(bool) -> M + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            checked,
            on_change: Handler::new(on_change),
            enabled: true,
            style: ChoiceStyle::default(),
            css: None,
        }
    }
    fn activation(&self) -> Action {
        Action::toggle(&self.on_change, !self.checked)
    }
    /// Disable the switch: it is greyed out and ignores activation.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Override colours.
    pub fn style(mut self, style: ChoiceStyle) -> Self {
        self.style = style;
        self
    }
}
choice_widget!(
    Switch,
    Role::Switch,
    true,
    |w: &Switch| ChoiceState::Toggle(if w.checked {
        CheckState::On
    } else {
        CheckState::Off
    })
);
