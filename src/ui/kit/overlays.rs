//! Overlays: dialog, drop-down menu, tooltip.
use super::{
    buttons::{Button, IconButton},
    css,
    events::OnClick,
    has_slot,
    icons::Icon,
    props::Text,
    space::Space,
};
use crate::ui::{
    element::{Element, ElementId},
    popover::Anchor,
    template::{Component, Slots, TemplateContext},
};

/// Modal dialog. Slots: default, `footer`.
///
/// ```ignore
/// {#if self.confirm}
///     <Dialog title="Delete?" onclose={Msg::Cancel}>
///         <Text>This cannot be undone.</Text>
///         <Button slot="footer" danger onclick={Msg::Delete}>Delete</Button>
///     </Dialog>
/// {/if}
/// ```
#[derive(Default)]
pub struct Dialog {
    /// Title and accessible name.
    pub title: Text,
    /// Sent by Escape, the close button and a click outside.
    pub onclose: OnClick,
    /// Hides the dialog when `false`, for `open={self.visible}`.
    pub open: Option<bool>,
    /// Small width.
    pub sm: bool,
    /// Large width.
    pub lg: bool,
    /// Liquid glass: blurs what lies behind it.
    pub glass: bool,
    /// Distance between the children of the body.
    pub gap: Space,
}
impl<R> Component<R> for Dialog {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let footer = has_slot(&slots, "footer");
        let close = self.onclose.action();
        let dismiss = close.clone();
        let gap = self.gap.or(12.);
        let open = self.open.unwrap_or(true);
        if !open {
            return kit_ui!(cx; <div class="k-none"></div>);
        }
        kit_ui!(cx, slots;
            <dialog class="k-dialog" class:k-dialog-sm={self.sm} class:k-dialog-lg={self.lg} class:k-surface-glass={self.glass}
                aria-label={self.title.0.as_str()} onclose={dismiss} backdrop="rgba(0,0,0,0.45)">
                <div class="k-dialog-head">
                    <h2 class="k-title k-title-3 k-grow">{self.title.0}</h2>
                    <IconButton icon="x" label="Close" sm onclick={close} />
                </div>
                <div class="k-flex k-dialog-body" style:gap={gap}><slot /></div>
                {#if footer}<div class="k-dialog-foot"><slot name="footer" /></div>{/if}
            </dialog>
        )
    }
}

/// Button that opens a menu of [`MenuItem`] children; the application keeps `open`.
///
/// ```ignore
/// <Dropdown label="Options" open={self.menu} ontoggle={Msg::ToggleMenu}>
///     <MenuItem icon="edit" onclick={Msg::Rename}>Rename</MenuItem>
/// </Dropdown>
/// ```
#[derive(Default)]
pub struct Dropdown {
    /// Text of the button.
    pub label: Text,
    /// Shows the menu.
    pub open: bool,
    /// Sent when the button is clicked and when the menu closes.
    pub ontoggle: OnClick,
    /// Brand colour of the button.
    pub primary: bool,
    /// Aligns the menu with the right of the button.
    pub end: bool,
}
impl<R> Component<R> for Dropdown {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let anchor: ElementId = cx.id.child("k-trigger");
        let toggle = self.ontoggle.action();
        kit_ui!(cx, slots;
            <div class="k-dropdown">
                <Button id={anchor} primary={self.primary} icon_right="chevron-down" onclick={toggle.clone()}>{self.label.0}</Button>
                {#if self.open}
                    {#if self.end}
                        <k-popover class="k-menu" anchor={Anchor::Element(anchor)} aria-label={self.label.0.as_str()}
                            onclose={toggle} menu placement="bottom-end" offset="4">
                            <slot />
                        </k-popover>
                    {:else}
                        <k-popover class="k-menu" anchor={Anchor::Element(anchor)} aria-label={self.label.0.as_str()}
                            onclose={toggle} menu placement="bottom-start" offset="4">
                            <slot />
                        </k-popover>
                    {/if}
                {/if}
            </div>
        )
    }
}

/// Entry of a [`Dropdown`].
#[derive(Default)]
pub struct MenuItem {
    /// Icon before the text.
    pub icon: &'static str,
    /// Destructive colour.
    pub danger: bool,
    /// Inactive.
    pub disabled: bool,
    /// Sent when chosen.
    pub onclick: OnClick,
}
impl<R> Component<R> for MenuItem {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let action = self.onclick.action();
        kit_ui!(cx, slots;
            <button class="k-menu-item" class:k-menu-danger={self.danger} role="menuitem"
                disabled={self.disabled} onclick={action}>
                {#if !self.icon.is_empty()}<Icon name={self.icon} />{/if}
                <slot />
            </button>
        )
    }
}

/// Shows a hint when its content is hovered: `<Tooltip text="Save"><Button…/></Tooltip>`.
#[derive(Default)]
pub struct Tooltip {
    /// Text of the hint.
    pub text: Text,
}
impl<R> Component<R> for Tooltip {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-tip" title={self.text.0.as_str()}><slot /></div>)
    }
}
