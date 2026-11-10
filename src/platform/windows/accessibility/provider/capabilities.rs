pub(super) use super::super::*;
pub(super) use std::panic::{AssertUnwindSafe, catch_unwind};
pub(super) use windows::{
    Win32::{
        Foundation::{E_FAIL, HWND},
        System::{Com::SAFEARRAY, Variant::VARIANT},
        UI::Accessibility::{
            ExpandCollapseState, IExpandCollapseProvider, IExpandCollapseProvider_Impl,
            IInvokeProvider, IInvokeProvider_Impl, IRangeValueProvider, IRangeValueProvider_Impl,
            IRawElementProviderFragment, IRawElementProviderFragment_Impl,
            IRawElementProviderFragmentRoot, IRawElementProviderFragmentRoot_Impl,
            IRawElementProviderSimple_Impl, ISelectionItemProvider, ISelectionItemProvider_Impl,
            ISelectionProvider, ISelectionProvider_Impl, ITextProvider, ITextProvider_Impl,
            ITextRangeProvider, IToggleProvider, IToggleProvider_Impl, IValueProvider,
            IValueProvider_Impl, NavigateDirection, ProviderOptions, SupportedTextSelection,
            ToggleState, UIA_PATTERN_ID, UiaHostProviderFromHwnd, UiaPoint,
        },
    },
    core::{BOOL, BSTR, IUnknown, PCWSTR, Ref, implement},
};
pub(super) use windows_sys::Win32::System::Threading::GetCurrentProcessId;

/// Aucun panic ne traverse la frontière COM : il devient E_FAIL.
pub(crate) fn boundary<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|_| Err(fail(E_FAIL)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Capabilities(pub(super) u8);
pub(super) const VALUE: u8 = 1;
pub(super) const INVOKE: u8 = 2;
pub(super) const TEXT: u8 = 4;
pub(super) const TOGGLE: u8 = 8;
pub(super) const SELECTION: u8 = 16;
pub(super) const SELECTION_ITEM: u8 = 32;
pub(super) const RANGE: u8 = 64;
pub(super) const EXPAND: u8 = 128;
impl Capabilities {
    pub(crate) fn of(node: &Node) -> Self {
        Self(
            [
                (node.value.is_some(), VALUE),
                (node.invokable, INVOKE),
                (node.text.is_some(), TEXT),
                (matches!(node.choice, Some(ChoiceState::Toggle(_))), TOGGLE),
                (node.selection_container(), SELECTION),
                (
                    matches!(node.choice, Some(ChoiceState::Radio(_))),
                    SELECTION_ITEM,
                ),
                (node.range.is_some(), RANGE),
                (node.expanded.is_some(), EXPAND),
            ]
            .into_iter()
            .filter(|(present, _)| *present)
            .fold(0, |bits, (_, bit)| bits | bit),
        )
    }
}

/// État commun à toutes les classes : le nœud est relu dans le snapshot à chaque appel.
pub(crate) struct Core {
    pub(crate) bridge: Arc<Bridge>,
    pub(crate) key: NodeId,
}
