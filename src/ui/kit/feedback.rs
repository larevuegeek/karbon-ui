//! Feedback: alert, progress, spinner, skeleton, notifications.
use super::{
    css,
    events::OnClick,
    icons::Icon,
    props::{Number, Text},
    tone::{CustomColor, Variant, pick, tone},
};
use crate::ui::{
    element::{Element, Length},
    template::{Component, Slots, TemplateContext},
};

/// Message banner: `<Alert warning title="Disk full">Free some space.</Alert>`.
#[derive(Default)]
pub struct Alert {
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
    /// Bold first line.
    pub title: Text,
    /// Shows a close button sending this message.
    pub onclose: OnClick,
}
fn state_icon(variant: Variant) -> &'static str {
    match variant {
        Variant::Success => "check-circle",
        Variant::Warning => "alert-triangle",
        Variant::Danger => "x-circle",
        _ => "info",
    }
}
impl<R> Component<R> for Alert {
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
        let t = tone(variant, CustomColor::default());
        let closable = self.onclose.is_set();
        let close = self.onclose.action();
        kit_ui!(cx, slots;
            <div class="k-alert" style:--k-fg={t.fg} style:--k-line={t.line}>
                <div class="k-alert-icon"><Icon name={state_icon(variant)} size={18} /></div>
                <div class="k-flex k-grow">
                    {#if !self.title.0.is_empty()}<div class="k-alert-title">{self.title.0}</div>{/if}
                    <div class="k-alert-body"><slot /></div>
                </div>
                {#if closable}
                    <button class="k-badge-close" aria-label="Close" onclick={close}><Icon name="x" size={14} /></button>
                {/if}
            </div>
        )
    }
}

/// Progress bar: `<Progress value={40} success />`.
#[derive(Default)]
pub struct Progress {
    /// Current amount.
    pub value: Number,
    /// Full amount, 100 by default.
    pub max: Option<f32>,
    /// Positive colour.
    pub success: bool,
    /// Attention colour.
    pub warning: bool,
    /// Destructive colour.
    pub danger: bool,
}
impl<R> Component<R> for Progress {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let variant = pick(
            Variant::Primary,
            [false, self.success, self.warning, self.danger, false],
        );
        let t = tone(variant, CustomColor::default());
        let max = self.max.filter(|m| *m > 0.).unwrap_or(100.);
        let share = (self.value.0 / max * 100.).clamp(0., 100.);
        kit_ui!(cx;
            <div class="k-progress">
                <div class="k-progress-bar" style:background={t.fill} style:width={Length::Percent(share)}></div>
            </div>
        )
    }
}

/// Spinning wait indicator.
#[derive(Default)]
pub struct Spinner {
    /// Large size.
    pub lg: bool,
}
impl<R> Component<R> for Spinner {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let size = if self.lg { 28 } else { 18 };
        kit_ui!(cx; <Icon name="loader" size={size} class="k-spin k-spin-primary" />)
    }
}

/// Placeholder shown while content loads.
#[derive(Default)]
pub struct Skeleton {
    /// Width in pixels; the whole width by default.
    pub width: Option<f32>,
    /// Height in pixels, 14 by default.
    pub height: Option<f32>,
    /// Round shape.
    pub circle: bool,
}
impl<R> Component<R> for Skeleton {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let height = self.height.unwrap_or(14.);
        let width = self.width.map_or(Length::Percent(100.), Length::Px);
        kit_ui!(cx;
            <div class="k-skeleton" class:k-skeleton-circle={self.circle}
                style:width={width} style:height={height}></div>
        )
    }
}

/// Corner of the window holding [`Toast`] children.
#[derive(Default)]
pub struct Toasts;
impl<R> Component<R> for Toasts {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-toasts"><slot /></div>)
    }
}

/// Short notification, inside [`Toasts`].
#[derive(Default)]
pub struct Toast {
    /// Positive colour.
    pub success: bool,
    /// Attention colour.
    pub warning: bool,
    /// Destructive colour.
    pub danger: bool,
    /// Bold first line.
    pub title: Text,
    /// Shows a close button sending this message.
    pub onclose: OnClick,
}
impl<R> Component<R> for Toast {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let variant = pick(
            Variant::Info,
            [false, self.success, self.warning, self.danger, false],
        );
        let t = tone(variant, CustomColor::default());
        let closable = self.onclose.is_set();
        let close = self.onclose.action();
        kit_ui!(cx, slots;
            <div class="k-toast" style:--k-fg={t.fg}>
                <div class="k-alert-icon"><Icon name={state_icon(variant)} size={18} /></div>
                <div class="k-flex k-grow">
                    {#if !self.title.0.is_empty()}<div class="k-bold">{self.title.0}</div>{/if}
                    <div class="k-muted k-sm"><slot /></div>
                </div>
                {#if closable}
                    <button class="k-badge-close" aria-label="Close" onclick={close}><Icon name="x" size={14} /></button>
                {/if}
            </div>
        )
    }
}
