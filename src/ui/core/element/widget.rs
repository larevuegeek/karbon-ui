use super::*;
/// Text measurement service; the backend measures with the same font stack it draws with.
///
/// Only [`measure_text`](Self::measure_text) is required; the other methods have
/// approximate defaults that native backends override.
///
/// # Errors
///
/// Every method returns [`Error::InvalidInput`](crate::Error::InvalidInput) for an
/// invalid font or size, or the backend's [`Error::Platform`](crate::Error::Platform)
/// when its text stack fails; layout stops at the first error.
pub trait TextMeasurer {
    /// Measure `value` with a full text format (font, size, line height) wrapped at `width`.
    fn measure_formatted(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        let mut measured = self.measure_font(value, format.size, format.font, width)?;
        if let Some(line) = format.line_height {
            measured.height = (measured.height / (format.size * 1.2)).ceil().max(1.) * line;
        }
        Ok(measured)
    }
    /// Lay out a single line with the given font; used for caret and selection geometry.
    fn layout_font_line(
        &mut self,
        value: &str,
        size: f32,
        _font: crate::ui::font::Font,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        self.layout_line(value, size)
    }
    /// Lay out a single line with a full text format; `line_height` overrides the line height.
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        let mut line = self.layout_font_line(value, format.size, format.font)?;
        if let Some(height) = format.line_height {
            line.height = height;
        }
        Ok(line)
    }
    /// Lay out an editable text wrapped at `width`, for a multiline
    /// [`TextInput`](crate::ui::text_input::TextInput): one
    /// [`WrappedLine`](crate::ui::text_input::WrappedLine) per visual line, in order,
    /// at least one (an empty text gives one empty line). `\n` ends a line; a text
    /// ending with `\n` ends with an empty line.
    ///
    /// Each line carries the caret stops and clusters of
    /// [`layout_formatted_line`](Self::layout_formatted_line) applied to that line
    /// alone, with byte offsets relative to the whole text: the field draws every line
    /// on its own, so its glyphs match this geometry. The default lays each paragraph
    /// out with `layout_formatted_line` and breaks it greedily after spaces (a word
    /// wider than `width` is broken between clusters); native backends break lines
    /// with their own text stack.
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: crate::ui::styling::text::TextFormat,
        width: f32,
    ) -> crate::Result<Vec<crate::ui::text_input::WrappedLine>> {
        crate::ui::text_input::wrap_text(self, value, format, width)
    }
    /// Measure `text` with the given font, wrapped at `max_width`.
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: crate::ui::font::Font,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.measure_text(text, size, font.weight >= 600, max_width)
    }
    /// Measure a [`RichParagraph`](crate::ui::styling::text::RichParagraph) wrapped at
    /// `width`. The default measures the whole text with the base format, ignoring the
    /// per-range sizes and families; native backends lay out every range.
    fn measure_rich(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
    ) -> crate::Result<SizeF> {
        paragraph.validate()?;
        self.measure_formatted(&paragraph.text, paragraph.format, width)
    }
    /// Rectangles covered by the bytes `range` of `paragraph` laid out at `width`, one per
    /// line fragment, relative to the layout origin. Used to hit-test links.
    ///
    /// The default lays the paragraph out on a single line with the base format; native
    /// backends return the wrapped geometry.
    fn rich_text_rects(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<RectF>> {
        let _ = width;
        let line = self.layout_formatted_line(&paragraph.text, paragraph.format)?;
        let mut rect: Option<RectF> = None;
        for cluster in &line.clusters {
            if cluster.start < range.end && cluster.end > range.start {
                rect = Some(match rect {
                    Some(r) => RectF {
                        left: r.left.min(cluster.rect.left),
                        top: r.top.min(cluster.rect.top),
                        right: r.right.max(cluster.rect.right),
                        bottom: r.bottom.max(cluster.rect.bottom),
                    },
                    None => cluster.rect,
                });
            }
        }
        Ok(rect.into_iter().collect())
    }
    /// Measure `text` at `size`, wrapped at `max_width`.
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        max_width: f32,
    ) -> crate::Result<SizeF>;
    /// Lay out a single line. Native backends replace this simple computation with the
    /// clusters of their text stack.
    fn layout_line(
        &mut self,
        value: &str,
        size: f32,
    ) -> crate::Result<crate::ui::text_input::TextLine> {
        use crate::ui::text_input::{TextCaret, TextCluster, TextLine};
        let mut carets = vec![TextCaret { byte: 0, x: 0. }];
        let mut clusters = Vec::new();
        let mut previous = 0.;
        for (byte, c) in value.char_indices() {
            let end = byte + c.len_utf8();
            let measured = self.measure_text(&value[..end], size, false, 1_000_000.)?;
            carets.push(TextCaret {
                byte: end,
                x: measured.width,
            });
            clusters.push(TextCluster {
                start: byte,
                end,
                rect: RectF::new(
                    previous,
                    0.,
                    (measured.width - previous).max(0.),
                    measured.height,
                ),
            });
            previous = measured.width;
        }
        Ok(TextLine {
            carets,
            clusters,
            width: previous,
            height: self.measure_text("M", size, false, 1_000_000.)?.height,
        })
    }
}

