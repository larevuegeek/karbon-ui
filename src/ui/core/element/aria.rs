//! États ARIA déclarés par l'application : `aria-pressed`, `aria-checked`,
//! `aria-selected` et `aria-current`. Le moteur ne les calcule pas, il les expose tels
//! quels aux technologies d'assistance, comme un navigateur.
use super::*;

/// Kind of current item declared by `aria-current`: the element represents the current
/// page, step, location, date or time within a set of related elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentItem {
    /// Current page of a navigation (`aria-current="page"`).
    Page,
    /// Current step of a process (`aria-current="step"`).
    Step,
    /// Current location in a flow chart or a map (`aria-current="location"`).
    Location,
    /// Current date in a calendar (`aria-current="date"`).
    Date,
    /// Current time in a timetable (`aria-current="time"`).
    Time,
    /// Current item of a set, without more detail (`aria-current="true"`).
    Item,
}
impl CurrentItem {
    /// Token of the `aria-current` attribute: `page`, `step`, `location`, `date`, `time`
    /// or `true`.
    pub fn token(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Step => "step",
            Self::Location => "location",
            Self::Date => "date",
            Self::Time => "time",
            Self::Item => "true",
        }
    }
    /// Value of an `aria-current` token; `false` and the empty string mean no current
    /// item (`None`), an unknown token counts as `true`, as in HTML.
    pub fn parse(token: &str) -> Option<Self> {
        Some(match token.trim() {
            "" | "false" => return None,
            "page" => Self::Page,
            "step" => Self::Step,
            "location" => Self::Location,
            "date" => Self::Date,
            "time" => Self::Time,
            _ => Self::Item,
        })
    }
}

/// États déclarés, regroupés : quelques octets par élément.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AriaStates {
    /// `aria-pressed` : bouton bascule.
    pub(crate) pressed: Option<CheckState>,
    /// `aria-checked` : case, interrupteur ou bouton radio dessinés par l'application.
    pub(crate) checked: Option<ChoiceState>,
    /// `aria-selected` : onglet ou option.
    pub(crate) selected: Option<bool>,
    /// `aria-current` : élément courant d'un ensemble (page d'une navigation…).
    pub(crate) current: Option<CurrentItem>,
}
impl AriaStates {
    /// État de choix exposé : `aria-checked`, sinon `aria-pressed` (bouton bascule),
    /// sinon `aria-selected` (élément d'une sélection unique : onglet, option).
    pub(crate) fn choice(&self) -> Option<ChoiceState> {
        self.checked
            .or(self.pressed.map(ChoiceState::Toggle))
            .or(self.selected.map(ChoiceState::Radio))
    }
}

impl Element {
    /// Declare a toggle button and its state, like HTML `aria-pressed`: assistive
    /// technologies announce a toggle button, pressed (`On`), not pressed (`Off`) or
    /// mixed, and their *Toggle* command activates the element. `None` removes it.
    pub fn aria_pressed(mut self, state: Option<CheckState>) -> Self {
        self.aria.pressed = state;
        self
    }
    /// Declare the checked state of a check box, switch or radio button drawn by the
    /// application (`role="checkbox"`, `"switch"` or `"radio"` with `aria-checked`):
    /// [`ChoiceState::Toggle`] for a check box or switch, [`ChoiceState::Radio`] for a
    /// radio button. Assistive technologies toggle or select it by activating the
    /// element. `None` removes it.
    pub fn aria_checked(mut self, state: Option<ChoiceState>) -> Self {
        self.aria.checked = state;
        self
    }
    /// Declare whether this tab or option is selected, like HTML `aria-selected`;
    /// selecting it from an assistive technology activates the element. `None` removes
    /// it.
    pub fn aria_selected(mut self, selected: Option<bool>) -> Self {
        self.aria.selected = selected;
        self
    }
    /// Declare that this element is the current item of a set (current page of a
    /// navigation…), like HTML `aria-current`. `None` removes it.
    pub fn aria_current(mut self, current: Option<CurrentItem>) -> Self {
        self.aria.current = current;
        self
    }
}
