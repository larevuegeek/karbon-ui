//! Tests du TextPattern et des plages de texte.
use super::super::tests::{code, same};
use super::range::{TextRange, TextRange_Impl};
use super::*;
use windows::{
    Win32::UI::Accessibility::{
        ITextProvider, TextPatternRangeEndpoint as Endpoint, TextUnit, UIA_PATTERN_ID,
        UIA_TEXTATTRIBUTE_ID,
    },
    core::{BOOL, BSTR, Interface},
};

fn sample(value: &str) -> TextData {
    let mut carets = vec![(0, 100.)];
    let mut clusters = Vec::new();
    for (index, (start, c)) in value.char_indices().enumerate() {
        let end = start + c.len_utf8();
        clusters.push((
            start..end,
            UiaRect {
                left: 100. + index as f64 * 10.,
                top: 200.,
                width: 10.,
                height: 20.,
            },
        ));
        carets.push((end, 110. + index as f64 * 10.));
    }
    TextData {
        value: value.into(),
        selection: 0..0,
        characters: carets.iter().map(|c| c.0).collect(),
        carets,
        clusters,
        lines: vec![(
            0..value.len(),
            UiaRect {
                left: 100.,
                top: 200.,
                width: value.chars().count() as f64 * 10.,
                height: 20.,
            },
        )],
        clip: UiaRect {
            left: 100.,
            top: 200.,
            width: 300.,
            height: 20.,
        },
    }
}
fn setup(value: &str) -> (Arc<Bridge>, IRawElementProviderSimple) {
    let bridge = super::super::tests::bridge();
    replace(&bridge, sample(value));
    let simple = bridge.interface(NodeId::Element(92)).unwrap();
    (bridge, simple)
}
fn replace(bridge: &Bridge, text: TextData) {
    let previous = lock(&bridge.snapshot).clone();
    let mut nodes = previous.nodes.clone();
    let node = Arc::make_mut(&mut nodes[1]);
    node.value = Some(text.value.to_string());
    node.text = Some(Arc::new(text));
    *lock(&bridge.snapshot) = Arc::new(Snapshot { nodes });
}
fn document_range(simple: &IRawElementProviderSimple) -> ITextRangeProvider {
    let provider: ITextProvider = simple.cast().unwrap();
    // SAFETY: interface vivante ; la plage rendue est possédée.
    unsafe { provider.DocumentRange() }.unwrap()
}
fn text(r: &ITextRangeProvider, max: i32) -> String {
    // SAFETY: interface vivante ; le BSTR rendu est libéré au drop.
    unsafe { r.GetText(max) }.unwrap().to_string()
}
fn clone(r: &ITextRangeProvider) -> ITextRangeProvider {
    // SAFETY: interface vivante ; la copie rendue est possédée.
    unsafe { r.Clone() }.unwrap()
}
/// Première plage d'un tableau VT_UNKNOWN possédé, détruit ensuite.
fn array_first(array: *mut SAFEARRAY) -> ITextRangeProvider {
    // SAFETY: tableau d'interfaces possédé rendu par le fournisseur.
    let items = unsafe { take::<Option<IUnknown>>(array) };
    items[0].as_ref().unwrap().cast().unwrap()
}
/// Objet Rust derrière une plage COM (transtypage dynamique de windows-rs).
fn inner(r: &ITextRangeProvider) -> &TextRange_Impl {
    r.cast_object_ref::<TextRange>().unwrap()
}

