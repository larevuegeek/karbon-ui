//! Passe de dessin d'une scène : formats et caches préparés, puis BeginDraw / EndDraw.
use super::*;

/// Ce qu'une passe redessine : toute la cible, ou des zones (pixels de la cible) avec,
/// pour chaque zone, le rang des commandes à dessiner (redessin partiel, KUI-055).
pub(super) enum Pass<'a> {
    Whole,
    Zones(&'a [(partial::Pixels, Vec<bool>)]),
}

impl Renderer {
    pub(super) fn draw_scene(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
        kind: Kind,
    ) -> crate::Result<()> {
        let visible = self.prepare_scene(scene, width, height, scale, kind)?;
        self.paint_scene(visible.as_ref().unwrap_or(scene), scene, Pass::Whole)
    }
    /// Cible à la bonne taille, formats, mises en page, filtres et ressources de la
    /// scène. Rend la scène à dessiner quand des filtres en remplacent des groupes.
    pub(super) fn prepare_scene(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
        kind: Kind,
    ) -> crate::Result<Option<Scene>> {
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
        let target = &mut self.target.as_mut().unwrap().canvas;
        target.prepare(&self.factory, visible.as_ref().unwrap_or(scene))?;
        Ok(visible)
    }
    /// Dessine `commands` (la scène préparée) dans la cible : fond, puis commandes ; avec
    /// des zones, chacune sous une découpe sans lissage, fond compris.
    pub(super) fn paint_scene(
        &mut self,
        commands: &Scene,
        scene: &Scene,
        pass: Pass,
    ) -> crate::Result<()> {
        let target = &mut self.target.as_mut().unwrap().canvas;
        let scale = {
            let (mut dpi, mut _y) = (96f32, 96f32);
            // SAFETY : contexte Direct2D valide ; sorties locales.
            unsafe { target.target.GetDpi(&mut dpi, &mut _y) };
            dpi / 96.
        };
        // SAFETY: contexte Direct2D valide ; EndDraw ci-dessous referme toujours la passe.
        unsafe { target.target.BeginDraw() };
        let scopes = &mut self.scopes;
        let mut draw = |keep: Option<&[bool]>| {
            // SAFETY: passe de dessin ouverte.
            unsafe { target.target.Clear(Some(&color_f(scene.clear))) };
            let result = target.draw_kept(
                commands,
                &self.factory,
                &self.formats,
                &self.typography,
                scopes,
                &self.filters,
                keep,
            );
            while let Some(scope) = scopes.pop() {
                effects::pop(&target.target, scope);
            }
            result
        };
        let result = match pass {
            Pass::Whole => draw(None),
            Pass::Zones(zones) => zones.iter().try_for_each(|(zone, keep)| {
                let clip = partial::dips(*zone, scale);
                // SAFETY: passe de dessin ouverte ; découpe retirée juste après.
                unsafe {
                    target
                        .target
                        .PushAxisAlignedClip(&rect_f(clip), D2D1_ANTIALIAS_MODE_ALIASED)
                };
                let result = draw(Some(keep));
                // SAFETY: dépile la découpe posée ci-dessus.
                unsafe { target.target.PopAxisAlignedClip() };
                result
            }),
        };
        // SAFETY: referme le BeginDraw ouvert ci-dessus ; étiquettes non demandées.
        let status = unsafe { target.target.EndDraw(None, None) };
        result?;
        status.check("Direct2D EndDraw")?;
        Ok(())
    }
}
