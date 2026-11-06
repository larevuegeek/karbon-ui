use super::*;
/// Labelled radio button. It must be placed inside a [`radio_group`], and at
/// most one button per group may be selected.
pub struct RadioButton {
    /// Label drawn next to the button, also used as the accessible name.
    pub label: Arc<str>,
    /// Whether this button is the selected one of its group.
    pub selected: bool,
    /// Message emitted when the button is chosen while not selected.
    pub command: Action,
    /// Whether the button accepts input.
    pub enabled: bool,
    /// Colour overrides.
    pub style: ChoiceStyle,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
}
impl RadioButton {
    /// Radio button emitting a clone of `message` when chosen. Choosing the
    /// selected button again emits nothing.
    pub fn new<M: Clone + 'static>(label: impl Into<Arc<str>>, selected: bool, message: M) -> Self {
        Self {
            label: label.into(),
            selected,
            command: Action::new(message),
            enabled: true,
            style: ChoiceStyle::default(),
            css: None,
        }
    }
    fn activation(&self) -> Action {
        self.command.clone()
    }
    /// Disable the button: it is greyed out and ignores activation.
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
choice_widget!(RadioButton, Role::RadioButton, false, |w: &RadioButton| {
    ChoiceState::Radio(w.selected)
});
struct Group {
    label: Arc<str>,
}
impl Widget for Group {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, _: &PaintContext, _: &mut Scene) {}
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::RadioGroup,
            name: &self.label,
            enabled: true,
        }
    }
}
/// Widget de groupe radio posé sur un conteneur existant (templates).
pub(crate) fn group_widget(label: Arc<str>) -> impl Widget {
    Group { label }
}
/// Column of radio choices. The label describes the group for accessibility;
/// add a separate [`Text`](crate::ui::components::Text) if a visible title is wanted.
///
/// Every [`RadioButton`] must belong to a group, and a group may contain at most
/// one selected button; otherwise view validation fails.
pub fn radio_group(id: u64, label: impl Into<Arc<str>>, children: Vec<Element>) -> Element {
    let mut element = Element::column(id, children).gap(4.);
    element.widget = Some(Box::new(Group {
        label: label.into(),
    }));
    element
}
pub(crate) struct RadioMember {
    pub id: ElementId,
    pub selected: bool,
    pub enabled: bool,
}
pub(crate) struct RadioMembers {
    pub members: Vec<RadioMember>,
}
pub(crate) fn radio_groups(root: &Element) -> crate::Result<Vec<RadioMembers>> {
    fn visit(
        node: &Element,
        current: Option<usize>,
        groups: &mut Vec<RadioMembers>,
    ) -> crate::Result<()> {
        let current = if node
            .widget
            .as_ref()
            .is_some_and(|w| w.semantics().role == Role::RadioGroup)
        {
            groups.push(RadioMembers {
                members: Vec::new(),
            });
            Some(groups.len() - 1)
        } else {
            current
        };
        if let Some(widget) = &node.widget
            && let Some(ChoiceState::Radio(selected)) = widget.choice()
        {
            let group = current.ok_or_else(|| {
                crate::Error::invalid_input("a radio button must belong to a radio_group")
            })?;
            groups[group].members.push(RadioMember {
                id: node.id,
                selected,
                enabled: widget.semantics().enabled && widget.focusable(),
            });
        }
        for child in &node.children {
            visit(child, current, groups)?;
        }
        Ok(())
    }
    let mut groups = Vec::new();
    visit(root, None, &mut groups)?;
    if groups
        .iter()
        .any(|g| g.members.iter().filter(|m| m.selected).count() > 1)
    {
        return Err(crate::Error::invalid_input(
            "several buttons are selected in one radio group",
        ));
    }
    Ok(groups)
}
