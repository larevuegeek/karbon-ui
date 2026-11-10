pub(super) use crate::ui::{
    element::{CheckState, ChoiceState, Role},
    runtime::{AccessibilityAction, AccessibleElement},
};
pub(super) use objc2::{rc::Retained, runtime::AnyObject};
pub(super) use objc2_app_kit::*;
pub(super) use objc2_foundation::{NSCopying, NSNumber, NSString};

/// Rôle AppKit d'un rôle du moteur.
pub(crate) fn role(role: Role) -> &'static NSString {
    // SAFETY : constantes d'AppKit.
    unsafe {
        match role {
            Role::Text => NSAccessibilityStaticTextRole,
            Role::Button => NSAccessibilityButtonRole,
            Role::Link => NSAccessibilityLinkRole,
            Role::Image => NSAccessibilityImageRole,
            Role::Heading => NSAccessibilityHeadingRole,
            Role::TextInput => NSAccessibilityTextFieldRole,
            Role::Checkbox | Role::Switch => NSAccessibilityCheckBoxRole,
            Role::RadioButton => NSAccessibilityRadioButtonRole,
            Role::RadioGroup => NSAccessibilityRadioGroupRole,
            // Comme WebKit : un onglet est un bouton radio (sous-rôle AXTabButton), une
            // liste d'onglets un groupe d'onglets.
            Role::Tab => NSAccessibilityRadioButtonRole,
            Role::TabList => NSAccessibilityTabGroupRole,
            Role::Slider => NSAccessibilitySliderRole,
            Role::Select => NSAccessibilityPopUpButtonRole,
            Role::Menu => NSAccessibilityMenuRole,
            Role::MenuItem => NSAccessibilityMenuItemRole,
            Role::List => NSAccessibilityListRole,
            Role::ScrollArea => NSAccessibilityScrollAreaRole,
            Role::Separator => NSAccessibilitySplitterRole,
            Role::Dialog
            | Role::Popover
            | Role::ListItem
            | Role::Group
            | Role::Main
            | Role::Navigation
            | Role::Banner
            | Role::ContentInfo
            | Role::Complementary
            | Role::TabPanel
            | Role::Custom => NSAccessibilityGroupRole,
        }
    }
}
/// Rôle AppKit d'un élément : celui de son rôle, sauf un champ multiligne
/// (`<textarea>`), annoncé comme zone de texte, et un bouton à `aria-pressed`, case à
/// cocher de sous-rôle AXToggle comme dans WebKit.
pub(crate) fn element_role(e: &AccessibleElement) -> &'static NSString {
    if e.role == Role::TextInput && e.text.as_ref().is_some_and(|t| t.multiline) {
        // SAFETY : constante d'AppKit.
        return unsafe { NSAccessibilityTextAreaRole };
    }
    if toggle_button(e) {
        // SAFETY : constante d'AppKit.
        return unsafe { NSAccessibilityCheckBoxRole };
    }
    role(e.role)
}
/// Bouton bascule (`aria-pressed`).
pub(super) fn toggle_button(e: &AccessibleElement) -> bool {
    e.role == Role::Button && matches!(e.choice, Some(ChoiceState::Toggle(_)))
}
/// Sous-rôle AppKit d'un élément : celui de son rôle, ou AXToggle pour un bouton bascule.
pub(crate) fn element_subrole(e: &AccessibleElement) -> Option<Retained<NSString>> {
    if toggle_button(e) {
        // SAFETY : constante d'AppKit.
        return Some(unsafe { NSAccessibilityToggleSubrole }.copy());
    }
    subrole(e.role)
}
/// Sous-rôle AppKit, quand il précise le rôle : interrupteur, dialogue, et repères de
/// page (`AXLandmark…`, ceux de WebKit, que VoiceOver propose dans son rotor).
pub(crate) fn subrole(role: Role) -> Option<Retained<NSString>> {
    let landmark = |name: &str| Some(NSString::from_str(name));
    // SAFETY : constantes d'AppKit.
    unsafe {
        match role {
            Role::Switch => Some(NSAccessibilitySwitchSubrole.copy()),
            Role::Dialog => Some(NSAccessibilityDialogSubrole.copy()),
            Role::Tab => Some(NSAccessibilityTabButtonSubrole.copy()),
            Role::Main => landmark("AXLandmarkMain"),
            Role::Navigation => landmark("AXLandmarkNavigation"),
            Role::Banner => landmark("AXLandmarkBanner"),
            Role::ContentInfo => landmark("AXLandmarkContentInfo"),
            Role::Complementary => landmark("AXLandmarkComplementary"),
            _ => None,
        }
    }
}
pub(super) fn object<T: objc2::Message>(value: Retained<T>) -> Retained<AnyObject> {
    // SAFETY : tout objet Objective-C est un `AnyObject`.
    unsafe { Retained::cast_unchecked(value) }
}
/// Valeur lue par VoiceOver : texte d'un champ ou d'un texte, état d'une case, valeur
/// d'un curseur, option choisie d'une liste.
pub(crate) fn value(e: &AccessibleElement) -> Option<Retained<AnyObject>> {
    if let Some(range) = &e.range {
        return Some(object(NSNumber::new_f64(range.value)));
    }
    match e.choice {
        Some(ChoiceState::Toggle(state)) => {
            let n = match state {
                CheckState::Off => 0,
                CheckState::On => 1,
                CheckState::Mixed => 2,
            };
            return Some(object(NSNumber::new_i32(n)));
        }
        Some(ChoiceState::Radio(on)) => return Some(object(NSNumber::new_bool(on))),
        None => {}
    }
    if let Some(text) = &e.text {
        return Some(object(NSString::from_str(&text.value)));
    }
    if let Some(select) = &e.select
        && let Some(option) = select.options.iter().find(|o| o.selected)
    {
        return Some(object(NSString::from_str(&option.label)));
    }
    match (&e.value, e.role) {
        (Some(value), _) => Some(object(NSString::from_str(value))),
        (None, Role::Text | Role::Heading) => Some(object(NSString::from_str(&e.name))),
        _ => None,
    }
}
/// Action d'un appui (`AXPress`) selon le rôle.
pub(crate) fn press(e: &AccessibleElement) -> Option<AccessibilityAction> {
    if !e.enabled {
        return None;
    }
    match e.role {
        Role::Checkbox | Role::Switch => Some(AccessibilityAction::Toggle(e.id)),
        Role::RadioButton | Role::Tab => Some(AccessibilityAction::SelectChoice(e.id)),
        _ if matches!(e.choice, Some(ChoiceState::Toggle(_))) => {
            Some(AccessibilityAction::Toggle(e.id))
        }
        _ if matches!(e.choice, Some(ChoiceState::Radio(_))) => {
            Some(AccessibilityAction::SelectChoice(e.id))
        }
        Role::Select => Some(AccessibilityAction::ExpandSelect {
            id: e.id,
            expanded: !e.select.as_ref().is_some_and(|s| s.expanded),
        }),
        Role::TextInput => Some(AccessibilityAction::Focus(e.id)),
        _ if e.invokable => Some(AccessibilityAction::Invoke(e.id)),
        _ if e.focusable => Some(AccessibilityAction::Focus(e.id)),
        _ => None,
    }
}
/// Pas d'un curseur (`AXIncrement`, `AXDecrement`).
pub(crate) fn step(e: &AccessibleElement, up: bool) -> Option<AccessibilityAction> {
    let range = e.range.as_ref().filter(|r| !r.read_only)?;
    let delta = if up {
        range.small_change
    } else {
        -range.small_change
    };
    Some(AccessibilityAction::SetNumber {
        id: e.id,
        value: (range.value + delta).clamp(range.min, range.max),
    })
}
/// Notification AppKit envoyée pour l'élément `element`.
pub(crate) fn post(element: &AnyObject, notification: &NSString) {
    // SAFETY : objet accessible valide ; nom de notification d'AppKit.
    unsafe { NSAccessibilityPostNotification(element, notification) };
}
