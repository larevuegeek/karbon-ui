//! Buttons.
use super::{
    css,
    events::OnClick,
    icons::Icon,
    tone::{CustomColor, Variant, pick, tone},
};
use crate::ui::{
    element::Element,
    template::{Component, Slots, TemplateContext},
};

/// Clickable button.
///
/// ```ignore
/// <Button primary icon="plus" onclick={Msg::Add}>Add</Button>
/// <Button danger outline sm onclick={Msg::Delete}>Delete</Button>
/// ```
#[derive(Default)]
pub struct Button {
    /// Brand colour.
    pub primary: bool,
    /// Positive colour.
    pub success: bool,
    /// Attention colour.
    pub warning: bool,
    /// Destructive colour.
    pub danger: bool,
    /// Informative colour.
    pub info: bool,
    /// Colour family chosen at run time.
    pub variant: Variant,
    /// One-off colour.
    pub color: CustomColor,
    /// Transparent with a border.
    pub outline: bool,
    /// Transparent until hovered.
    pub ghost: bool,
    /// Tinted background.
    pub soft: bool,
    /// Looks like a link.
    pub link: bool,
    /// Tinted background with a border.
    pub faded: bool,
    /// Coloured shadow.
    pub shadow: bool,
    /// Translucent glass over what lies behind it.
    pub glass: bool,
    /// Fully rounded ends.
    pub pill: bool,
    /// Small size.
    pub sm: bool,
    /// Large size.
    pub lg: bool,
    /// Take the whole width.
    pub full: bool,
    /// Inactive.
    pub disabled: bool,
    /// Shows a spinner and ignores clicks.
    pub loading: bool,
    /// Icon before the label.
    pub icon: &'static str,
    /// Icon after the label.
    pub icon_right: &'static str,
    /// Message sent when activated.
    pub onclick: OnClick,
}
impl<R> Component<R> for Button {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let variant = pick(
            self.variant,
            [
                self.primary,
                self.success,
                self.warning,
                self.danger,
                self.info,
            ],
        );
        let t = tone(variant, self.color);
        let inactive = self.disabled || self.loading;
        let action = self.onclick.action();
        let shadow = |wanted: Variant| self.shadow && variant == wanted;
        kit_ui!(cx, slots;
            <button class="k-btn" class:k-btn-outline={self.outline} class:k-btn-ghost={self.ghost}
                class:k-btn-soft={self.soft} class:k-btn-faded={self.faded} class:k-btn-link={self.link}
                class:k-btn-glass={self.glass} class:k-btn-sm={self.sm} class:k-btn-lg={self.lg}
                class:k-btn-pill={self.pill} class:k-btn-full={self.full}
                class:k-shadow-primary={shadow(Variant::Primary)} class:k-shadow-success={shadow(Variant::Success)}
                class:k-shadow-warning={shadow(Variant::Warning)} class:k-shadow-danger={shadow(Variant::Danger)}
                class:k-shadow-info={shadow(Variant::Info)} class:k-shadow-neutral={shadow(Variant::Neutral)}
                style:--k-c={t.fill} style:--k-on={t.on} style:--k-fg={t.fg} style:--k-line={t.line}
                disabled={inactive} onclick={action}>
                {#if self.loading}
                    <Icon name="loader" class="k-spin" />
                {:else if !self.icon.is_empty()}
                    <Icon name={self.icon} />
                {/if}
                <slot />
                {#if !self.icon_right.is_empty()}
                    <Icon name={self.icon_right} />
                {/if}
            </button>
        )
    }
}

/// Square button holding a single icon; `label` names it for screen readers.
#[derive(Default)]
pub struct IconButton {
    /// Icon name.
    pub icon: &'static str,
    /// Accessible name.
    pub label: String,
    /// Brand colour.
    pub primary: bool,
    /// Destructive colour.
    pub danger: bool,
    /// Colour family chosen at run time.
    pub variant: Variant,
    /// Transparent with a border.
    pub outline: bool,
    /// Tinted background.
    pub soft: bool,
    /// Solid fill instead of the default transparent look.
    pub solid: bool,
    /// Round.
    pub pill: bool,
    /// Small size.
    pub sm: bool,
    /// Large size.
    pub lg: bool,
    /// Inactive.
    pub disabled: bool,
    /// Message sent when activated.
    pub onclick: OnClick,
}
impl<R> Component<R> for IconButton {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let variant = pick(
            self.variant,
            [self.primary, false, false, self.danger, false],
        );
        let t = tone(variant, CustomColor::default());
        let ghost = !self.outline && !self.soft && !self.solid;
        let action = self.onclick.action();
        kit_ui!(cx;
            <button class="k-btn" class:k-btn-sm={self.sm} class:k-btn-lg={self.lg}
                class:k-btn-square={!self.sm && !self.lg} class:k-btn-square-sm={self.sm} class:k-btn-square-lg={self.lg}
                class:k-btn-ghost={ghost} class:k-btn-outline={self.outline} class:k-btn-soft={self.soft}
                class:k-btn-pill={self.pill}
                style:--k-c={t.fill} style:--k-on={t.on} style:--k-fg={t.fg} style:--k-line={t.line}
                aria-label={self.label.as_str()} disabled={self.disabled} onclick={action}>
                <Icon name={self.icon} />
            </button>
        )
    }
}
