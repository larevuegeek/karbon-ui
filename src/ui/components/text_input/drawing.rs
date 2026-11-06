use super::*;
impl TextInputs {
    pub fn paint(&self, id: ElementId, input: &TextInput, ctx: &PaintContext, scene: &mut Scene) {
        let Some(e) = self.states.get(&id) else {
            input.paint(ctx, scene);
            return;
        };
        if e.rows.is_some() {
            self.paint_rows(id, e, input, ctx, scene);
            return;
        }
        let focused = self.focused == Some(id) && self.active;
        paint_input_container(input, ctx.rect, ctx.theme, focused, scene);
        scene.commands.push(DrawCommand::PushClip(e.clip));
        let line = e.line.as_ref().unwrap();
        let top = e.rect.top + (layout::height(e.rect) - line.height).max(0.) / 2.;
        if focused && e.composition.is_none() {
            for mut rect in line.selection(e.range()) {
                rect.left += e.rect.left + e.text_offset - e.scroll;
                rect.right += e.rect.left + e.text_offset - e.scroll;
                rect.top += top;
                rect.bottom += top;
                scene.fill(rect, 0., ctx.theme.pressed);
            }
        }
        let color = input
            .css
            .as_ref()
            .and_then(|c| c.text.color)
            .unwrap_or_else(|| {
                if !input.enabled {
                    ctx.theme.muted
                } else {
                    input.appearance().foreground.unwrap_or(ctx.theme.text)
                }
            });
        let mut format = input.format();
        let left = if e.display.is_empty() {
            e.rect.left
        } else {
            format.align = crate::ui::styling::TextAlign::Start;
            e.rect.left + e.text_offset
        };
        let rect = RectF::new(
            left - e.scroll,
            top,
            line.width.max(layout::width(e.rect)),
            line.height + 2.,
        );
        let value = if e.display.is_empty() {
            Arc::clone(&input.placeholder)
        } else {
            Arc::clone(&e.display)
        };
        let ink = if e.display.is_empty() {
            input
                .css
                .as_ref()
                .and_then(|c| c.local.placeholder_color)
                .unwrap_or(ctx.theme.muted)
        } else {
            color
        };
        if input.css.is_some() {
            crate::ui::styling::text::draw(scene, rect, value, format, ink, false);
        } else {
            scene.text_single_line(
                RectF::new(
                    e.rect.left - e.scroll,
                    top,
                    line.width.max(layout::width(e.rect)) + 2.,
                    line.height + 2.,
                ),
                if e.display.is_empty() {
                    Arc::clone(&input.placeholder)
                } else {
                    Arc::clone(&e.display)
                },
                input.style.font_size,
                false,
                if e.display.is_empty() {
                    ctx.theme.muted
                } else {
                    color
                },
            );
        }
        if let Some(range) = e
            .composition
            .as_ref()
            .map(|c| c.range.start..c.range.start + c.value.len())
        {
            for rect in line.selection(range) {
                scene.fill(
                    RectF::new(
                        e.rect.left + e.text_offset - e.scroll + rect.left,
                        top + line.height - 1.,
                        layout::width(rect),
                        1.,
                    ),
                    0.,
                    ctx.theme.accent,
                );
            }
        }
        if focused && self.caret_visible {
            scene.fill(e.caret(), 0., color);
        }
        scene.commands.push(DrawCommand::PopClip);
    }
}
