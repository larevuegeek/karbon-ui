use super::*;
/// Drop-down choice among [`SelectOption`]s, controlled by the application.
///
/// Limits checked when the view is validated: at most 512 options with unique
/// keys, `selected` must name an existing option, and `visible_items` must be
/// between 1 and 20.
pub struct Select {
    /// Key of the selected option, or `None` to show the placeholder.
    pub selected: Option<u64>,
    /// Options in display order.
    pub options: Arc<[SelectOption]>,
    pub(crate) on_change: Handler<u64>,
    /// Accessible name of the control.
    pub label: Arc<str>,
    /// Text shown when no option is selected.
    pub placeholder: Arc<str>,
    /// Whether the control accepts input.
    pub enabled: bool,
    /// Maximum number of rows shown in the open list before scrolling (1–20, 8 by default).
    pub visible_items: usize,
    /// Colour overrides.
    pub style: SelectStyle,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
    text_height: std::cell::Cell<f32>,
}
impl Select {
    /// Select showing the option whose key is `selected`; choosing another option
    /// sends its key through `on_change`, for example `Msg::Theme` with `Theme(u64)`.
    pub fn new<M: 'static>(
        selected: Option<u64>,
        options: impl Into<Arc<[SelectOption]>>,
        on_change: impl Fn(u64) -> M + 'static,
    ) -> Self {
        Self {
            selected,
            options: options.into(),
            on_change: Handler::new(on_change),
            label: "Choix".into(),
            placeholder: "Choisir…".into(),
            enabled: true,
            visible_items: 8,
            style: SelectStyle::default(),
            css: None,
            text_height: std::cell::Cell::new(20.),
        }
    }
    /// Set the accessible name.
    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = label.into();
        self
    }
    /// Set the text shown when no option is selected.
    pub fn placeholder(mut self, label: impl Into<Arc<str>>) -> Self {
        self.placeholder = label.into();
        self
    }
    /// Disable the control: it is greyed out and cannot be opened.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Set the maximum number of visible rows in the open list (1–20).
    pub fn visible_items(mut self, count: usize) -> Self {
        self.visible_items = count;
        self
    }
    /// Override colours.
    pub fn style(mut self, style: SelectStyle) -> Self {
        self.style = style;
        self
    }
    pub(crate) fn validate(&self) -> crate::Result<()> {
        let mut ids = HashSet::new();
        if self.options.len() > 512
            || !(1..=20).contains(&self.visible_items)
            || self.options.iter().any(|o| !ids.insert(o.id))
            || self.selected.is_some_and(|id| !ids.contains(&id))
        {
            return Err(crate::Error::invalid_input(
                "invalid select options, selection or height",
            ));
        }
        Ok(())
    }
    pub(crate) fn format(&self) -> crate::ui::styling::text::TextFormat {
        crate::ui::styling::text::TextFormat::new(
            self.css.as_ref().map_or(Default::default(), |c| c.text),
            crate::ui::font::Font::system(false),
            14.,
        )
    }
    pub(crate) fn row_height(&self) -> f32 {
        (self.text_height.get() + 12.).max(ROW_HEIGHT)
    }
    pub(crate) fn paint_state(&self, expanded: bool, ctx: &PaintContext, scene: &mut Scene) {
        let t = ctx.theme;
        let r = ctx.rect;
        let background = if self.enabled && ctx.state.pressed {
            t.pressed
        } else if self.enabled && ctx.state.hovered {
            t.hover
        } else {
            self.style.background.unwrap_or(t.title_bar)
        };
        let radius = self.css.as_ref().and_then(|c| c.local.radius).unwrap_or(8.);
        if !self.css.as_ref().is_some_and(|c| c.element_background) {
            crate::ui::styling::paint::fill(scene, r, radius, background, self.css.as_deref());
        }
        if !self.css.as_ref().is_some_and(|c| c.element_border) {
            scene.commands.push(DrawCommand::Stroke {
                rect: RectF::new(
                    r.left + 1.,
                    r.top + 1.,
                    (layout::width(r) - 2.).max(0.),
                    (layout::height(r) - 2.).max(0.),
                ),
                radius: (radius - 1.).max(0.),
                width: if ctx.state.focused || expanded {
                    2.
                } else {
                    1.
                },
                color: if ctx.state.focused || expanded {
                    self.style.focus.unwrap_or(t.accent)
                } else {
                    self.style.border.unwrap_or(t.border)
                },
            });
        }
        let selected = self
            .selected
            .and_then(|id| self.options.iter().find(|o| o.id == id));
        let label =
            selected.map_or_else(|| Arc::clone(&self.placeholder), |o| Arc::clone(&o.label));
        let fg = self
            .css
            .as_ref()
            .and_then(|c| c.text.color)
            .unwrap_or_else(|| {
                if !self.enabled || selected.is_none() {
                    t.muted
                } else {
                    self.style.foreground.unwrap_or(t.text)
                }
            });
        let h = self.text_height.get();
        let pad = if self.css.as_ref().is_some_and(|c| c.element_padding) {
            0.
        } else {
            12.
        };
        let label_rect = RectF::new(
            r.left + pad,
            r.top + (layout::height(r) - h).max(0.) / 2.,
            (layout::width(r) - pad - 36.).max(0.),
            h + 2.,
        );
        if self.css.is_some() {
            crate::ui::styling::text::draw(scene, label_rect, label, self.format(), fg, false);
        } else {
            scene.text_single_line(
                RectF::new(
                    r.left + 12.,
                    r.top + (layout::height(r) - 20.).max(0.) / 2.,
                    (layout::width(r) - 48.).max(0.),
                    22.,
                ),
                selected.map_or_else(|| Arc::clone(&self.placeholder), |o| Arc::clone(&o.label)),
                14.,
                false,
                if !self.enabled || selected.is_none() {
                    t.muted
                } else {
                    self.style.foreground.unwrap_or(t.text)
                },
            );
        }
        // `appearance: none` : le chevron vient de l'image de fond de la feuille.
        if self
            .css
            .as_ref()
            .is_some_and(|c| c.local.appearance_none == Some(true))
        {
            return;
        }
        let x = r.right - 20.;
        let y = (r.top + r.bottom) / 2.;
        let direction = if expanded { -1. } else { 1. };
        for (start, end) in [
            (
                PointF {
                    x: x - 4.,
                    y: y - 2. * direction,
                },
                PointF {
                    x,
                    y: y + 2. * direction,
                },
            ),
            (
                PointF {
                    x,
                    y: y + 2. * direction,
                },
                PointF {
                    x: x + 4.,
                    y: y - 2. * direction,
                },
            ),
        ] {
            scene.commands.push(DrawCommand::Line {
                start,
                end,
                width: 1.5,
                color: if self.enabled { t.text } else { t.muted },
            });
        }
    }
}
impl Widget for Select {
    fn apply_style(&mut self, value: &crate::ui::styling::ComputedStyle) -> bool {
        crate::ui::styling::text::update(&mut self.css, value)
    }
    fn measure(&self, _: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        let height = if self.css.is_some() {
            crate::ui::styling::text::measure(text, "M", self.format(), 1000.)?.height
        } else {
            text.measure_text("M", 14., false, 1000.)?.height
        };
        self.text_height.set(height);
        Ok(SizeF {
            width: 240.,
            height: if self.css.as_ref().is_some_and(|c| c.element_padding) {
                height
            } else {
                (height + 20.).max(40.)
            },
        })
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        self.paint_state(false, ctx, scene);
    }
    fn select(&self) -> Option<&Select> {
        Some(self)
    }
    fn focusable(&self) -> bool {
        self.enabled
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Select,
            name: &self.label,
            enabled: self.enabled,
        }
    }
}
