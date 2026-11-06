use super::*;
impl Selects {
    pub fn paint_control(&self, id: ElementId, s: &Select, ctx: &PaintContext, scene: &mut Scene) {
        s.paint_state(self.expanded(id), ctx, scene);
    }

    pub fn paint_popup(&self, root: &Element, theme: Theme, scene: &mut Scene) {
        let Some(o) = &self.open else {
            return;
        };
        let Some((s, _, _)) = control(root, o.id) else {
            return;
        };
        let r = o.layer.bounds;
        scene.commands.push(DrawCommand::PushClip(o.viewport));
        scene.fill(
            RectF::new(r.left, r.top + 3., layout::width(r), layout::height(r)),
            8.,
            Color::rgba(0, 0, 0, 55),
        );
        scene.fill(r, 8., s.style.background.unwrap_or(theme.surface));
        scene.commands.push(DrawCommand::Stroke {
            rect: r,
            radius: 8.,
            width: 1.,
            color: s.style.border.unwrap_or(theme.border),
        });
        scene.commands.push(DrawCommand::PushClip(o.inner()));
        if o.options.is_empty() {
            scene.text_single_line(o.row(0), "Aucune option", 14., false, theme.muted);
        }
        for (index, option) in o
            .options
            .iter()
            .enumerate()
            .skip(o.first)
            .take(o.rows() + 1)
        {
            let row = o.row(index);
            let active = o.active == Some(option.id) && option.enabled;
            if active {
                scene.fill(
                    row,
                    4.,
                    if o.pressed == Some(option.id) {
                        theme.pressed
                    } else {
                        s.style.highlight.unwrap_or(theme.hover)
                    },
                );
            }
            if option.enabled && Some(option.id) == o.selected {
                scene.fill(
                    RectF::new(row.left + 8., row.top + 13., 6., 6.),
                    3.,
                    theme.accent,
                );
            }
            let h = (o.row_height - 12.).max(14.);
            let label_rect = RectF::new(
                row.left + 24.,
                row.top + 6.,
                (layout::width(row) - 30.).max(0.),
                h + 2.,
            );
            let fg = if option.enabled {
                s.css
                    .as_ref()
                    .and_then(|c| c.text.color)
                    .or(s.style.foreground)
                    .unwrap_or(theme.text)
            } else {
                theme.muted
            };
            if s.css.is_some() {
                crate::ui::styling::text::draw(
                    scene,
                    label_rect,
                    Arc::clone(&option.label),
                    s.format(),
                    fg,
                    false,
                );
            } else {
                scene.text_single_line(
                    RectF::new(
                        row.left + 24.,
                        row.top + 6.,
                        (layout::width(row) - 30.).max(0.),
                        22.,
                    ),
                    Arc::clone(&option.label),
                    14.,
                    false,
                    if option.enabled {
                        s.style.foreground.unwrap_or(theme.text)
                    } else {
                        theme.muted
                    },
                );
            }
            if active {
                scene.commands.push(DrawCommand::Stroke {
                    rect: RectF::new(
                        row.left + 1.,
                        row.top + 1.,
                        (layout::width(row) - 2.).max(0.),
                        o.row_height - 2.,
                    ),
                    radius: 3.,
                    width: 1.,
                    color: s.style.focus.unwrap_or(theme.accent),
                });
            }
        }
        scene.commands.push(DrawCommand::PopClip);
        if let Some((track, thumb)) = o.scroll_bar() {
            scene.fill(track, 3., theme.border);
            scene.fill(
                thumb,
                3.,
                if o.drag_scroll.is_some() {
                    theme.accent
                } else {
                    theme.muted
                },
            );
        }
        scene.commands.push(DrawCommand::PopClip);
    }
}
