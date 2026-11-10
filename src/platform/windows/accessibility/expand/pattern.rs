pub(super) use super::super::*;
pub(super) use crate::ui::select::SelectInfo;
pub(super) use windows::Win32::UI::Accessibility::ExpandCollapseState;

/// IExpandCollapseProvider.
impl provider::Core {
    pub(crate) fn set_expanded(&self, expanded: bool) -> Result<()> {
        check(self.bridge.dispatch(Command::Expand(self.key, expanded)))
    }
    pub(crate) fn expand_state(&self) -> Result<ExpandCollapseState> {
        let expanded = self.node()?.expanded.ok_or_else(|| fail(UNSUPPORTED))?;
        Ok(ExpandCollapseState(i32::from(expanded)))
    }
}
pub(crate) fn raise_events(
    provider: &IRawElementProviderSimple,
    before: Option<bool>,
    after: Option<bool>,
) {
    if before.is_some() != after.is_some() {
        raise_property(
            provider,
            30028,
            Ok(Variant::boolean(before.is_some())),
            Ok(Variant::boolean(after.is_some())),
        );
    }
    if let (Some(a), Some(b)) = (before, after)
        && a != b
    {
        raise_property(
            provider,
            30070,
            Ok(Variant::integer(i32::from(a))),
            Ok(Variant::integer(i32::from(b))),
        );
    }
}
pub(crate) fn append_nodes(
    nodes: &mut Vec<Arc<Node>>,
    owner: ElementId,
    info: SelectInfo,
    origin: Point,
    scale: f32,
    visible: bool,
    focused: bool,
) {
    let host = NodeId::Element(owner.0);
    let popup = NodeId::SelectPopup(owner.0);
    let bounds = info
        .popup
        .map_or(UiaRect::default(), |r| physical(r, origin, scale));
    let prototype = Node {
        key: popup,
        parent: Some(host),
        name: "Options".into(),
        help: String::new(),
        control_type: 50008,
        is_dialog: false,
        enabled: true,
        focusable: false,
        focused: false,
        offscreen: !visible || !info.expanded,
        invokable: false,
        value: None,
        text: None,
        choice: None,
        selection_container: None,
        range: None,
        expanded: None,
        current: None,
        read_only: true,
        bounds,
        clipped: bounds,
    };
    if info.expanded {
        nodes.push(Arc::new(prototype.clone()));
    }
    for option in info.options {
        let mut node = prototype.clone();
        node.key = NodeId::SelectOption(owner.0, option.id);
        node.parent = Some(if info.expanded { popup } else { host });
        node.name = option.label.to_string();
        node.control_type = 50007;
        node.enabled = option.enabled;
        node.focusable = info.expanded && option.enabled && state::options().activate;
        node.focused = focused && option.focused;
        node.bounds = physical(option.bounds, origin, scale);
        node.clipped = physical(option.clip, origin, scale);
        node.offscreen = !visible || node.clipped.width <= 0. || node.clipped.height <= 0.;
        node.choice = Some(ChoiceState::Radio(option.selected));
        node.selection_container = Some(host);
        node.read_only = false;
        nodes.push(Arc::new(node));
    }
}
