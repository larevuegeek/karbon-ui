//! Tests du pont UI Automation (VARIANT, navigation, identité COM, appels concurrents,
//! géométrie) et fixtures partagées par les tests des patterns.
use super::notify::{MENU_CLOSED_EVENT_20007, MENU_OPENED_EVENT_20003, menu_events};
use super::*;
use windows::{
    Win32::{
        System::Variant::{VARIANT, VT_R8},
        UI::Accessibility::{IValueProvider, UIA_PROPERTY_ID},
    },
    core::{IUnknown, PCWSTR},
};

fn node(key: NodeId, parent: Option<NodeId>, value: Option<&str>) -> Arc<Node> {
    Arc::new(Node {
        key,
        parent,
        name: "Champ de test".into(),
        help: "Exemple".into(),
        control_type: if value.is_some() { 50004 } else { 50032 },
        is_dialog: false,
        enabled: true,
        focusable: true,
        focused: false,
        offscreen: false,
        invokable: false,
        value: value.map(str::to_owned),
        text: None,
        choice: None,
        selection_container: None,
        range: None,
        expanded: None,
        current: None,
        read_only: false,
        bounds: UiaRect {
            left: 100.,
            top: 200.,
            width: 300.,
            height: 42.,
        },
        clipped: UiaRect {
            left: 100.,
            top: 200.,
            width: 300.,
            height: 42.,
        },
    })
}
pub(super) fn bridge() -> Arc<Bridge> {
    let bridge = Bridge::new(null_mut());
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(vec![
        node(NodeId::Root, None, None),
        node(NodeId::Element(92), Some(NodeId::Root), Some("Café 😃")),
    ]));
    bridge
}
/// HRESULT d'un appel : S_OK en cas de succès, sinon le code d'erreur exact.
pub(super) fn code<T>(result: Result<T>) -> HRESULT {
    result.map_or_else(|e| e.code(), |_| OK)
}
/// IRawElementProviderSimple::GetPropertyValue, VARIANT adopté et libéré au drop.
pub(super) fn property(simple: &IRawElementProviderSimple, id: i32) -> Result<Variant> {
    // SAFETY: interface vivante ; le VARIANT rendu appartient à l'appelant.
    unsafe {
        simple
            .GetPropertyValue(UIA_PROPERTY_ID(id))
            .map(|v| Variant::adopt(v))
    }
}
/// Identité COM : deux interfaces désignent le même objet si leurs IUnknown coïncident.
pub(super) fn same<A: Interface, B: Interface>(a: &A, b: &B) -> bool {
    a.cast::<IUnknown>().unwrap() == b.cast::<IUnknown>().unwrap()
}
#[test]
fn variant_abi_and_tree_navigation() {
    assert_eq!(
        std::mem::size_of::<VARIANT>(),
        if cfg!(target_pointer_width = "64") {
            24
        } else {
            16
        }
    );
    let rectangle = Variant::rectangle(UiaRect {
        left: -10.,
        top: 20.,
        width: 30.,
        height: 40.,
    })
    .unwrap();
    assert_eq!(rectangle.kind(), 0x2005);
    // SAFETY: tableau VT_R8 possédé par `rectangle`, lu sans être libéré.
    let values = unsafe { elements::<f64>(rectangle.as_array(VT_R8).unwrap()) };
    assert_eq!(values, [-10., 20., 30., 40.]);
    drop(rectangle);
    // Un panic dans un callback COM devient E_FAIL au lieu de traverser la frontière FFI.
    assert_eq!(
        code(provider::boundary(|| -> Result<()> { panic!("test") })),
        FAIL
    );
    let bridge = bridge();
    let snapshot = lock(&bridge.snapshot);
    let navigate = |key, direction| snapshot.navigate(key, direction).map_err(|e| e.code());
    assert_eq!(navigate(NodeId::Root, 3), Ok(Some(NodeId::Element(92))));
    assert_eq!(navigate(NodeId::Element(92), 0), Ok(Some(NodeId::Root)));
    assert_eq!(navigate(NodeId::Element(92), 1), Ok(None));
    assert_eq!(navigate(NodeId::Element(92), 9), Err(INVALID_ARGUMENT));
}
#[test]
fn menu_events_follow_appearance_replacement_and_removal_without_firing_on_hover() {
    let empty = Snapshot::new(vec![node(NodeId::Root, None, None)]);
    let menu = |key| {
        let mut n = (*node(key, Some(NodeId::Root), None)).clone();
        n.control_type = 50009;
        Arc::new(n)
    };
    let first = Snapshot::new(vec![menu(NodeId::Element(10))]);
    let second = Snapshot::new(vec![menu(NodeId::Element(20))]);
    assert_eq!(
        menu_events(&empty, &first),
        vec![(NodeId::Element(10), MENU_OPENED_EVENT_20003)]
    );
    assert!(menu_events(&first, &first).is_empty());
    assert_eq!(
        menu_events(&first, &second),
        vec![
            (NodeId::Element(10), MENU_CLOSED_EVENT_20007),
            (NodeId::Element(20), MENU_OPENED_EVENT_20003)
        ]
    );
    assert_eq!(
        menu_events(&second, &empty),
        vec![(NodeId::Element(20), MENU_CLOSED_EVENT_20007)]
    );
}

