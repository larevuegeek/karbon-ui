pub(super) use super::super::*;
pub(super) use windows::{
    Win32::{
        System::{Com::SAFEARRAY, Variant::VT_UNKNOWN},
        UI::Accessibility::ToggleState,
    },
    core::{BOOL, IUnknown},
};

pub(super) fn state_value(state: CheckState) -> i32 {
    match state {
        CheckState::Off => 0,
        CheckState::On => 1,
        CheckState::Mixed => 2,
    }
}
pub(super) fn selected(snapshot: &Snapshot, group: NodeId) -> Vec<NodeId> {
    snapshot
        .nodes
        .iter()
        .filter(|n| {
            n.selection_container == Some(group) && n.choice == Some(ChoiceState::Radio(true))
        })
        .map(|n| n.key)
        .collect()
}
pub(super) fn selected_now(bridge: &Bridge, group: NodeId) -> Vec<NodeId> {
    selected(&lock(&bridge.snapshot), group)
}
/// Tableau VT_UNKNOWN des fournisseurs simples canoniques de `keys`.
pub(super) fn array(bridge: &Arc<Bridge>, keys: &[NodeId]) -> Result<*mut SAFEARRAY> {
    let providers = keys
        .iter()
        .map(|key| {
            bridge
                .interface::<IRawElementProviderSimple>(*key)
                .map(IUnknown::from)
        })
        .collect::<Result<Vec<_>>>()?;
    unknowns(&providers)
}
pub(crate) fn property(bridge: &Arc<Bridge>, node: &Node, id: i32) -> Result<Variant> {
    Ok(match id {
        30036 => Variant::boolean(matches!(node.choice, Some(ChoiceState::Radio(_)))),
        30037 => Variant::boolean(node.selection_container()),
        30041 => Variant::boolean(matches!(node.choice, Some(ChoiceState::Toggle(_)))),
        30086 if matches!(node.choice, Some(ChoiceState::Toggle(_))) => {
            let Some(ChoiceState::Toggle(state)) = node.choice else {
                unreachable!()
            };
            Variant::integer(state_value(state))
        }
        30079 if matches!(node.choice, Some(ChoiceState::Radio(_))) => {
            Variant::boolean(node.choice == Some(ChoiceState::Radio(true)))
        }
        30080 if node.selection_container.is_some() => Variant::unknown(
            bridge
                .interface::<IRawElementProviderSimple>(node.selection_container.unwrap())?
                .into(),
        ),
        30059 if node.selection_container() => {
            Variant::array(VT_UNKNOWN, array(bridge, &selected_now(bridge, node.key))?)
        }
        30060 if node.selection_container() => Variant::boolean(false),
        30061 if node.selection_container() => {
            Variant::boolean(!selected_now(bridge, node.key).is_empty())
        }
        _ => Variant::default(),
    })
}
/// IToggleProvider, ISelectionProvider et ISelectionItemProvider.
impl provider::Core {
    pub(crate) fn toggle(&self) -> Result<()> {
        check(self.bridge.dispatch(Command::Toggle(self.key)))
    }
    pub(crate) fn toggle_state(&self) -> Result<ToggleState> {
        let n = self.node()?;
        let Some(ChoiceState::Toggle(state)) = n.choice else {
            return Err(fail(UNSUPPORTED));
        };
        Ok(ToggleState(state_value(state)))
    }
    pub(crate) fn selection(&self) -> Result<*mut SAFEARRAY> {
        let n = self.node()?;
        if !n.selection_container() {
            return Err(fail(UNSUPPORTED));
        }
        array(&self.bridge, &selected_now(&self.bridge, self.key))
    }
    pub(crate) fn can_select_multiple(&self) -> Result<BOOL> {
        self.node()?;
        Ok(false.into())
    }
    pub(crate) fn selection_required(&self) -> Result<BOOL> {
        self.node()?;
        Ok((!selected_now(&self.bridge, self.key).is_empty()).into())
    }
    pub(crate) fn select_item(&self) -> Result<()> {
        check(self.bridge.dispatch(Command::SelectChoice(self.key)))
    }
    pub(crate) fn add_to_selection(&self) -> Result<()> {
        let n = self.node()?;
        if !n.enabled {
            return Err(fail(DISABLED));
        }
        let group = n.selection_container.ok_or_else(|| fail(UNSUPPORTED))?;
        if selected_now(&self.bridge, group)
            .iter()
            .any(|key| *key != self.key)
        {
            return Err(fail(INVALID_OPERATION));
        }
        check(self.bridge.dispatch(Command::SelectChoice(self.key)))
    }
    pub(crate) fn remove_from_selection(&self) -> Result<()> {
        let n = self.node()?;
        if !n.enabled {
            return Err(fail(DISABLED));
        }
        match n.choice {
            Some(ChoiceState::Radio(false)) => Ok(()),
            Some(ChoiceState::Radio(true)) => Err(fail(INVALID_OPERATION)),
            _ => Err(fail(UNSUPPORTED)),
        }
    }
    pub(crate) fn is_selected(&self) -> Result<BOOL> {
        let n = self.node()?;
        let Some(ChoiceState::Radio(selected)) = n.choice else {
            return Err(fail(UNSUPPORTED));
        };
        Ok(selected.into())
    }
    pub(crate) fn selection_container(&self) -> Result<IRawElementProviderSimple> {
        let n = self.node()?;
        self.bridge
            .interface(n.selection_container.ok_or_else(|| fail(UNSUPPORTED))?)
    }
}
pub(crate) fn raise_events(
    bridge: &Arc<Bridge>,
    provider: &IRawElementProviderSimple,
    before: &Node,
    node: &Node,
    old: &Snapshot,
    new: &Snapshot,
) {
    if let (Some(ChoiceState::Toggle(a)), Some(ChoiceState::Toggle(b))) =
        (before.choice, node.choice)
        && a != b
    {
        raise_property(
            provider,
            30086,
            Ok(Variant::integer(state_value(a))),
            Ok(Variant::integer(state_value(b))),
        );
    }
    if let (Some(ChoiceState::Radio(a)), Some(ChoiceState::Radio(b))) = (before.choice, node.choice)
        && a != b
    {
        raise_property(
            provider,
            30079,
            Ok(Variant::boolean(a)),
            Ok(Variant::boolean(b)),
        );
        if b {
            raise(provider, 20012);
        }
    }
    if node.selection_container() {
        let a = selected(old, node.key);
        let b = selected(new, node.key);
        if a != b {
            raise(provider, 20013);
            let variant = |keys: &[NodeId]| {
                array(bridge, keys).map(|value| Variant::array(VT_UNKNOWN, value))
            };
            raise_property(provider, 30059, variant(&a), variant(&b));
            if a.is_empty() != b.is_empty() {
                raise_property(
                    provider,
                    30061,
                    Ok(Variant::boolean(!a.is_empty())),
                    Ok(Variant::boolean(!b.is_empty())),
                );
            }
        }
    }
}
