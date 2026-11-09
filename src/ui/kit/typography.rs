//! Text: headings, paragraphs, code, keys, links.
use super::{css, events::OnClick, slot_text};
use crate::ui::{
    element::Element,
    template::{Component, Slots, TemplateContext},
};

/// Heading: `<Title>Library</Title>`, `<Title h2>Albums</Title>`.
#[derive(Default)]
pub struct Title {
    /// Very large, for a landing page.
    pub display: bool,
    /// Second level.
    pub h2: bool,
    /// Third level.
    pub h3: bool,
    /// Fourth level.
    pub h4: bool,
    /// Filled with the brand gradient.
    pub gradient: bool,
    /// Centred.
    pub center: bool,
}
impl<R> Component<R> for Title {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let text = slot_text(&mut slots);
        if self.display {
            kit_ui!(cx; <h1 class="k-title k-title-display" class:k-gradient-text={self.gradient} class:k-text-center={self.center}>{text}</h1>)
        } else if self.h2 {
            kit_ui!(cx; <h2 class="k-title k-title-2" class:k-gradient-text={self.gradient} class:k-text-center={self.center}>{text}</h2>)
        } else if self.h3 {
            kit_ui!(cx; <h3 class="k-title k-title-3" class:k-gradient-text={self.gradient} class:k-text-center={self.center}>{text}</h3>)
        } else if self.h4 {
            kit_ui!(cx; <h4 class="k-title k-title-4" class:k-gradient-text={self.gradient} class:k-text-center={self.center}>{text}</h4>)
        } else {
            kit_ui!(cx; <h1 class="k-title k-title-1" class:k-gradient-text={self.gradient} class:k-text-center={self.center}>{text}</h1>)
        }
    }
}

/// Paragraph: `<Text muted sm>Last sync 2 min ago</Text>`.
#[derive(Default)]
pub struct Text {
    /// Secondary colour.
    pub muted: bool,
    /// Small size.
    pub sm: bool,
    /// Large size.
    pub lg: bool,
    /// Bold.
    pub bold: bool,
    /// Centred.
    pub center: bool,
    /// Monospaced digits and letters.
    pub mono: bool,
}
impl<R> Component<R> for Text {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots;
            <div class="k-text" class:k-muted={self.muted} class:k-sm={self.sm} class:k-lg={self.lg}
                class:k-bold={self.bold} class:k-text-center={self.center} class:k-mono={self.mono}>
                <slot />
            </div>
        )
    }
}

/// Inline code sample.
#[derive(Default)]
pub struct Code;
impl<R> Component<R> for Code {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-code"><slot /></div>)
    }
}

/// Keyboard key: `<Kbd>Ctrl K</Kbd>`.
#[derive(Default)]
pub struct Kbd;
impl<R> Component<R> for Kbd {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        kit_ui!(cx, slots; <div class="k-kbd"><slot /></div>)
    }
}

/// Text link: `<Link onclick={Msg::Open}>Read more</Link>`.
#[derive(Default)]
pub struct Link {
    /// Message sent when activated.
    pub onclick: OnClick,
}
impl<R> Component<R> for Link {
    fn render(self, cx: &TemplateContext<'_, R>, mut slots: Slots) -> Element {
        let text = slot_text(&mut slots);
        let action = self.onclick.action();
        kit_ui!(cx; <button class="k-link" onclick={action}><div>{text}</div></button>)
    }
}