#[test]
fn text_pattern_abi_document_selection_and_com_identity() {
    let (bridge, simple) = setup("Café 😃");
    let text_provider: ITextProvider = simple.cast().unwrap();
    assert!(same(&text_provider, &simple));
    // SAFETY: interface vivante ; le pattern rendu est possédé.
    let pattern = unsafe { simple.GetPatternProvider(UIA_PATTERN_ID(10014)) }.unwrap();
    assert_eq!(pattern.as_raw(), text_provider.as_raw());
    let r = document_range(&simple);
    assert_eq!(text(&r, -1), "Café 😃");
    assert_eq!(text(&r, 6), "Café "); // Ne jamais couper la paire UTF-16 de l'emoji.
    assert_eq!(text(&r, 7), "Café 😃");
    let again: ITextRangeProvider = r.cast().unwrap();
    assert_eq!(again.as_raw(), r.as_raw());
    assert!(same(&again, &r));
    let mut data = sample("Café 😃");
    data.selection = 3..5;
    replace(&bridge, data);
    // SAFETY: interfaces vivantes ; tableaux et plages rendus sont possédés.
    unsafe {
        assert_eq!(
            text(&array_first(text_provider.GetSelection().unwrap()), -1),
            "é"
        );
        assert_eq!(code(r.GetText(-2)), INVALID_ARGUMENT);
        let nan = UiaPoint { x: f64::NAN, y: 0. };
        assert_eq!(code(text_provider.RangeFromPoint(nan)), INVALID_ARGUMENT);
        assert_eq!(
            code(text_provider.RangeFromChild(&simple)),
            INVALID_ARGUMENT
        );
        assert_eq!(
            text_provider.SupportedTextSelection().unwrap(),
            SupportedTextSelection_Single
        );
    }
    bridge.close();
    assert!(text_provider.cast::<ITextProvider>().is_ok());
    // SAFETY: interface vivante ; le nœud a disparu du pont fermé.
    assert_eq!(code(unsafe { r.GetText(-1) }), UNAVAILABLE);
}

#[test]
fn ranges_move_by_clusters_words_and_document_without_overflow() {
    let (_, simple) = setup("Un 😃 deux");
    let r = document_range(&simple);
    // SAFETY: interfaces vivantes ; seules des valeurs scalaires sont échangées.
    unsafe {
        r.ExpandToEnclosingUnit(TextUnit(2)).unwrap();
        assert_eq!(text(&r, -1), "Un ");
        assert_eq!(r.Move(TextUnit(2), 1).unwrap(), 1);
        assert_eq!(text(&r, -1), "😃 ");
        assert_eq!(r.Move(TextUnit(2), i32::MAX).unwrap(), 1);
        assert_eq!(text(&r, -1), "deux");
        r.MoveEndpointByUnit(Endpoint(1), TextUnit(0), i32::MIN)
            .unwrap();
        assert_eq!(text(&r, -1), "");
        assert_eq!(inner(&r).current().unwrap().2, 0..0);
        assert_eq!(
            r.MoveEndpointByUnit(Endpoint(1), TextUnit(0), 4).unwrap(),
            4
        );
        assert_eq!(text(&r, -1), "Un 😃");
        let old = clone(&r);
        r.Move(TextUnit(3), 0).unwrap();
        assert!(r.Compare(&old).unwrap().as_bool());
        assert_eq!(
            code(r.MoveEndpointByUnit(Endpoint(9), TextUnit(0), 1)),
            INVALID_ARGUMENT
        );
        assert_eq!(code(r.ExpandToEnclosingUnit(TextUnit(7))), INVALID_ARGUMENT);
        // Un objet étranger passé comme plage (ici le fournisseur simple) est refusé.
        let mut equal = BOOL(0);
        let compare = (Interface::vtable(&r).Compare)(r.as_raw(), simple.as_raw(), &mut equal);
        assert_eq!(compare, INVALID_ARGUMENT);
    }

    let (_, simple) = setup("e\u{301}x");
    let bridge = inner(&document_range(&simple)).bridge.clone();
    let mut t = sample("e\u{301}x");
    t.carets.retain(|(byte, _)| *byte != 1);
    t.characters.retain(|&byte| byte != 1);
    replace(&bridge, t);
    let r = document_range(&simple);
    // SAFETY: interface vivante ; seule une unité scalaire est transmise.
    unsafe { r.ExpandToEnclosingUnit(TextUnit(0)) }.unwrap();
    assert_eq!(text(&r, -1), "e\u{301}");
}