#[test]
fn com_identity_value_pattern_and_lifetime_are_canonical() {
    let bridge = bridge();
    let key = NodeId::Element(92);
    let simple: IRawElementProviderSimple = bridge.interface(key).unwrap();
    let duplicate: IRawElementProviderSimple = bridge.interface(key).unwrap();
    assert_eq!(duplicate.as_raw(), simple.as_raw());
    drop(duplicate);
    let value: IValueProvider = simple.cast().unwrap();
    assert!(same(&value, &simple));
    // SAFETY: interface vivante ; le BSTR rendu est possédé et libéré au drop.
    let text = unsafe { value.Value() }.unwrap();
    assert_eq!(text.to_string(), "Café 😃");
    // SAFETY: un pointeur de chaîne nul est explicitement rejeté avant toute lecture.
    assert_eq!(code(unsafe { value.SetValue(PCWSTR::null()) }), POINTER);
    bridge.close();
    assert!(simple.cast::<IValueProvider>().is_ok());
    // SAFETY: interface vivante ; le nœud a disparu du pont fermé.
    assert_eq!(code(unsafe { value.Value() }), UNAVAILABLE);
}
#[test]
fn provider_getters_are_thread_safe_and_removed_nodes_become_unavailable() {
    let bridge = bridge();
    let owner: IRawElementProviderSimple = bridge.interface(NodeId::Element(92)).unwrap();
    let raw = owner.as_raw() as usize;
    let mut workers = Vec::new();
    for _ in 0..4 {
        let bridge = bridge.clone();
        workers.push(std::thread::spawn(move || {
            // Même objet canonique, obtenu depuis un autre thread via le cache faible.
            let object: IRawElementProviderSimple = bridge.interface(NodeId::Element(92)).unwrap();
            assert_eq!(object.as_raw() as usize, raw);
            for _ in 0..50 {
                assert_eq!(property(&object, 30045).unwrap().kind(), 8);
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    *lock(&bridge.snapshot) = Arc::new(Snapshot::new(vec![]));
    assert_eq!(code(property(&owner, 30005)), UNAVAILABLE);
}
#[test]
fn physical_coordinates_include_dpi_and_screen_origin() {
    let r = physical(
        crate::ui::scene::RectF::new(10., 20., 200., 40.),
        Point { x: -300, y: 100 },
        1.5,
    );
    assert_eq!(
        r,
        UiaRect {
            left: -285.,
            top: 130.,
            width: 300.,
            height: 60.
        }
    );
    assert!(contains(&r, -200., 150.));
    assert!(!contains(&r, 16., 150.));
}
