//! Navigation: tabs, breadcrumb, steps, top bar, side panel.
use super::{
    css,
    events::{OnChange, OnClick},
    icons::Icon,
    props::{Text, Texts},
};
use crate::ui::{
    element::Element,
    template::{Component, Slots, TemplateContext},
};

/// Tabs: `<Tabs items={["Albums", "Artists"]} value={self.tab} onchange={Msg::Tab} />`.
#[derive(Default)]
pub struct Tabs {
    /// Tab titles.
    pub items: Texts,
    /// Index of the active tab.
    pub value: usize,
    /// Sent with the index of the clicked tab.
    pub onchange: OnChange<usize>,
    /// A line under the active tab instead of a raised pill.
    pub underlined: bool,
}
impl<R> Component<R> for Tabs {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let items: Vec<(usize, String)> = self.items.0.into_iter().enumerate().collect();
        let active = self.value;
        let change = self.onchange;
        let under = self.underlined;
        kit_ui!(cx;
            <div class="k-tabs" class:k-tabs-under={under} role="tablist">
                {#each items as item (item.0)}
                    <button class="k-tab" class:k-tab-active={item.0 == active && !under} class:k-tab-under={under}
                        class:k-tab-under-active={under && item.0 == active} role="tab"
                        aria-selected={item.0 == active} onclick={change.action(item.0)}><div>{item.1}</div></button>
                {/each}
            </div>
        )
    }
}

/// Path to the current page; the last item is the current one.
#[derive(Default)]
pub struct Breadcrumb {
    /// Page names.
    pub items: Texts,
    /// Sent with the index of the clicked item.
    pub onchange: OnChange<usize>,
}
impl<R> Component<R> for Breadcrumb {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let last = self.items.0.len().saturating_sub(1);
        let items: Vec<(usize, String)> = self.items.0.into_iter().enumerate().collect();
        let change = self.onchange;
        kit_ui!(cx;
            <nav class="k-crumbs" aria-label="Breadcrumb">
                {#each items as item (item.0)}
                    {#if item.0 == last}
                        <div class="k-bold">{item.1}</div>
                    {:else}
                        <button class="k-link k-crumb" onclick={change.action(item.0)}><div>{item.1}</div></button>
                        <div class="k-muted"><Icon name="chevron-right" size={14} /></div>
                    {/if}
                {/each}
            </nav>
        )
    }
}

/// Numbered progress through a procedure.
#[derive(Default)]
pub struct Steps {
    /// Step names.
    pub items: Texts,
    /// Index of the current step.
    pub current: usize,
}
impl<R> Component<R> for Steps {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let current = self.current;
        let last = self.items.0.len().saturating_sub(1);
        let items: Vec<(usize, String)> = self.items.0.into_iter().enumerate().collect();
        kit_ui!(cx;
            <div class="k-steps">
                {#each items as item (item.0)}
                    <div class="k-step">
                        <div class="k-step-dot" class:k-step-done={item.0 < current} class:k-step-now={item.0 == current}>
                            {#if item.0 < current}<Icon name="check" size={14} />{:else}<div>{item.0 + 1}</div>{/if}
                        </div>
                        <div class:k-bold={item.0 == current} class:k-muted={item.0 > current}>{item.1}</div>
                    </div>
                    {#if item.0 != last}<div class="k-step-line"></div>{/if}
                {/each}
            </div>
        )
    }
}

/// Top bar. Slots: `logo`, default (links), `actions`.
#[derive(Default)]
pub struct Navbar {
    /// Name at the start of the bar.
    pub brand: Text,
    /// Liquid glass: blurs what lies behind it.
    pub glass: bool,
}
impl<R> Component<R> for Navbar {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots;
            <header class="k-navbar" class:k-surface-glass={self.glass}>
                <div class="k-brand-row"><slot name="logo" /><div class="k-brand">{self.brand.0}</div></div>
                <div class="k-navbar-links"><slot /></div>
                <div class="k-spacer"></div>
                <div class="k-navbar-links"><slot name="actions" /></div>
            </header>
        )
    }
}

/// Side panel holding [`NavItem`] children.
#[derive(Default)]
pub struct Sidebar {
    /// Name at the top of the panel.
    pub brand: Text,
    /// In a window narrower than 900 pixels, only the icons of the links remain (their
    /// text stays readable by screen readers).
    pub responsive: bool,
}
impl<R> Component<R> for Sidebar {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let brand = !self.brand.0.is_empty();
        kit_ui!(cx, slots;
            <aside class="k-sidebar" class:k-sidebar-responsive={self.responsive}>
                {#if brand}<div class="k-brand k-sidebar-brand">{self.brand.0}</div>{/if}
                <slot />
            </aside>
        )
    }
}

/// Heading of a group of links in a [`Sidebar`].
#[derive(Default)]
pub struct NavLabel;
impl<R> Component<R> for NavLabel {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-nav-label"><slot /></div>)
    }
}

/// Link of a [`Sidebar`] or a [`Navbar`].
#[derive(Default)]
pub struct NavItem {
    /// Icon before the text.
    pub icon: &'static str,
    /// Current page.
    pub active: bool,
    /// Sent when clicked.
    pub onclick: OnClick,
}
impl<R> Component<R> for NavItem {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let action = self.onclick.action();
        kit_ui!(cx, slots;
            <button class="k-nav-item" class:k-nav-active={self.active} onclick={action}>
                {#if !self.icon.is_empty()}<Icon name={self.icon} size={18} />{/if}
                <div class="k-nav-text"><slot /></div>
            </button>
        )
    }
}
