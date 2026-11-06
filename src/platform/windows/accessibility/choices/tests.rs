//! Tests des patterns Toggle, Selection et SelectionItem.
use super::super::tests::{code, same};
use super::*;
use windows::Win32::{
    System::Variant::VariantClear,
    UI::Accessibility::{
        IInvokeProvider, ISelectionItemProvider, ISelectionProvider, IToggleProvider,
    },
};

fn fixture() -> Arc<Bridge> {
    let bridge = super::super::tests::bridge();
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    let check = Arc::make_mut(&mut nodes[1]);
    check.control_type = 50002;
    check.value = None;
    check.choice = Some(ChoiceState::Toggle(CheckState::Mixed));
    let mut group = check.clone();
    group.key = NodeId::Element(100);
    group.control_type = 50026;
    group.choice = None;
    group.focusable = false;
    let mut first = group.clone();
    first.key = NodeId::Element(101);
    first.control_type = 50013;
    first.parent = Some(group.key);
    first.selection_container = Some(group.key);
    first.choice = Some(ChoiceState::Radio(true));
    first.focusable = true;
    let mut second = first.clone();
    second.key = NodeId::Element(102);
    second.choice = Some(ChoiceState::Radio(false));
    nodes.extend([Arc::new(group), Arc::new(first), Arc::new(second)]);
    *lock(&bridge.snapshot) = Arc::new(Snapshot { nodes });
    bridge
}
fn object<I: Interface>(bridge: &Arc<Bridge>, id: u64) -> I {
    bridge.interface(NodeId::Element(id)).unwrap()
}
/// Fournisseurs d'un tableau VT_UNKNOWN possédé, détruit ensuite.
fn providers(array: *mut SAFEARRAY) -> Vec<IUnknown> {
    // SAFETY: tableau d'interfaces possédé rendu par le fournisseur.
    unsafe { take::<Option<IUnknown>>(array) }
        .into_iter()
        .map(Option::unwrap)
        .collect()
}
#[test]
fn toggle_exposes_mixed_state_and_canonical_identity_and_rejects_disabled_actions() {
    let bridge = fixture();
    let simple: IRawElementProviderSimple = object(&bridge, 92);
    let toggle_provider: IToggleProvider = simple.cast().unwrap();
    assert!(same(&toggle_provider, &simple));
    assert!(simple.cast::<IInvokeProvider>().is_err());
    // SAFETY: interface vivante ; seul un état scalaire est rendu.
    assert_eq!(unsafe { toggle_provider.ToggleState() }.unwrap().0, 2);
    let available = property(&bridge, &bridge.node(NodeId::Element(92)).unwrap(), 30041).unwrap();
    assert_eq!(available.as_bool(), Some(true));
    drop(available);
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    Arc::make_mut(&mut nodes[1]).enabled = false;
    *lock(&bridge.snapshot) = Arc::new(Snapshot { nodes });
    // SAFETY: interface vivante ; aucun pointeur n'est échangé.
    unsafe {
        assert_eq!(code(toggle_provider.Toggle()), DISABLED);
        // La lecture de l'état reste permise sur un contrôle désactivé.
        assert_eq!(toggle_provider.ToggleState().unwrap().0, 2);
    }
    bridge.close();
    assert!(toggle_provider.cast::<IToggleProvider>().is_ok());
    // SAFETY: interface vivante ; le nœud a disparu du pont fermé.
    assert_eq!(code(unsafe { toggle_provider.ToggleState() }), UNAVAILABLE);
}

#[test]
fn radio_patterns_report_selection_container_and_prevent_multiple_or_empty_selection() {
    let bridge = fixture();
    let group: ISelectionProvider = object(&bridge, 100);
    let first: ISelectionItemProvider = object(&bridge, 101);
    let second: ISelectionItemProvider = object(&bridge, 102);
    assert!(first.cast::<IToggleProvider>().is_err());
    // SAFETY: interfaces vivantes ; tableaux et interfaces rendus sont possédés.
    unsafe {
        let selected = providers(group.GetSelection().unwrap());
        assert_eq!(
            selected[0].as_raw(),
            object::<IRawElementProviderSimple>(&bridge, 101).as_raw()
        );
        assert_eq!(
            first.SelectionContainer().unwrap().as_raw(),
            object::<IRawElementProviderSimple>(&bridge, 100).as_raw()
        );
        assert!(group.IsSelectionRequired().unwrap().as_bool());
        assert!(!group.CanSelectMultiple().unwrap().as_bool());
        assert_eq!(code(second.AddToSelection()), INVALID_OPERATION);
        assert_eq!(code(first.RemoveFromSelection()), INVALID_OPERATION);
        assert_eq!(code(second.RemoveFromSelection()), OK);
    }
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    Arc::make_mut(&mut nodes[3]).choice = Some(ChoiceState::Radio(false));
    *lock(&bridge.snapshot) = Arc::new(Snapshot { nodes });
    // SAFETY: interface vivante ; le tableau rendu est détruit par `providers`.
    unsafe {
        assert!(!group.IsSelectionRequired().unwrap().as_bool());
        assert!(providers(group.GetSelection().unwrap()).is_empty());
    }
    bridge.close();
    assert!(group.cast::<ISelectionProvider>().is_ok());
    // SAFETY: interface vivante ; le nœud a disparu du pont fermé.
    assert_eq!(code(unsafe { group.GetSelection() }), UNAVAILABLE);
}

#[test]
fn choice_properties_own_and_release_com_arrays_and_container_references() {
    let bridge = fixture();
    for (id, property_id, kind) in [
        (92, 30086, 3),
        (101, 30079, 11),
        (101, 30080, 13),
        (100, 30059, 0x200d),
        (100, 30060, 11),
        (100, 30061, 11),
    ] {
        let node = bridge.node(NodeId::Element(id)).unwrap();
        let v = property(&bridge, &node, property_id).unwrap();
        assert_eq!(v.kind(), kind);
        let mut raw = v.into_raw();
        // SAFETY: VARIANT possédé, libéré exactement une fois ici.
        assert!(unsafe { VariantClear(&mut raw) }.is_ok());
    }
    assert_eq!(
        bridge.dispatch(Command::Toggle(NodeId::Element(101))),
        UNSUPPORTED
    );
    assert_eq!(
        bridge.dispatch(Command::SelectChoice(NodeId::Element(92))),
        UNSUPPORTED
    );
    assert!(lock(&bridge.requests).is_empty());
}
