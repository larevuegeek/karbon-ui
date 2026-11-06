use super::*;
/// Text field controlled by the application, on one line or, with
/// [`multiline`](Self::multiline), on several (`<textarea>` in templates). The
/// editing state (selection, undo history, IME composition, vertical scroll) is kept
/// by the runtime per element ID, so the element ID must stay stable between views.
pub struct TextInput {
    /// Text supplied by the application.
    pub value: Arc<str>,
    /// Hint shown while the field is empty.
    pub placeholder: Arc<str>,
    /// Accessible name.
    pub label: Arc<str>,
    pub(crate) on_change: Handler<String>,
    pub(crate) on_submit: Option<Action>,
    pub(crate) on_key: Option<Handler<crate::ui::runtime::Key>>,
    /// Whether the field accepts input.
    pub enabled: bool,
    /// Whether the text can be selected and copied but not edited.
    pub read_only: bool,
    /// Maximum number of characters (4096 by default, at most 65536).
    pub max_chars: usize,
    /// Show every character as a bullet, hide the value from assistive technologies
    /// and refuse copy/cut (password field).
    pub masked: bool,
    /// Accept several lines: the text wraps at the field width, Enter inserts a line
    /// break (instead of [`on_submit`](Self::on_submit)), Up/Down move between visual
    /// lines and the content scrolls vertically.
    pub multiline: bool,
    /// Visible lines giving the natural height of a multiline field (2 by default, as
    /// HTML `rows`, 1 to 1000).
    pub rows: u16,
    /// Appearance.
    pub style: InputStyle,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
    /// Labels of the built-in context menu (cut, copy, paste, select all).
    pub(crate) menu_labels: EditMenuLabels,
}
impl TextInput {
    pub(crate) fn format(&self) -> crate::ui::styling::text::TextFormat {
        let mut format = crate::ui::styling::text::TextFormat::new(
            self.css.as_ref().map_or(Default::default(), |c| c.text),
            crate::ui::font::Font::system(false),
            self.style.font_size,
        );
        // La valeur éditée reste telle quelle sur une ligne : ni casse, ni coupure, ni ellipse.
        format.transform = Default::default();
        format.white_space = crate::ui::styling::WhiteSpace::Normal;
        format.ellipsis = false;
        format.max_lines = 0;
        format
    }
    pub(crate) fn appearance(&self) -> InputStyle {
        let mut style = self.style;
        if let Some(css) = &self.css {
            style.font_size = css.text.size.unwrap_or(style.font_size);
            style.foreground = css.text.color.or(style.foreground);
            style.radius = css.local.radius.unwrap_or(style.radius);
            style.background = css.local.background.or(style.background);
            style.border = css.local.border.or(style.border);
            if css.element_padding {
                style.padding = 0.;
            }
        }
        style
    }
    /// Single-line field showing `value`; each edit sends the whole new text through
    /// `on_change` (for example `Msg::NameChanged` with `NameChanged(String)`). The
    /// application keeps the value and passes it back on the next view.
    pub fn new<M: 'static>(
        value: impl Into<Arc<str>>,
        on_change: impl Fn(String) -> M + 'static,
    ) -> Self {
        Self {
            value: value.into(),
            placeholder: "".into(),
            label: "Champ de saisie".into(),
            on_change: Handler::new(on_change),
            on_submit: None,
            on_key: None,
            enabled: true,
            read_only: false,
            max_chars: 4096,
            masked: false,
            multiline: false,
            rows: 2,
            style: InputStyle::default(),
            css: None,
            menu_labels: DEFAULT_MENU_LABELS,
        }
    }
    /// Translate the built-in context menu (right click, Shift+F10 or the Menu key):
    /// labels of cut, copy, paste and select all, in that order. The menu is drawn by
    /// the engine on every platform; an element handler set with
    /// [`context_menu`](Element::context_menu) on the field or one of its ancestors
    /// replaces it.
    pub fn menu_labels(mut self, labels: EditMenuLabels) -> Self {
        self.menu_labels = labels;
        self
    }
    /// Set the hint shown while the field is empty.
    pub fn placeholder(mut self, value: impl Into<Arc<str>>) -> Self {
        self.placeholder = value.into();
        self
    }
    /// Set the accessible name.
    pub fn label(mut self, value: impl Into<Arc<str>>) -> Self {
        self.label = value.into();
        self
    }
    /// Accept several lines (see [`multiline`](Self::multiline)).
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }
    /// Set the number of visible lines of a multiline field (see [`rows`](Self::rows)).
    pub fn rows(mut self, count: u16) -> Self {
        self.rows = count;
        self
    }
    /// Emit a clone of `message` when Enter is pressed (single-line fields only).
    pub fn on_submit<M: Clone + 'static>(mut self, message: M) -> Self {
        self.on_submit = Some(Action::new(message));
        self
    }
    /// Send `handler(key)` for each key pressed in the field (`onkeydown` in templates),
    /// before the field handles it: arrows, Page Up/Down, Enter, Escape, Home, End,
    /// Tab, Backspace, Delete, Space (typed text arrives through `on_change`). Up, Down,
    /// Page Up and Page Down are then consumed by the field, so an application can move
    /// a highlighted suggestion with them (a combobox, see
    /// [`Popover::combobox`](crate::ui::popover::Popover::combobox)). Nothing is sent
    /// during an IME composition.
    pub fn on_key<M: 'static>(
        mut self,
        handler: impl Fn(crate::ui::runtime::Key) -> M + 'static,
    ) -> Self {
        self.on_key = Some(Handler::new(handler));
        self
    }
    /// Disable the field: it is greyed out and ignores input.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Allow selecting and copying but not editing.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
    /// Mask the value like a password field (see [`masked`](Self::masked)).
    pub fn password(mut self) -> Self {
        self.masked = true;
        self
    }
    /// Limit the number of characters (at most 65536); extra input is dropped.
    pub fn max_chars(mut self, count: usize) -> Self {
        self.max_chars = count;
        self
    }
    /// Set the appearance.
    pub fn style(mut self, style: InputStyle) -> Self {
        self.style = style;
        self
    }
    pub(crate) fn validate(&self) -> crate::Result<()> {
        if !self.style.font_size.is_finite()
            || !(1. ..=128.).contains(&self.style.font_size)
            || !self.style.padding.is_finite()
            || self.style.padding < 0.
            || !self.style.radius.is_finite()
            || self.style.radius < 0.
            || self.max_chars > 65_536
            || !(1..=1000).contains(&self.rows)
        {
            return Err(crate::Error::invalid_input(
                "invalid text input style or length limit",
            ));
        }
        Ok(())
    }
    pub(super) fn inner(&self, rect: RectF) -> RectF {
        let p = self.appearance().padding;
        RectF::new(
            rect.left + p,
            rect.top + p,
            (layout::width(rect) - 2. * p).max(0.),
            (layout::height(rect) - 2. * p).max(0.),
        )
    }
}
impl Widget for TextInput {
    fn apply_style(&mut self, value: &crate::ui::styling::ComputedStyle) -> bool {
        crate::ui::styling::text::update(&mut self.css, value)
    }
    fn measure(&self, _: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        let appearance = self.appearance();
        let font = if self.css.is_some() {
            crate::ui::styling::text::measure(text, "M", self.format(), 1000.)?
        } else {
            text.measure_text("M", appearance.font_size, false, 1000.)?
        };
        let lines = if self.multiline {
            f32::from(self.rows)
        } else {
            1.
        };
        Ok(SizeF {
            width: 240.,
            height: (font.height * lines + 2. * appearance.padding).max(
                if self.css.as_ref().is_some_and(|c| c.element_padding) {
                    0.
                } else {
                    42.
                },
            ),
        })
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        paint_input_container(self, ctx.rect, ctx.theme, false, scene);
        let inner = self.inner(ctx.rect);
        let value = if self.value.is_empty() {
            Arc::clone(&self.placeholder)
        } else {
            Arc::clone(&self.value)
        };
        if let Some(css) = &self.css {
            crate::ui::styling::text::draw(
                scene,
                inner,
                value,
                self.format(),
                match css.local.placeholder_color {
                    Some(color) if self.value.is_empty() => color,
                    _ => css.text.color.unwrap_or(ctx.theme.text),
                },
                self.multiline,
            );
        } else {
            scene.text_single_line(
                inner,
                if self.value.is_empty() {
                    Arc::clone(&self.placeholder)
                } else {
                    Arc::clone(&self.value)
                },
                self.style.font_size,
                false,
                ctx.theme.text,
            );
        }
    }
    fn text_input(&self) -> Option<&TextInput> {
        Some(self)
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::TextInput,
            name: &self.label,
            enabled: self.enabled,
        }
    }
}
pub(super) fn paint_input_container(
    input: &TextInput,
    rect: RectF,
    theme: Theme,
    focused: bool,
    scene: &mut Scene,
) {
    let style = input.appearance();
    if !input.css.as_ref().is_some_and(|c| c.element_background) {
        crate::ui::styling::paint::fill(
            scene,
            rect,
            style.radius,
            style.background.unwrap_or(theme.title_bar),
            input.css.as_deref(),
        );
    }
    if !input.css.as_ref().is_some_and(|c| c.element_border) {
        scene.commands.push(DrawCommand::Stroke {
            rect: RectF::new(
                rect.left + 1.,
                rect.top + 1.,
                (layout::width(rect) - 2.).max(0.),
                (layout::height(rect) - 2.).max(0.),
            ),
            radius: style.radius,
            width: if focused { 2. } else { 1. },
            color: if focused {
                style.focus_border.unwrap_or(theme.accent)
            } else {
                style.border.unwrap_or(theme.border)
            },
        });
    }
}
