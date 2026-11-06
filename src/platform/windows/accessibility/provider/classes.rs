//! Une classe COM par profil de capacités : QueryInterface n'expose que ses patterns.
use super::*;

/// Méthodes d'un pattern, chacune enveloppée par `boundary`.
macro_rules! pattern {
    (root, $outer:ident) => {
        impl IRawElementProviderFragmentRoot_Impl for $outer {
            fn ElementProviderFromPoint(
                &self,
                x: f64,
                y: f64,
            ) -> Result<IRawElementProviderFragment> {
                boundary(|| self.core.element_from_point(x, y))
            }
            fn GetFocus(&self) -> Result<IRawElementProviderFragment> {
                boundary(|| self.core.focused_element())
            }
        }
    };
    (value, $outer:ident) => {
        impl IValueProvider_Impl for $outer {
            fn SetValue(&self, value: &PCWSTR) -> Result<()> {
                boundary(|| self.core.set_value(*value))
            }
            fn Value(&self) -> Result<BSTR> {
                boundary(|| self.core.value())
            }
            fn IsReadOnly(&self) -> Result<BOOL> {
                boundary(|| self.core.value_read_only())
            }
        }
    };
    (invoke, $outer:ident) => {
        impl IInvokeProvider_Impl for $outer {
            fn Invoke(&self) -> Result<()> {
                boundary(|| self.core.invoke())
            }
        }
    };
    (text, $outer:ident) => {
        impl ITextProvider_Impl for $outer {
            fn GetSelection(&self) -> Result<*mut SAFEARRAY> {
                boundary(|| self.core.text_selection())
            }
            fn GetVisibleRanges(&self) -> Result<*mut SAFEARRAY> {
                boundary(|| self.core.visible_ranges())
            }
            fn RangeFromChild(
                &self,
                _child: Ref<'_, IRawElementProviderSimple>,
            ) -> Result<ITextRangeProvider> {
                boundary(|| self.core.range_from_child())
            }
            fn RangeFromPoint(&self, point: &UiaPoint) -> Result<ITextRangeProvider> {
                boundary(|| self.core.range_from_point(*point))
            }
            fn DocumentRange(&self) -> Result<ITextRangeProvider> {
                boundary(|| self.core.document_range())
            }
            fn SupportedTextSelection(&self) -> Result<SupportedTextSelection> {
                boundary(|| self.core.supported_text_selection())
            }
        }
    };
    (toggle, $outer:ident) => {
        impl IToggleProvider_Impl for $outer {
            fn Toggle(&self) -> Result<()> {
                boundary(|| self.core.toggle())
            }
            fn ToggleState(&self) -> Result<ToggleState> {
                boundary(|| self.core.toggle_state())
            }
        }
    };
    (selection, $outer:ident) => {
        impl ISelectionProvider_Impl for $outer {
            fn GetSelection(&self) -> Result<*mut SAFEARRAY> {
                boundary(|| self.core.selection())
            }
            fn CanSelectMultiple(&self) -> Result<BOOL> {
                boundary(|| self.core.can_select_multiple())
            }
            fn IsSelectionRequired(&self) -> Result<BOOL> {
                boundary(|| self.core.selection_required())
            }
        }
    };
    (selection_item, $outer:ident) => {
        impl ISelectionItemProvider_Impl for $outer {
            fn Select(&self) -> Result<()> {
                boundary(|| self.core.select_item())
            }
            fn AddToSelection(&self) -> Result<()> {
                boundary(|| self.core.add_to_selection())
            }
            fn RemoveFromSelection(&self) -> Result<()> {
                boundary(|| self.core.remove_from_selection())
            }
            fn IsSelected(&self) -> Result<BOOL> {
                boundary(|| self.core.is_selected())
            }
            fn SelectionContainer(&self) -> Result<IRawElementProviderSimple> {
                boundary(|| self.core.selection_container())
            }
        }
    };
    (range, $outer:ident) => {
        impl IRangeValueProvider_Impl for $outer {
            fn SetValue(&self, value: f64) -> Result<()> {
                boundary(|| self.core.set_number(value))
            }
            fn Value(&self) -> Result<f64> {
                boundary(|| self.core.number(30047))
            }
            fn IsReadOnly(&self) -> Result<BOOL> {
                boundary(|| self.core.range_read_only())
            }
            fn Maximum(&self) -> Result<f64> {
                boundary(|| self.core.number(30050))
            }
            fn Minimum(&self) -> Result<f64> {
                boundary(|| self.core.number(30049))
            }
            fn LargeChange(&self) -> Result<f64> {
                boundary(|| self.core.number(30051))
            }
            fn SmallChange(&self) -> Result<f64> {
                boundary(|| self.core.number(30052))
            }
        }
    };
    (expand, $outer:ident) => {
        impl IExpandCollapseProvider_Impl for $outer {
            fn Expand(&self) -> Result<()> {
                boundary(|| self.core.set_expanded(true))
            }
            fn Collapse(&self) -> Result<()> {
                boundary(|| self.core.set_expanded(false))
            }
            fn ExpandCollapseState(&self) -> Result<ExpandCollapseState> {
                boundary(|| self.core.expand_state())
            }
        }
    };
}

