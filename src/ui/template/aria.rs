//! Valeurs des états ARIA écrites en expressions Rust dans les gabarits :
//! `aria-pressed={on}`, `aria-current={actif.then_some("page")}`.
use crate::ui::element::{CheckState, CurrentItem};

/// Value of `aria-pressed={…}` or `aria-checked={…}`: a `bool` or a [`CheckState`]
/// (for the mixed state).
pub trait AriaCheck {
    /// Declared state.
    fn check_state(self) -> CheckState;
}
impl AriaCheck for bool {
    fn check_state(self) -> CheckState {
        if self {
            CheckState::On
        } else {
            CheckState::Off
        }
    }
}
impl AriaCheck for &bool {
    fn check_state(self) -> CheckState {
        (*self).check_state()
    }
}
impl AriaCheck for CheckState {
    fn check_state(self) -> CheckState {
        self
    }
}
impl AriaCheck for &CheckState {
    fn check_state(self) -> CheckState {
        *self
    }
}
/// Converts an `aria-pressed={…}` or `aria-checked={…}` value.
pub fn aria_check(value: impl AriaCheck) -> CheckState {
    value.check_state()
}
/// Token of an `aria-pressed` or `aria-checked` attribute (`true`, `false` or `mixed`),
/// as read by attribute selectors (`[aria-pressed="true"]`).
pub fn aria_check_token(state: CheckState) -> &'static str {
    match state {
        CheckState::On => "true",
        CheckState::Off => "false",
        CheckState::Mixed => "mixed",
    }
}

/// Value of `aria-current={…}`: a token as in HTML (`"page"`, `"step"`, `"true"`,
/// `"false"`…), an optional token (`None` is like an absent attribute, as `undefined`
/// in a web framework), a `bool` or a [`CurrentItem`].
pub trait AriaCurrent {
    /// Declared current item, `None` when the element is not current.
    fn current_item(self) -> Option<CurrentItem>;
}
impl AriaCurrent for bool {
    fn current_item(self) -> Option<CurrentItem> {
        self.then_some(CurrentItem::Item)
    }
}
impl AriaCurrent for &str {
    fn current_item(self) -> Option<CurrentItem> {
        CurrentItem::parse(self)
    }
}
impl AriaCurrent for &String {
    fn current_item(self) -> Option<CurrentItem> {
        CurrentItem::parse(self)
    }
}
impl AriaCurrent for String {
    fn current_item(self) -> Option<CurrentItem> {
        CurrentItem::parse(&self)
    }
}
impl AriaCurrent for Option<&str> {
    fn current_item(self) -> Option<CurrentItem> {
        self.and_then(CurrentItem::parse)
    }
}
impl AriaCurrent for &&str {
    fn current_item(self) -> Option<CurrentItem> {
        CurrentItem::parse(self)
    }
}
impl AriaCurrent for &bool {
    fn current_item(self) -> Option<CurrentItem> {
        (*self).current_item()
    }
}
impl AriaCurrent for &Option<&str> {
    fn current_item(self) -> Option<CurrentItem> {
        self.and_then(CurrentItem::parse)
    }
}
impl AriaCurrent for &CurrentItem {
    fn current_item(self) -> Option<CurrentItem> {
        Some(*self)
    }
}
impl AriaCurrent for &Option<CurrentItem> {
    fn current_item(self) -> Option<CurrentItem> {
        *self
    }
}
impl AriaCurrent for CurrentItem {
    fn current_item(self) -> Option<CurrentItem> {
        Some(self)
    }
}
impl AriaCurrent for Option<CurrentItem> {
    fn current_item(self) -> Option<CurrentItem> {
        self
    }
}
/// Converts an `aria-current={…}` value.
pub fn aria_current(value: impl AriaCurrent) -> Option<CurrentItem> {
    value.current_item()
}
