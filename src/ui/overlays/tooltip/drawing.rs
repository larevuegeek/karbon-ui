use super::*;
impl Tooltips {
    pub fn paint(&self, scene: &mut Scene, theme: Theme) {
        let Some(info) = self.snapshot() else {
            return;
        };
        let t = &self.target.as_ref().unwrap().tooltip;
        let r = info.bounds;
        scene.commands.push(DrawCommand::PushClip(r));
        scene.fill(r, 6., t.style.background.unwrap_or(theme.title_bar));
        scene.commands.push(DrawCommand::Stroke {
            rect: r,
            radius: 6.,
            width: 1.,
            color: t.style.border.unwrap_or(theme.border),
        });
        scene.text(
            RectF::new(
                r.left + 10.,
                r.top + 8.,
                (layout::width(r) - 20.).max(0.),
                (layout::height(r) - 16.).max(0.),
            ),
            info.text,
            t.style.font_size,
            false,
            t.style.foreground.unwrap_or(theme.text),
        );
        scene.commands.push(DrawCommand::PopClip);
    }
}
