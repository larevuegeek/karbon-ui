use super::*;
/// Theme-based colour scheme of a [`Button`]; a [`ButtonStyle`] with explicit
/// colours overrides it.
#[derive(Clone, Copy)]
pub enum ButtonVariant {
    /// Accent background with contrasting text (the default).
    Primary,
    /// Neutral background using the theme's title bar colour.
    Secondary,
    /// Background using the theme's danger colour, for destructive actions.
    Danger,
    /// Transparent background, no border.
    Ghost,
    /// Transparent background with a theme border.
    Outline,
}
/// Push button with a text label that emits an application message when
/// activated (click, Enter/Space or accessibility invoke).
pub struct Button {
    /// Text shown on the button and used as its accessible name.
    pub label: Arc<str>,
    /// Message emitted on activation.
    pub command: Action,
    /// Whether the button accepts input; a disabled button is greyed out.
    pub enabled: bool,
    /// Theme-based colour scheme.
    pub variant: ButtonVariant,
    /// Optional style recipe overriding the default [`ButtonStyle`].
    pub style: Option<Arc<ButtonStyle>>,
    text_height: Cell<f32>,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
    /// The label only replaces the missing content of an empty `<button aria-label>`.
    fallback_label: bool,
}
impl Button {
    /// Button emitting a clone of `message` when activated.
    pub fn new<M: Clone + 'static>(label: impl Into<Arc<str>>, message: M) -> Self {
        Self {
            label: label.into(),
            command: Action::new(message),
            enabled: true,
            variant: ButtonVariant::Primary,
            style: None,
            text_height: Cell::new(0.),
            css: None,
            fallback_label: false,
        }
    }
    /// Mark the label as a stand-in for missing content (an empty `<button>` named by
    /// `aria-label`): it stays the accessible name but is not drawn when the stylesheet
    /// paints a background image, such as the cross of Bootstrap's `.btn-close`.
    pub fn fallback_label(mut self, fallback: bool) -> Self {
        self.fallback_label = fallback;
        self
    }
    /// The label is hidden behind the stylesheet's icon.
    fn icon_only(&self) -> bool {
        self.fallback_label
            && self
                .css
                .as_ref()
                .is_some_and(|c| !c.local.background_images.is_empty())
    }
    /// Disable the button: it is greyed out and ignores activation.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Set the theme-based colour scheme.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    /// Use a custom style recipe (sizes, radius, per-state colours).
    pub fn style(mut self, style: impl Into<Arc<ButtonStyle>>) -> Self {
        self.style = Some(style.into());
        self.text_height.set(0.);
        self
    }
    fn appearance(&self) -> &ButtonStyle {
        self.style.as_deref().unwrap_or(&ButtonStyle::DEFAULT)
    }
}
impl Widget for Button {
    fn apply_style(&mut self, value: &crate::ui::styling::ComputedStyle) -> bool {
        crate::ui::styling::text::update(&mut self.css, value)
    }
    fn validate(&self) -> crate::Result<()> {
        self.appearance().validate()
    }
    fn measure(&self, _available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        self.validate()?;
        let style = self.appearance();
        let shown: &str = if self.icon_only() { "" } else { &self.label };
        let label = if let Some(css) = &self.css {
            crate::ui::styling::text::measure(
                text,
                shown,
                crate::ui::styling::text::TextFormat::new(
                    css.text,
                    crate::ui::font::Font::system(style.bold),
                    style.font_size,
                ),
                16384.,
            )?
        } else {
            text.measure_text(shown, style.font_size, style.bold, 16384.)?
        };
        let padding = if self.css.as_ref().is_some_and(|c| c.element_padding) {
            (0., 0.)
        } else {
            (style.padding_x, style.padding_y)
        };
        self.text_height.set(label.height);
        Ok(SizeF {
            width: label.width.ceil() + 2. * padding.0,
            height: (label.height + 2. * padding.1).max(
                if self.css.as_ref().is_some_and(|c| c.element_padding) {
                    0.
                } else {
                    style.min_height
                },
            ),
        })
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        let t = ctx.theme;
        let style = self.appearance();
        let light = matches!(self.variant, ButtonVariant::Ghost | ButtonVariant::Outline);
        let background = if !self.enabled {
            if light {
                Color::rgba(0, 0, 0, 0)
            } else {
                t.border
            }
        } else if ctx.state.pressed {
            t.pressed
        } else if ctx.state.hovered {
            t.hover
        } else {
            match self.variant {
                ButtonVariant::Primary => t.accent,
                ButtonVariant::Secondary => t.title_bar,
                ButtonVariant::Danger => t.danger,
                ButtonVariant::Ghost | ButtonVariant::Outline => Color::rgba(0, 0, 0, 0),
            }
        };
        let foreground = if !self.enabled {
            t.muted
        } else if matches!(self.variant, ButtonVariant::Primary | ButtonVariant::Danger)
            && !ctx.state.hovered
            && !ctx.state.pressed
        {
            t.on_accent
        } else {
            t.text
        };
        let colors = if !self.enabled {
            style.disabled
        } else if ctx.state.pressed {
            style.pressed
        } else if ctx.state.hovered {
            style.hovered
        } else {
            style.normal
        };
        let (background, foreground) = colors.map_or((background, foreground), |c| {
            (c.background.resolve(t), c.foreground.resolve(t))
        });
        let radius = self
            .css
            .as_ref()
            .and_then(|c| c.local.radius)
            .unwrap_or(style.radius);
        if !self.css.as_ref().is_some_and(|c| c.element_background) {
            crate::ui::styling::paint::fill(
                scene,
                ctx.rect,
                radius,
                background,
                self.css.as_deref(),
            );
        }
        let border = style
            .border
            .map(|c| c.resolve(t))
            .or_else(|| matches!(self.variant, ButtonVariant::Outline).then_some(t.border));
        if let Some(color) = border.filter(|_| !self.css.as_ref().is_some_and(|c| c.element_border))
        {
            crate::ui::styling::paint::border(
                scene,
                ctx.rect,
                radius,
                style.border_width,
                color,
                self.css.as_deref(),
            );
        }
        let text_height = if self.text_height.get() > 0. {
            self.text_height.get()
        } else {
            style.font_size * 1.4
        };
        let pad = if self.css.as_ref().is_some_and(|c| c.element_padding) {
            0.
        } else {
            style.padding_x
        };
        let label_rect = RectF::new(
            ctx.rect.left + pad,
            ctx.rect.top + (height(ctx.rect) - text_height).max(0.) / 2.,
            (width(ctx.rect) - 2. * pad).max(0.),
            text_height + 2.,
        );
        if self.icon_only() {
            // Icône de la feuille à la place du libellé de repli.
        } else if let Some(css) = &self.css {
            crate::ui::styling::text::draw(
                scene,
                label_rect,
                Arc::clone(&self.label),
                crate::ui::styling::text::TextFormat::new(
                    css.text,
                    crate::ui::font::Font::system(style.bold),
                    style.font_size,
                ),
                css.text.color.unwrap_or(foreground),
                false,
            );
        } else {
            scene.text_single_line(
                RectF::new(
                    ctx.rect.left + style.padding_x,
                    ctx.rect.top + (height(ctx.rect) - text_height).max(0.) / 2.,
                    (width(ctx.rect) - 2. * style.padding_x).max(0.),
                    text_height + 2.,
                ),
                Arc::clone(&self.label),
                style.font_size,
                style.bold,
                foreground,
            );
        }
        if ctx.state.focused && self.enabled && !self.css.as_ref().is_some_and(|c| c.element_border)
        {
            scene.commands.push(DrawCommand::Stroke {
                rect: RectF::new(
                    ctx.rect.left + 2.,
                    ctx.rect.top + 2.,
                    (width(ctx.rect) - 4.).max(0.),
                    (height(ctx.rect) - 4.).max(0.),
                ),
                radius: (style.radius - 2.).max(0.),
                width: 2.,
                color: style.focus.map_or(t.text, |c| c.resolve(t)),
            });
        }
    }
    fn action(&self) -> Option<Action> {
        self.enabled.then(|| self.command.clone())
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: &self.label,
            enabled: self.enabled,
        }
    }
}
