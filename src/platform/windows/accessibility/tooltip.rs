//! Info-bulle passive : identité distincte, sans focus ni pattern d'action.
use super::*;
pub(super) fn append(
    nodes: &mut Vec<Arc<Node>>,
    info: crate::ui::tooltip::TooltipInfo,
    origin: Point,
    scale: f32,
    visible: bool,
) {
    let owner = NodeId::Element(info.owner.0);
    let parent = if nodes.iter().any(|n| n.key == owner) {
        owner
    } else {
        NodeId::Root
    };
    let bounds = physical(info.bounds, origin, scale);
    nodes.push(Arc::new(Node {
        key: NodeId::Tooltip(info.owner.0),
        parent: Some(parent),
        name: info.text.to_string(),
        help: String::new(),
        control_type: 50022,
        is_dialog: false,
        enabled: true,
        focusable: false,
        focused: false,
        offscreen: !visible,
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
    }));
}

#[cfg(test)]
mod tests {
    use super::super::tests::{code, property, same};
    use super::*;
    use crate::ui::{element::ElementId, scene::RectF, tooltip::TooltipInfo};
    use windows::Win32::UI::Accessibility::{
        IInvokeProvider, IRawElementProviderFragment, IRawElementProviderFragmentRoot,
    };
    fn info(owner: u64) -> TooltipInfo {
        TooltipInfo {
            owner: ElementId(owner),
            text: "Texte d’aide 😃".into(),
            bounds: RectF::new(10., 20., 100., 30.),
        }
    }
    #[test]
    fn tooltip_has_scoped_identity_passive_properties_and_preserves_native_focus() {
        let bridge = super::super::tests::bridge();
        let mut nodes = lock(&bridge.snapshot).nodes.clone();
        Arc::make_mut(&mut nodes[1]).focused = true;
        append(&mut nodes, info(92), Point { x: -200, y: 100 }, 1.5, true);
        assert_eq!(nodes.last().unwrap().parent, Some(NodeId::Element(92)));
        assert_eq!(nodes.last().unwrap().bounds.left, -185.);
        *lock(&bridge.snapshot) = Arc::new(Snapshot::new(nodes));
        let object: IRawElementProviderSimple = bridge.interface(NodeId::Tooltip(92)).unwrap();
        assert!(object.cast::<IInvokeProvider>().is_err());
        for id in [30009, 30017, 30008, 30031] {
            let v = property(&object, id).unwrap();
            assert_eq!(v.kind(), 11);
            assert_eq!(v.as_bool(), Some(false));
        }
        let fragment: IRawElementProviderFragment = object.cast().unwrap();
        let root: IRawElementProviderFragmentRoot = bridge.interface(NodeId::Root).unwrap();
        let owner: IRawElementProviderSimple = bridge.interface(NodeId::Element(92)).unwrap();
        // SAFETY: interfaces vivantes ; tableau et fragment rendus sont possédés.
        unsafe {
            assert_eq!(take::<i32>(fragment.GetRuntimeId().unwrap()), [3, 5, 92, 0]);
            let focused = root.GetFocus().unwrap();
            assert!(same(&focused, &owner));
        }
        *lock(&bridge.snapshot) = Arc::new(Snapshot::new(vec![]));
        assert_eq!(code(property(&object, 30005)), UNAVAILABLE);
    }
}
