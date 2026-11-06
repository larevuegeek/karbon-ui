use super::*;
/// Entry of a menu [`Popover`](super::Popover); choosing it emits its message
/// and dismisses the menu.
pub struct MenuItem {
    /// Displayed label, also used as the accessible name.
    pub label: Arc<str>,
    /// Message emitted when the item is chosen.
    pub command: Action,
    /// Whether the item can be chosen.
    pub enabled: bool,
    /// Shortcut hint shown on the right (display only).
    pub shortcut: Arc<str>,
    /// Label colour override.
    pub foreground: Option<Color>,
}
impl MenuItem {
    /// Menu item emitting a clone of `message` when chosen.
    pub fn new<M: Clone + 'static>(label: impl Into<Arc<str>>, message: M) -> Self {
        Self {
            label: label.into(),
            command: Action::new(message),
            enabled: true,
            shortcut: "".into(),
            foreground: None,
        }
    }
    /// Show the item greyed out and prevent choosing it.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Hint text only; the application must wire up the shortcut itself.
    pub fn shortcut(mut self, value: impl Into<Arc<str>>) -> Self {
        self.shortcut = value.into();
        self
    }
    /// Override the label colour, for example for a destructive item.
    pub fn color(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }
}
impl Widget for MenuItem {
    fn measure(&self, _: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        let label = text.measure_text(&self.label, 14., false, 16384.)?;
        let hint = text.measure_text(&self.shortcut, 12., false, 16384.)?;
        Ok(SizeF {
            width: label.width + hint.width + 48.,
            height: 36.,
        })
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        let t = ctx.theme;
        let r = ctx.rect;
        if self.enabled && (ctx.state.hovered || ctx.state.focused || ctx.state.pressed) {
            scene.fill(
                r,
                5.,
                if ctx.state.pressed {
                    t.pressed
                } else {
                    t.hover
                },
            );
        }
        let hint_width = if self.shortcut.is_empty() {
            0.
        } else {
            (self.shortcut.chars().count() as f32 * 8. + 16.).min(layout::width(r) / 2.)
        };
        scene.text_single_line(
            RectF::new(
                r.left + 10.,
                r.top + 8.,
                (layout::width(r) - 20. - hint_width).max(0.),
                22.,
            ),
            Arc::clone(&self.label),
            14.,
            false,
            if self.enabled {
                self.foreground.unwrap_or(t.text)
            } else {
                t.muted
            },
        );
        if hint_width > 0. {
            scene.text_single_line(
                RectF::new(
                    r.right - hint_width,
                    r.top + 9.,
                    (hint_width - 6.).max(0.),
                    20.,
                ),
                Arc::clone(&self.shortcut),
                12.,
                false,
                t.muted,
            );
        }
        if self.enabled && ctx.state.focused {
            scene.commands.push(DrawCommand::Stroke {
                rect: r,
                radius: 5.,
                width: 1.,
                color: t.accent,
            });
        }
    }
    fn action(&self) -> Option<Action> {
        self.enabled.then(|| self.command.clone())
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::MenuItem,
            name: &self.label,
            enabled: self.enabled,
        }
    }
}
/// Horizontal line separating groups of menu items.
pub struct MenuSeparator;
impl Widget for MenuSeparator {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: 1.,
            height: 9.,
        })
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        scene.fill(
            RectF::new(
                ctx.rect.left + 4.,
                ctx.rect.top + 4.,
                (layout::width(ctx.rect) - 8.).max(0.),
                1.,
            ),
            0.,
            ctx.theme.border,
        );
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Separator,
            name: "",
            enabled: true,
        }
    }
}
