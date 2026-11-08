//! Tests du pattern RangeValue.
use super::super::tests::{code, same};
use super::*;
use windows::Win32::UI::Accessibility::{IInvokeProvider, IRangeValueProvider, IValueProvider};

fn fixture() -> (Arc<Bridge>, IRawElementProviderSimple) {
    let bridge = super::super::tests::bridge();
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    let node = Arc::make_mut(&mut nodes[1]);
    node.control_type = 50015;
    node.value = None;
    node.range = Some(RangeInfo {
        value: 40.,
        min: 0.,
        max: 100.,
        small_change: 2.,
        large_change: 20.,
        read_only: false,
        axis: Axis::Horizontal,
    });
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(nodes));
    let provider = bridge.interface(NodeId::Element(92)).unwrap();
    (bridge, provider)
}
#[test]
fn range_pattern_vtable_returns_numeric_values_and_preserves_com_identity() {
    let (bridge, simple) = fixture();
    let range: IRangeValueProvider = simple.cast().unwrap();
    assert!(same(&range, &simple));
    assert!(simple.cast::<IValueProvider>().is_err());
    assert!(simple.cast::<IInvokeProvider>().is_err());
    // SAFETY: interface vivante ; seules des valeurs scalaires sont rendues.
    unsafe {
        assert_eq!(range.Value().unwrap(), 40.);
        assert_eq!(range.Maximum().unwrap(), 100.);
        assert_eq!(range.Minimum().unwrap(), 0.);
        assert_eq!(range.LargeChange().unwrap(), 20.);
        assert_eq!(range.SmallChange().unwrap(), 2.);
        assert!(!range.IsReadOnly().unwrap().as_bool());
    }
    let node = bridge.node(NodeId::Element(92)).unwrap();
    let range_info = node.range.unwrap();
    let value = property(range_info, 30047);
    assert_eq!(value, Scalar::Number(40.));
    assert_eq!(value.variant().kind(), 5);
    assert_eq!(value.variant().as_f64(), Some(40.));
    let orientation = property(range_info, 30023);
    assert_eq!(orientation.variant().kind(), 3);
    assert_eq!(orientation.variant().as_i32(), Some(1));
    bridge.close();
    assert!(range.cast::<IRangeValueProvider>().is_ok());
    // SAFETY: interface vivante ; le nœud a disparu du pont fermé.
    assert_eq!(code(unsafe { range.Value() }), UNAVAILABLE);
}
#[test]
fn range_setter_rejects_invalid_disabled_and_read_only_values_before_queueing() {
    let (bridge, simple) = fixture();
    let range: IRangeValueProvider = simple.cast().unwrap();
    for v in [f64::NAN, f64::INFINITY, -1., 101.] {
        // SAFETY: interface vivante ; la valeur est rejetée avant la file de commandes.
        assert_eq!(code(unsafe { range.SetValue(v) }), INVALID_ARGUMENT);
    }
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    Arc::make_mut(&mut nodes[1])
        .range
        .as_mut()
        .unwrap()
        .read_only = true;
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(nodes));
    // SAFETY: interface vivante ; seules des valeurs scalaires sont échangées.
    unsafe {
        assert!(range.IsReadOnly().unwrap().as_bool());
        assert_eq!(code(range.SetValue(50.)), INVALID_OPERATION);
    }
    let old = lock(&bridge.snapshot).clone();
    let mut nodes = old.nodes.clone();
    Arc::make_mut(&mut nodes[1]).enabled = false;
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(nodes));
    // SAFETY: interface vivante ; la commande est refusée avant la file.
    assert_eq!(code(unsafe { range.SetValue(50.) }), DISABLED);
    assert!(lock(&bridge.requests).is_empty());
}