/// Accessibility role exposed to assistive technologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Static text.
    Text,
    /// Push button.
    Button,
    /// Visual separator.
    Separator,
    /// Scrollable area.
    ScrollArea,
    /// List of items.
    List,
    /// Editable text field.
    TextInput,
    /// Check box.
    Checkbox,
    /// On/off switch.
    Switch,
    /// Radio button.
    RadioButton,
    /// Group of radio buttons.
    RadioGroup,
    /// Slider.
    Slider,
    /// Drop-down selection.
    Select,
    /// Dialog.
    Dialog,
    /// Popover.
    Popover,
    /// Menu.
    Menu,
    /// Menu item.
    MenuItem,
    /// Hyperlink inside a text.
    Link,
    /// Image.
    Image,
    /// Heading of a section.
    Heading,
    /// Item of a list.
    ListItem,
    /// Generic group of related elements (sections, articles, forms…).
    Group,
    /// Main content of the page.
    Main,
    /// Navigation links.
    Navigation,
    /// Page or section header.
    Banner,
    /// Page or section footer.
    ContentInfo,
    /// Content complementary to the main content.
    Complementary,
    /// Custom widget without a specific role.
    Custom,
}
/// Tri-state value of a check box or switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    /// Unchecked.
    Off,
    /// Checked.
    On,
    /// Indeterminate.
    Mixed,
}
impl CheckState {
    /// State after a toggle: `Off` and `Mixed` become `On`, `On` becomes `Off`.
    pub fn next(self) -> Self {
        match self {
            Self::Off | Self::Mixed => Self::On,
            Self::On => Self::Off,
        }
    }
}
/// Checked state of a choice widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceState {
    /// Check box or switch state.
    Toggle(CheckState),
    /// Radio button, `true` when selected.
    Radio(bool),
}
/// Accessibility description of a widget.
pub struct Semantics<'a> {
    /// Role exposed to assistive technologies.
    pub role: Role,
    /// Accessible name.
    pub name: &'a str,
    /// Whether the widget accepts input.
    pub enabled: bool,
}
/// Interaction state used when painting a widget.
#[derive(Default, Clone, Copy)]
pub struct VisualState {
    /// Pointer is over the widget.
    pub hovered: bool,
    /// Widget is being pressed.
    pub pressed: bool,
    /// Widget has keyboard focus.
    pub focused: bool,
}
/// Everything a widget needs to paint itself.
pub struct PaintContext {
    /// Rectangle assigned by layout.
    pub rect: RectF,
    /// Natural size measured during layout.
    pub intrinsic: SizeF,
    /// Active theme.
    pub theme: Theme,
    /// Hover, press and focus state.
    pub state: VisualState,
}

/// Contract of a widget; new widgets can implement it without modifying the backend.
pub trait Widget {
    /// Called on style changes before measurement. Custom widgets may opt in;
    /// return `true` when the change affects layout.
    fn apply_style(&mut self, _style: &crate::ui::styling::ComputedStyle) -> bool {
        false
    }
    /// Optional validation specific to a custom widget, run before layout.
    ///
    /// # Errors
    ///
    /// Return [`Error::InvalidInput`](crate::Error::InvalidInput) to reject the widget's
    /// configuration; [`layout::validate`](crate::ui::layout::validate) reports it.
    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }
    /// Natural size of the widget given the `available` space.
    ///
    /// # Errors
    ///
    /// Typically propagates the errors of `text`; layout stops at the first error.
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF>;
    /// Append the drawing commands of the widget to `scene`.
    fn paint(&self, context: &PaintContext, scene: &mut Scene);
    /// Message emitted on activation (click, Enter/Space, accessibility Invoke).
    /// `None` makes the widget inert.
    fn action(&self) -> Option<Action> {
        None
    }
    /// Text input state, when the widget is a text field.
    fn text_input(&self) -> Option<&crate::ui::text_input::TextInput> {
        None
    }
    /// Checked state, when the widget is a check box, switch or radio button.
    fn choice(&self) -> Option<ChoiceState> {
        None
    }
    /// Slider state, when the widget is a slider.
    fn slider(&self) -> Option<&crate::ui::slider::Slider> {
        None
    }
    /// Select state, when the widget is a drop-down selection.
    fn select(&self) -> Option<&crate::ui::select::Select> {
        None
    }
    /// Modal options, when the widget is a dialog.
    fn modal(&self) -> Option<&crate::ui::modal::Modal> {
        None
    }
    /// Popover options, when the widget is a popover.
    fn popover(&self) -> Option<&crate::ui::popover::Popover> {
        None
    }
    /// Whether the widget takes keyboard focus; by default when it has an action or is an enabled text input.
    fn focusable(&self) -> bool {
        self.action().is_some() || self.text_input().is_some_and(|input| input.enabled)
    }
    /// Accessibility description; defaults to a [`Role::Custom`] with an empty name.
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Custom,
            name: "",
            enabled: true,
        }
    }
    /// Number of inline child elements this leaf widget positions itself (for example the
    /// links of a [`RichText`](crate::ui::components::RichText)). A leaf element may only
    /// have this many children. Defaults to 0.
    fn inline_children(&self) -> usize {
        0
    }
    /// Rectangles of the inline children, in window coordinates, once the content box
    /// `area` is known; called during layout after [`measure`](Self::measure). Must return
    /// [`inline_children`](Self::inline_children) rectangles. Defaults to none.
    ///
    /// # Errors
    ///
    /// Typically propagates the errors of `text`.
    fn inline_boxes(
        &mut self,
        area: RectF,
        text: &mut dyn TextMeasurer,
    ) -> crate::Result<Vec<RectF>> {
        let _ = (area, text);
        Ok(Vec::new())
    }
    /// Refines pointer hit-testing inside the element's content box `rect`: return
    /// `false` when `point` falls outside the widget's actual shape. Defaults to `true`.
    fn hit_test(&self, rect: RectF, point: crate::ui::scene::PointF) -> bool {
        let _ = (rect, point);
        true
    }
}