#[test]
fn text_search_endpoints_and_read_only_attributes() {
    let (bridge, simple) = setup("Été 😃 été");
    let r = document_range(&simple);
    // SAFETY: interfaces vivantes ; BSTR et plages rendues sont possédés.
    let found = unsafe { r.FindText(&BSTR::from("ÉTÉ"), true, true) }.unwrap();
    assert_eq!(text(&found, -1), "été");
    // SAFETY: interfaces vivantes ; seules des bornes scalaires sont échangées.
    unsafe {
        assert!(
            r.CompareEndpoints(Endpoint(0), &found, Endpoint(0))
                .unwrap()
                < 0
        );
        r.MoveEndpointByRange(Endpoint(0), &found, Endpoint(0))
            .unwrap();
    }
    assert_eq!(text(&r, -1), "été");
    let previous = lock(&bridge.snapshot).clone();
    let mut nodes = previous.nodes.clone();
    Arc::make_mut(&mut nodes[1]).read_only = true;
    *lock(&bridge.snapshot) = Arc::new(Snapshot { nodes });
    // SAFETY: interfaces vivantes ; les VARIANT rendus sont adoptés puis libérés.
    unsafe {
        let value = Variant::adopt(r.GetAttributeValue(UIA_TEXTATTRIBUTE_ID(40015)).unwrap());
        assert_eq!(value.as_bool(), Some(true));
        let value = Variant::adopt(r.GetAttributeValue(UIA_TEXTATTRIBUTE_ID(40005)).unwrap());
        assert_eq!(value.kind(), 13);
        let integer = Variant::integer(1);
        assert_eq!(
            code(r.FindAttribute(UIA_TEXTATTRIBUTE_ID(40015), integer.get(), false)),
            INVALID_ARGUMENT
        );
        assert_eq!(
            code(r.FindAttribute(UIA_TEXTATTRIBUTE_ID(40005), integer.get(), false)),
            NOT_IMPLEMENTED
        );
        let found = r
            .FindAttribute(
                UIA_TEXTATTRIBUTE_ID(40015),
                Variant::boolean(true).get(),
                false,
            )
            .unwrap();
        assert_eq!(text(&found, -1), "été");
        assert_eq!(code(r.AddToSelection()), INVALID_OPERATION);
    }
}

#[test]
fn physical_rectangles_point_hit_and_visible_ranges_use_clipping() {
    let (bridge, simple) = setup("abcdef");
    let mut t = sample("abcdef");
    t.clip.left = 115.;
    t.clip.width = 20.;
    replace(&bridge, t);
    let provider: ITextProvider = simple.cast().unwrap();
    // SAFETY: interfaces vivantes ; tableaux et plages rendus sont possédés.
    unsafe {
        assert_eq!(
            text(&array_first(provider.GetVisibleRanges().unwrap()), -1),
            "bcd"
        );
        let hit = provider
            .RangeFromPoint(UiaPoint { x: 129., y: 205. })
            .unwrap();
        assert_eq!(inner(&hit).current().unwrap().2, 3..3);
        let r = document_range(&simple);
        assert_eq!(
            take::<f64>(r.GetBoundingRectangles().unwrap()),
            [115., 200., 20., 20.]
        );
        let enclosure = r.GetEnclosingElement().unwrap();
        assert_eq!(enclosure.as_raw(), simple.as_raw());
    }
}

#[test]
fn retained_ranges_rebase_after_edits_and_expire_after_removal() {
    let (bridge, simple) = setup("abc 😃 fin");
    let r = document_range(&simple);
    inner(&r).update(&"abc 😃 fin".into(), 4..8);
    let original = clone(&r);
    replace(&bridge, sample("Début abc 😃 fin"));
    assert_eq!(text(&r, -1), "😃");
    // SAFETY: interfaces vivantes ; seule une comparaison scalaire est rendue.
    assert!(unsafe { r.Compare(&original) }.unwrap().as_bool());
    replace(&bridge, sample(""));
    assert_eq!(text(&r, -1), "");
    *lock(&bridge.snapshot) = Arc::new(Snapshot { nodes: Vec::new() });
    // SAFETY: interface vivante ; le nœud a disparu du snapshot.
    assert_eq!(code(unsafe { r.GetText(-1) }), UNAVAILABLE);
    assert!(r.cast::<ITextRangeProvider>().is_ok());
}
