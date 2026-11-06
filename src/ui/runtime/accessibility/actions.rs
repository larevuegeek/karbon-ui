use super::super::*;
impl UiRuntime {
    /// Apply a request from an assistive technology, as the equivalent user input would.
    pub fn accessibility_action(
        &mut self,
        action: AccessibilityAction,
    ) -> Result<(), AccessibilityError> {
        let focus_action = matches!(&action, AccessibilityAction::Focus(_));
        let id = match &action {
            AccessibilityAction::Focus(id)
            | AccessibilityAction::Invoke(id)
            | AccessibilityAction::Toggle(id)
            | AccessibilityAction::SelectChoice(id) => *id,
            AccessibilityAction::SetValue { id, .. }
            | AccessibilityAction::SelectText { id, .. }
            | AccessibilityAction::RevealText { id, .. }
            | AccessibilityAction::SetNumber { id, .. } => *id,
            AccessibilityAction::ExpandSelect { id, .. }
            | AccessibilityAction::ChooseOption { id, .. }
            | AccessibilityAction::HighlightOption { id, .. } => *id,
        };
        let node = self
            .root
            .as_ref()
            .and_then(|r| layers::active(r).find(id))
            .ok_or(AccessibilityError::Unavailable)?;
        if node.widget.as_ref().is_some_and(|w| !w.semantics().enabled) {
            return Err(AccessibilityError::Disabled);
        }
        match action {
            AccessibilityAction::ExpandSelect { expanded, .. } => {
                if node.widget.as_ref().and_then(|w| w.select()).is_none() {
                    // Déclencheur `aria-expanded` : l'activer bascule son panneau.
                    let Some(current) = node.expanded else {
                        return Err(AccessibilityError::Unsupported);
                    };
                    if node.widget.as_ref().and_then(|w| w.action()).is_none() {
                        return Err(AccessibilityError::Unsupported);
                    }
                    if current != expanded {
                        self.activate(id);
                        self.invalidation.repaint = true;
                    }
                    return Ok(());
                }
                if expanded == self.components.selects.expanded(id) {
                    return Ok(());
                }
                let response = if expanded {
                    self.event(UiEvent::Cancel);
                    self.interaction.focused = Some(id);
                    self.reveal_focus();
                    self.components.selects.expand(
                        id,
                        self.root.as_ref().unwrap(),
                        self.layer_viewport(),
                    )
                } else {
                    self.components.selects.close()
                };
                self.select_response(response);
                if expanded && !self.components.selects.expanded(id) {
                    return Err(AccessibilityError::Unavailable);
                }
            }
            AccessibilityAction::ChooseOption { option, .. }
            | AccessibilityAction::HighlightOption { option, .. } => {
                let select = node
                    .widget
                    .as_ref()
                    .and_then(|w| w.select())
                    .ok_or(AccessibilityError::Unsupported)?;
                let option_item = select
                    .options
                    .iter()
                    .find(|o| o.id == option)
                    .ok_or(AccessibilityError::InvalidValue)?;
                if !option_item.enabled {
                    return Err(AccessibilityError::Disabled);
                }
                let response = if matches!(action, AccessibilityAction::ChooseOption { .. }) {
                    self.components
                        .selects
                        .choose(id, option, self.root.as_ref().unwrap())
                } else {
                    self.components.selects.highlight(id, option)
                }
                .ok_or(AccessibilityError::Unavailable)?;
                self.select_response(response);
            }
            AccessibilityAction::SetNumber { value, .. } => {
                let slider = node
                    .widget
                    .as_ref()
                    .and_then(|w| w.slider())
                    .ok_or(AccessibilityError::Unsupported)?;
                if slider.read_only {
                    return Err(AccessibilityError::ReadOnly);
                }
                if !value.is_finite() || !(slider.min..=slider.max).contains(&value) {
                    return Err(AccessibilityError::InvalidValue);
                }
                let response =
                    self.components
                        .sliders
                        .set_value(id, value, self.root.as_ref().unwrap());
                self.slider_response(response);
            }
            AccessibilityAction::Focus(_) => {
                if !self.interaction.focus_order.contains(&id) {
                    return Err(AccessibilityError::Unsupported);
                }
                self.event(UiEvent::Cancel);
                self.interaction.focused = Some(id);
                self.interaction.focus_visible = true;
                self.reveal_virtual(id);
                self.reveal_focus();
                self.invalidation.repaint = true;
            }
            AccessibilityAction::Invoke(_) => {
                if !node
                    .widget
                    .as_ref()
                    .is_some_and(|w| w.action().is_some() && w.choice().is_none())
                {
                    return Err(AccessibilityError::Unsupported);
                }
                self.activate(id);
                self.invalidation.repaint = true;
            }
            AccessibilityAction::Toggle(_) | AccessibilityAction::SelectChoice(_) => {
                let choice = node.widget.as_ref().and_then(|w| w.choice());
                let valid = matches!(
                    (&action, choice),
                    (AccessibilityAction::Toggle(_), Some(ChoiceState::Toggle(_)))
                        | (
                            AccessibilityAction::SelectChoice(_),
                            Some(ChoiceState::Radio(_))
                        )
                );
                if !valid {
                    return Err(AccessibilityError::Unsupported);
                }
                self.activate(id);
                self.invalidation.repaint = true;
            }
            AccessibilityAction::SetValue { value, .. } => {
                let input = node
                    .widget
                    .as_ref()
                    .and_then(|w| w.text_input())
                    .ok_or(AccessibilityError::Unsupported)?;
                if input.read_only {
                    return Err(AccessibilityError::ReadOnly);
                }
                let handler = input.on_change.clone();
                let max = input.max_chars;
                if let Some(value) = self.components.inputs.set_value(id, &value, max) {
                    self.invalidation.semantics();
                    self.send(handler.emit(value));
                }
                self.invalidation.repaint = true;
            }
            AccessibilityAction::SelectText {
                range, expected, ..
            } => {
                if !node
                    .widget
                    .as_ref()
                    .is_some_and(|w| w.text_input().is_some())
                {
                    return Err(AccessibilityError::Unsupported);
                }
                if !self.components.inputs.select(id, range, &expected) {
                    return Err(AccessibilityError::Unavailable);
                }
                self.invalidation.repaint = true;
            }
            AccessibilityAction::RevealText { byte, .. } => {
                if !self.components.inputs.reveal(id, byte) {
                    return Err(AccessibilityError::Unsupported);
                }
                self.reveal_element(id);
                self.invalidation.repaint = true;
            }
        }
        if self.components.tooltips.dismiss() {
            self.invalidation.repaint_semantics();
        }
        if focus_action {
            self.tooltip_input_mode(true);
        }
        Ok(())
    }
}
