//! Tests du pattern ExpandCollapse et des options du Select.
use super::super::tests::{code, same};
use super::*;
use crate::ui::{scene::RectF, select::SelectOptionInfo};
use windows::{
    Win32::UI::Accessibility::{
        IExpandCollapseProvider, IRawElementProviderFragment, IRawElementProviderFragmentRoot,
        ISelectionItemProvider, ISelectionProvider, IValueProvider, UIA_PATTERN_ID,
    },
    core::IUnknown,
};

fn snapshot(bridge: &Bridge, expanded: bool) {
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes[..2].to_vec();
    let host = Arc::make_mut(&mut nodes[1]);
    host.control_type = 50003;
    host.value = None;
    host.expanded = Some(expanded);
    host.focused = !expanded;
    let info = SelectInfo {
        expanded,
        popup: expanded.then_some(RectF::new(20., 100., 200., 72.)),
        options: vec![
            SelectOptionInfo {
                id: 0,
                label: "Alpha".into(),
                enabled: true,
                selected: true,
                focused: expanded,
                bounds: RectF::new(24., 104., 180., 32.),
                clip: if expanded {
                    RectF::new(24., 104., 180., 32.)
                } else {
                    RectF::new(0., 0., 0., 0.)
                },
            },
            SelectOptionInfo {
                id: u64::MAX,
                label: "Bravo".into(),
                enabled: true,
                selected: false,
                focused: false,
                bounds: RectF::new(24., 136., 180., 32.),
                clip: RectF::new(24., 136., 180., 32.),
            },
        ]
        .into_iter()
        .filter(|o| expanded || o.selected)
        .collect(),
    };
    append_nodes(
        &mut nodes,
        ElementId(92),
        info,
        Point { x: 0, y: 0 },
        1.,
        true,
        true,
    );
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(nodes));
}
fn fixture(expanded: bool) -> Arc<Bridge> {
    let bridge = super::super::tests::bridge();
    snapshot(&bridge, expanded);
    bridge
}
fn object<I: Interface>(bridge: &Arc<Bridge>, key: NodeId) -> I {
    bridge.interface(key).unwrap()
}

#[test]
fn expand_pattern_and_selection_are_canonical_and_track_current_state() {
    let bridge = fixture(false);
    let simple: IRawElementProviderSimple = object(&bridge, NodeId::Element(92));
    let expand: IExpandCollapseProvider = simple.cast().unwrap();
    assert!(same(&expand, &simple));
    assert!(simple.cast::<ISelectionProvider>().is_ok());
    assert!(simple.cast::<IValueProvider>().is_err());
    // SAFETY: interfaces vivantes ; seuls un pattern possédé et des états scalaires sont rendus.
    unsafe {
        let pattern = simple.GetPatternProvider(UIA_PATTERN_ID(10005)).unwrap();
        assert_eq!(pattern.as_raw(), expand.as_raw());
        assert_eq!(expand.ExpandCollapseState().unwrap().0, 0);
        snapshot(&bridge, true);
        assert_eq!(expand.ExpandCollapseState().unwrap().0, 1);
    }
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    Arc::make_mut(&mut nodes[1]).enabled = false;
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(nodes));
    // SAFETY: interface vivante ; aucun pointeur n'est échangé.
    unsafe {
        assert_eq!(code(expand.Expand()), DISABLED);
        assert_eq!(code(expand.Collapse()), DISABLED);
    }
    assert!(lock(&bridge.requests).is_empty());
    bridge.close();
    assert!(expand.cast::<IExpandCollapseProvider>().is_ok());
    // SAFETY: interface vivante ; le nœud a disparu du pont fermé.
    assert_eq!(code(unsafe { expand.ExpandCollapseState() }), UNAVAILABLE);
}

#[test]
fn option_identity_runtime_id_and_selection_container_do_not_use_element_ids() {
    let bridge = fixture(true);
    let key = NodeId::SelectOption(92, u64::MAX);
    let option: IRawElementProviderSimple = object(&bridge, key);
    assert_ne!(key, NodeId::Element(u64::MAX));
    assert_ne!(key, NodeId::SelectOption(93, u64::MAX));
    let item: ISelectionItemProvider = option.cast().unwrap();
    assert!(same(&item, &option));
    let fragment: IRawElementProviderFragment = option.cast().unwrap();
    // SAFETY: interfaces vivantes ; tableaux et interfaces rendus sont possédés.
    unsafe {
        assert_eq!(
            item.SelectionContainer().unwrap().as_raw(),
            object::<IRawElementProviderSimple>(&bridge, NodeId::Element(92)).as_raw()
        );
        assert_eq!(
            take::<i32>(fragment.GetRuntimeId().unwrap()),
            [3, 4, 92, 0, -1, -1]
        );
    }
    snapshot(&bridge, false);
    // SAFETY: interface vivante ; le rectangle est rendu par valeur.
    assert_eq!(code(unsafe { fragment.BoundingRectangle() }), UNAVAILABLE);
    assert!(option.cast::<ISelectionItemProvider>().is_ok());
}

#[test]
fn closed_selection_remains_readable_and_open_popup_exposes_the_active_option() {
    let bridge = fixture(false);
    let selected: IRawElementProviderSimple = object(&bridge, NodeId::SelectOption(92, 0));
    let selection: ISelectionProvider = object(&bridge, NodeId::Element(92));
    // SAFETY: interface vivante ; le tableau possédé est détruit par `take`.
    let items = unsafe { take::<Option<IUnknown>>(selection.GetSelection().unwrap()) };
    assert_eq!(items[0].as_ref().unwrap().as_raw(), selected.as_raw());
    assert!(bridge.node(NodeId::SelectOption(92, 0)).unwrap().offscreen);
    snapshot(&bridge, true);
    let root: IRawElementProviderFragmentRoot = object(&bridge, NodeId::Root);
    // SAFETY: interface vivante ; les fragments rendus sont possédés.
    unsafe {
        let hit = root.ElementProviderFromPoint(40., 110.).unwrap();
        assert!(same(&hit, &selected));
        let focus = root.GetFocus().unwrap();
        assert!(same(&focus, &selected));
    }
}
