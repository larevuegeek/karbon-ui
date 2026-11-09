//! Display: card, badge, avatar, figure, table, list, empty state, accordion.
use super::{
    css,
    events::{OnChange, OnClick},
    has_slot,
    icons::Icon,
    props::{Text, Texts},
    space::Space,
    tone::{CustomColor, Variant, pick, tone, translucent},
};
use crate::ui::{
    element::Element,
    scene::Image,
    template::{Component, Slots, TemplateContext},
};
use std::sync::Arc;

/// Panel with an optional header and footer. Slots: default, `cover`, `actions`, `footer`.
#[derive(Default)]
pub struct Card {
    /// Title of the header.
    pub title: Text,
    /// Text under the title.
    pub subtitle: Text,
    /// Inner margin of the body.
    pub padding: Space,
    /// Distance between the children of the body.
    pub gap: Space,
    /// Reacts to the pointer.
    pub hover: bool,
    /// Liquid glass: blurs what lies behind it.
    pub glass: bool,
    /// Makes the whole card clickable; do not put buttons inside.
    pub onclick: OnClick,
}
impl<R> Component<R> for Card {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let header = !self.title.0.is_empty() || has_slot(&slots, "actions");
        let footer = has_slot(&slots, "footer");
        let cover = has_slot(&slots, "cover");
        let clickable = self.onclick.is_set();
        let action = self.onclick.action();
        let gap = self.gap.or(12.);
        let padding = self.padding.or(20.);
        let top = self.padding.or(if header { 14. } else { 20. });
        let bottom = if footer { 14. } else { padding };
        kit_ui!(cx, slots;
            <section class="k-card" class:k-card-hover={self.hover} class:k-surface-glass={self.glass} class:k-card-clip={cover}>
                {#if cover}<slot name="cover" />{/if}
                {#if header}
                    <div class="k-card-head">
                        <div class="k-flex k-fill">
                            <h3 class="k-title k-title-4">{self.title.0}</h3>
                            {#if !self.subtitle.0.is_empty()}
                                <div class="k-muted k-sm">{self.subtitle.0}</div>
                            {/if}
                        </div>
                        <slot name="actions" />
                    </div>
                {/if}
                <div class="k-flex k-card-body" style:padding={padding} style:padding_top={top}
                    style:padding_bottom={bottom} style:gap={gap}><slot /></div>
                {#if footer}
                    <div class="k-card-foot"><slot name="footer" /></div>
                {/if}
                {#if clickable}
                    <button class="k-card-hit" aria-label={self.title.0.as_str()} onclick={action}><div></div></button>
                {/if}
            </section>
        )
    }
}

/// Small status label: `<Badge success>Active</Badge>`.
#[derive(Default)]
pub struct Badge {
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
    /// Solid background.
    pub solid: bool,
    /// Transparent with a border.
    pub outline: bool,
    /// Shows a coloured dot.
    pub dot: bool,
    /// Shows a close button sending this message.
    pub onclose: OnClick,
}
impl<R> Component<R> for Badge {
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
        let closable = self.onclose.is_set();
        let close = self.onclose.action();
        kit_ui!(cx, slots;
            <div class="k-badge" class:k-badge-solid={self.solid} class:k-badge-outline={self.outline}
                style:--k-c={t.fill} style:--k-on={t.on} style:--k-fg={t.fg} style:--k-line={t.line}>
                {#if self.dot}<div class="k-badge-dot"></div>{/if}
                <slot />
                {#if closable}
                    <button class="k-badge-close" aria-label="Remove" onclick={close}><Icon name="x" size={12} /></button>
                {/if}
            </div>
        )
    }
}

/// Round picture or initials: `<Avatar name="Ada Lovelace" />`.
#[derive(Default)]
pub struct Avatar {
    /// Person's name: gives the initials and the colour.
    pub name: Text,
    /// Picture, in place of the initials.
    pub src: Option<Arc<Image>>,
    /// Small size.
    pub sm: bool,
    /// Large size.
    pub lg: bool,
}
impl<R> Component<R> for Avatar {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let initials: String = self
            .name
            .0
            .split_whitespace()
            .take(2)
            .filter_map(|word| word.chars().next())
            .flat_map(char::to_uppercase)
            .collect();
        let hash = self
            .name
            .0
            .bytes()
            .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
        let variants = [
            Variant::Primary,
            Variant::Success,
            Variant::Warning,
            Variant::Danger,
            Variant::Info,
        ];
        let t = tone(
            variants[hash as usize % variants.len()],
            CustomColor::default(),
        );
        kit_ui!(cx;
            <div class="k-avatar" class:k-avatar-sm={self.sm} class:k-avatar-lg={self.lg}
                style:background={translucent(t.fill, 0.18)} style:color={t.fill} aria-label={self.name.0.as_str()}>
                {#if let Some(src) = self.src}
                    <img class="k-avatar-img" src={src} alt={self.name.0.as_str()}>
                {:else}
                    <div class="k-avatar-text">{initials}</div>
                {/if}
            </div>
        )
    }
}

/// Picture and name of a person: `<User name="Ada Lovelace" description="Engineer" />`.
#[derive(Default)]
pub struct User {
    /// Person's name.
    pub name: Text,
    /// Text under the name.
    pub description: Text,
    /// Picture, in place of the initials.
    pub src: Option<Arc<Image>>,
}
impl<R> Component<R> for User {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        kit_ui!(cx;
            <div class="k-user">
                <Avatar sm name={self.name.0.as_str()} src={self.src} />
                <div class="k-flex">
                    <div class="k-user-name">{self.name.0}</div>
                    {#if !self.description.0.is_empty()}<div class="k-muted k-sm">{self.description.0}</div>{/if}
                </div>
            </div>
        )
    }
}

/// Overlapping pictures: `<AvatarGroup names={["Ada", "Alan", "Grace"]} extra={4} />`.
#[derive(Default)]
pub struct AvatarGroup {
    /// Names of the people shown.
    pub names: Texts,
    /// Number of people not shown.
    pub extra: usize,
}
impl<R> Component<R> for AvatarGroup {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let names: Vec<(usize, String)> = self.names.0.into_iter().enumerate().collect();
        let extra = self.extra;
        kit_ui!(cx;
            <div class="k-avatar-group">
                {#each names as item (item.0)}
                    <div class="k-avatar-ring" style:margin_left={if item.0 == 0 { 0. } else { -10. }}>
                        <Avatar sm name={item.1.as_str()} />
                    </div>
                {/each}
                {#if extra > 0}
                    <div class="k-avatar-ring k-avatar-more" style:margin_left={-10.}>{format!("+{extra}")}</div>
                {/if}
            </div>
        )
    }
}

/// Block of code: `<Snippet code="cargo run" />`.
#[derive(Default)]
pub struct Snippet {
    /// The code, on one or several lines.
    pub code: Text,
}
impl<R> Component<R> for Snippet {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        kit_ui!(cx; <div class="k-snippet"><pre class="k-snippet-code">{self.code.0}</pre></div>)
    }
}

/// Key figure: `<Stat label="Tracks" value="9 440" delta="+12" up />`.
#[derive(Default)]
pub struct Stat {
    /// What is measured.
    pub label: Text,
    /// The figure.
    pub value: Text,
    /// Change since the last period.
    pub delta: Text,
    /// The change is good.
    pub up: bool,
    /// The change is bad.
    pub down: bool,
    /// Icon in the corner.
    pub icon: &'static str,
}
impl<R> Component<R> for Stat {
    fn render(self, cx: &TemplateContext<'_, R>, _: Slots) -> Element {
        let variant = if self.up {
            Variant::Success
        } else if self.down {
            Variant::Danger
        } else {
            Variant::Neutral
        };
        let t = tone(variant, CustomColor::default());
        kit_ui!(cx;
            <div class="k-stat">
                <div class="k-stat-head">
                    <div class="k-muted k-sm">{self.label.0}</div>
                    {#if !self.icon.is_empty()}<div class="k-muted"><Icon name={self.icon} /></div>{/if}
                </div>
                <div class="k-stat-value">{self.value.0}</div>
                {#if !self.delta.0.is_empty()}
                    <div class="k-sm" style:color={t.fg}>{self.delta.0}</div>
                {/if}
            </div>
        )
    }
}

/// Table: `columns` and `rows` of text, or [`TableRow`] children for rich cells.
#[derive(Default)]
pub struct Table {
    /// Column titles.
    pub columns: Texts,
    /// Text rows.
    pub rows: Vec<Vec<String>>,
    /// Sent with the index of the clicked row.
    pub onrowclick: OnChange<usize>,
    /// Alternates the background of the rows.
    pub striped: bool,
    /// Tighter rows.
    pub compact: bool,
}
impl<R> Component<R> for Table {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let compact = self.compact;
        let click = self.onrowclick;
        let rows: Vec<(usize, Vec<String>)> = self.rows.into_iter().enumerate().collect();
        kit_ui!(cx, slots;
            <div class="k-table" class:k-table-striped={self.striped} class:k-table-compact={compact}>
                {#if !self.columns.0.is_empty()}
                    <div class="k-tr k-thead">
                        {#each self.columns.0 as name (name.as_str())}
                            <div class="k-td">{name}</div>
                        {/each}
                    </div>
                {/if}
                {#each rows as row (row.0)}
                    <button class="k-tr k-tr-click" onclick={click.action(row.0)}>
                        {#each row.1.iter().enumerate() as cell (cell.0)}
                            <div class="k-td">{cell.1}</div>
                        {/each}
                    </button>
                {/each}
                <slot />
            </div>
        )
    }
}

/// Row of a [`Table`] holding [`TableCell`] children.
#[derive(Default)]
pub struct TableRow {
    /// Sent when the row is clicked.
    pub onclick: OnClick,
}
impl<R> Component<R> for TableRow {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let clickable = self.onclick.is_set();
        let action = self.onclick.action();
        if clickable {
            kit_ui!(cx, slots; <button class="k-tr k-tr-click" onclick={action}><slot /></button>)
        } else {
            kit_ui!(cx, slots; <div class="k-tr"><slot /></div>)
        }
    }
}

/// Cell of a [`TableRow`].
#[derive(Default)]
pub struct TableCell;
impl<R> Component<R> for TableCell {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-td"><slot /></div>)
    }
}

/// Vertical list of [`ListItem`] children.
#[derive(Default)]
pub struct List;
impl<R> Component<R> for List {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-list"><slot /></div>)
    }
}

/// Line of a [`List`]. Slots: `leading`, `trailing`.
#[derive(Default)]
pub struct ListItem {
    /// Main text.
    pub title: Text,
    /// Text under the title.
    pub subtitle: Text,
    /// Icon at the start.
    pub icon: &'static str,
    /// Highlights the line.
    pub active: bool,
    /// Sent when the line is clicked.
    pub onclick: OnClick,
}
impl<R> Component<R> for ListItem {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let clickable = self.onclick.is_set();
        let action = self.onclick.action();
        kit_ui!(cx, slots;
            <button class="k-list-item" class:k-list-click={clickable} class:k-list-active={self.active} onclick={action}>
                <slot name="leading" />
                {#if !self.icon.is_empty()}<div class="k-muted"><Icon name={self.icon} size={18} /></div>{/if}
                <div class="k-flex k-fill">
                    <div class="k-bold">{self.title.0}</div>
                    {#if !self.subtitle.0.is_empty()}<div class="k-muted k-sm">{self.subtitle.0}</div>{/if}
                </div>
                <slot name="trailing" />
            </button>
        )
    }
}

/// Placeholder for an empty area. Slot: default, for its actions.
#[derive(Default)]
pub struct EmptyState {
    /// Large icon.
    pub icon: &'static str,
    /// What is empty.
    pub title: Text,
    /// What to do about it.
    pub text: Text,
}
impl<R> Component<R> for EmptyState {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots;
            <div class="k-empty">
                {#if !self.icon.is_empty()}<div class="k-empty-icon"><Icon name={self.icon} size={24} /></div>{/if}
                <h3 class="k-title k-title-4">{self.title.0}</h3>
                {#if !self.text.0.is_empty()}<div class="k-muted k-text-center">{self.text.0}</div>{/if}
                <slot />
            </div>
        )
    }
}

/// Section that opens and closes: the application keeps its `open` state.
#[derive(Default)]
pub struct Accordion {
    /// Title, always visible.
    pub title: Text,
    /// Shows the content.
    pub open: bool,
    /// Sent when the title is clicked.
    pub ontoggle: OnClick,
}
impl<R> Component<R> for Accordion {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let action = self.ontoggle.action();
        let icon = if self.open {
            "chevron-up"
        } else {
            "chevron-down"
        };
        kit_ui!(cx, slots;
            <div class="k-accordion">
                <button class="k-accordion-head" onclick={action} aria-expanded={self.open}>
                    <div class="k-bold k-fill">{self.title.0}</div>
                    <Icon name={icon} />
                </button>
                {#if self.open}<div class="k-accordion-body"><slot /></div>{/if}
            </div>
        )
    }
}
