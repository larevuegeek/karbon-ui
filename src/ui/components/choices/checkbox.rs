use super::*;
/// Labelled checkbox controlled by the application: it shows `state` and
/// reports the next value, the application keeps the state.
pub struct Checkbox {
    /// Label drawn next to the box, also used as the accessible name.
    pub label: Arc<str>,
    /// Displayed state (off, on or mixed).
    pub state: CheckState,
    pub(crate) on_change: Handler<bool>,
    /// Whether the checkbox accepts input.
    pub enabled: bool,
    /// Colour overrides.
    pub style: ChoiceStyle,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
}
impl Checkbox {
    /// Checkbox reporting its next value through `on_change` (`true` once checked).
    /// A mixed checkbox becomes checked.
    pub fn new<M: 'static>(
        label: impl Into<Arc<str>>,
        checked: bool,
        on_change: impl Fn(bool) -> M + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            state: if checked {
                CheckState::On
            } else {
                CheckState::Off
            },
            on_change: Handler::new(on_change),
            enabled: true,
            style: ChoiceStyle::default(),
            css: None,
        }
    }
    fn activation(&self) -> Action {
        Action::toggle(&self.on_change, self.state.next() == CheckState::On)
    }
    /// Show an explicit state, for example [`CheckState::Mixed`].
    pub fn state(mut self, state: CheckState) -> Self {
        self.state = state;
        self
    }
    /// Disable the checkbox: it is greyed out and ignores activation.
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
choice_widget!(Checkbox, Role::Checkbox, false, |w: &Checkbox| {
    ChoiceState::Toggle(w.state)
});
