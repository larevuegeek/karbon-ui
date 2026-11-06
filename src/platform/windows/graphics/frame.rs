//! Passe de dessin d'une scène : formats et caches préparés, puis BeginDraw / EndDraw.
use super::*;

impl Renderer {
    pub(super) fn draw_scene(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
        kind: Kind,
    ) -> crate::Result<()> {
        scene.validate()?;
        self.resize(width, height, scale, kind)?;
        let mut used_formats = std::collections::HashSet::new();
        for command in &scene.commands {
            if let DrawCommand::Text { size, bold, .. } = command {
                used_formats.insert((size.to_bits(), Font::system(*bold)));
                self.format(*size, *bold)?;
            } else if let DrawCommand::FontText { size, font, .. } = command {
                used_formats.insert((size.to_bits(), *font));
                self.font_format(*size, *font)?;
            } else if let DrawCommand::FormattedText { format, .. } = command {
                used_formats.insert((format.size.to_bits(), format.font));
                self.font_format(format.size, format.font)?;
            } else if let DrawCommand::RichText { paragraph, .. } = command {
                let format = paragraph.format;
                used_formats.insert((format.size.to_bits(), format.font));
                self.font_format(format.size, format.font)?;
            }
        }
        self.formats.retain(|key, _| used_formats.contains(key));
        self.typography.prepare(
            &self.text_factory,
            &self.formats,
            self.fonts.as_ref(),
            scene,
        )?;
        let has_filters = scene
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::PushFilter { .. }));
        let visible = if has_filters {
            self.filters.prepare(
                &self.gpu.gpu().d2d,
                &self.factory,
                scene,
                &self.formats,
                &self.typography,
                scale,
            )?;
            Some(filters::visible(scene))
        } else {
            self.filters.clear();
            None
        };
        let commands = visible.as_ref().unwrap_or(scene);
        let target = &mut self.target.as_mut().unwrap().canvas;
        target.prepare(&self.factory, commands)?;
        // SAFETY: contexte Direct2D valide ; EndDraw ci-dessous referme toujours la passe.
        unsafe {
            target.target.BeginDraw();
            target.target.Clear(Some(&color_f(scene.clear)));
        }
        let scopes = &mut self.scopes;
        let result = target.draw(
            commands,
            &self.factory,
            &self.formats,
            &self.typography,
            scopes,
            &self.filters,
        );
        while let Some(scope) = scopes.pop() {
            effects::pop(&target.target, scope);
        }
        // SAFETY: referme le BeginDraw ouvert ci-dessus ; étiquettes non demandées.
        let status = unsafe { target.target.EndDraw(None, None) };
        result?;
        status.check("Direct2D EndDraw")?;
        Ok(())
    }
}