/// Simple + Fragment, puis les patterns listés, pour la classe générée `$outer`.
/// (`#[implement]` reste hors de macro_rules : son QueryInterface dépend de l'hygiène.)
macro_rules! provider_impl {
    ($outer:ident, [$($pattern:ident),*]) => {
        impl IRawElementProviderSimple_Impl for $outer {
            fn ProviderOptions(&self) -> Result<ProviderOptions> {
                boundary(|| self.core.options())
            }
            fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
                boundary(|| self.core.pattern(pattern.0))
            }
            fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
                boundary(|| self.core.property(property.0).map(Variant::into_raw))
            }
            fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
                boundary(|| self.core.host())
            }
        }
        impl IRawElementProviderFragment_Impl for $outer {
            fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
                boundary(|| self.core.navigate(direction.0))
            }
            fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
                boundary(|| self.core.runtime_id())
            }
            fn BoundingRectangle(&self) -> Result<UiaRect> {
                boundary(|| Ok(self.core.node()?.bounds))
            }
            fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
                boundary(|| self.core.node().map(|_| null_mut()))
            }
            fn SetFocus(&self) -> Result<()> {
                boundary(|| check(self.core.bridge.dispatch(Command::Focus(self.core.key))))
            }
            fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
                boundary(|| {
                    self.core.node()?;
                    self.core.bridge.interface(NodeId::Root)
                })
            }
        }
        $(pattern!($pattern, $outer);)*
    };
}

// Une classe par profil : QueryInterface n'expose que les patterns du profil.
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IRawElementProviderFragmentRoot,
    Agile = false
)]
struct RootProvider {
    core: Core,
}
provider_impl!(RootProvider_Impl, [root]);
#[implement(IRawElementProviderSimple, IRawElementProviderFragment, Agile = false)]
struct PlainProvider {
    core: Core,
}
provider_impl!(PlainProvider_Impl, []);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IInvokeProvider,
    Agile = false
)]
struct InvokeProvider {
    core: Core,
}
provider_impl!(InvokeProvider_Impl, [invoke]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IValueProvider,
    Agile = false
)]
struct ValueProvider {
    core: Core,
}
provider_impl!(ValueProvider_Impl, [value]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IValueProvider,
    ITextProvider,
    Agile = false
)]
struct EditProvider {
    core: Core,
}
provider_impl!(EditProvider_Impl, [value, text]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IToggleProvider,
    Agile = false
)]
struct ToggleProvider {
    core: Core,
}
provider_impl!(ToggleProvider_Impl, [toggle]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    ISelectionProvider,
    Agile = false
)]
struct SelectionProvider {
    core: Core,
}
provider_impl!(SelectionProvider_Impl, [selection]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    ISelectionItemProvider,
    Agile = false
)]
struct ItemProvider {
    core: Core,
}
provider_impl!(ItemProvider_Impl, [selection_item]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IRangeValueProvider,
    Agile = false
)]
struct RangeProvider {
    core: Core,
}
provider_impl!(RangeProvider_Impl, [range]);
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IExpandCollapseProvider,
    ISelectionProvider,
    Agile = false
)]
struct SelectProvider {
    core: Core,
}
provider_impl!(SelectProvider_Impl, [expand, selection]);
// Repli pour les combinaisons sans profil dédié : tous les patterns, état revalidé à l'appel.
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IValueProvider,
    IInvokeProvider,
    ITextProvider,
    IToggleProvider,
    ISelectionProvider,
    ISelectionItemProvider,
    IRangeValueProvider,
    IExpandCollapseProvider,
    Agile = false
)]
struct FullProvider {
    core: Core,
}
provider_impl!(
    FullProvider_Impl,
    [
        value,
        invoke,
        text,
        toggle,
        selection,
        selection_item,
        range,
        expand
    ]
);

// Nouvel objet COM pour `node` ; seule la racine expose IRawElementProviderFragmentRoot.
pub(in super::super) fn create(bridge: Arc<Bridge>, node: &Node) -> IRawElementProviderSimple {
    let core = Core {
        bridge,
        key: node.key,
    };
    if node.key == NodeId::Root {
        return RootProvider { core }.into();
    }
    match Capabilities::of(node).0 {
        0 => PlainProvider { core }.into(),
        INVOKE => InvokeProvider { core }.into(),
        VALUE => ValueProvider { core }.into(),
        bits if bits == VALUE | TEXT => EditProvider { core }.into(),
        TOGGLE => ToggleProvider { core }.into(),
        SELECTION => SelectionProvider { core }.into(),
        SELECTION_ITEM => ItemProvider { core }.into(),
        RANGE => RangeProvider { core }.into(),
        bits if bits == EXPAND | SELECTION => SelectProvider { core }.into(),
        _ => FullProvider { core }.into(),
    }
}
