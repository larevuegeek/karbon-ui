//! Traduction des commandes de la scène en appels Core Graphics, avec la pile des
//! portées (clips, effets, masques) et des remplissages du texte.
use super::super::{
    bitmap::Bitmap,
    ffi::{affine, rect as cg_rect},
    fonts::Fonts,
};
use super::{geometry, text};
use crate::platform::shared::{scope_end, single};
use crate::ui::{
    rendering::{brush::Background, transform::Affine},
    scene::{DrawCommand, RectF},
    theme::Color,
};
use objc2_core_foundation::{CGAffineTransform, CGFloat};
use objc2_core_graphics::{CGContext, CGLineCap, CGLineJoin};
use std::sync::Arc;

/// Portée ouverte : transformation à restaurer et calque de transparence éventuel.
struct Scope {
    transform: Affine,
    layer: bool,
}
pub(super) struct Canvas<'a> {
    pub context: &'a CGContext,
    /// Taille de la cible en pixels.
    pub width: usize,
    pub height: usize,
    /// Pixels de la cible relus par `backdrop-filter` ; absents dans une couche de filtre.
    pub surface: Option<&'a Bitmap>,
    /// Repère logique courant vers les pixels de la cible (y vers le bas).
    pub transform: Affine,
    pub fonts: &'a mut Fonts,
    pub text: &'a mut text::Cache,
    pub caches: &'a mut super::cache::Caches,
    pub text_fills: Vec<(Background, RectF)>,
    scopes: Vec<Scope>,
}
/// Matrice qui retourne l'axe y d'une cible de `height` pixels.
pub(super) fn flip(height: usize) -> CGAffineTransform {
    CGAffineTransform {
        a: 1.,
        b: 0.,
        c: 0.,
        d: -1.,
        tx: 0.,
        ty: height as CGFloat,
    }
}
impl<'a> Canvas<'a> {
    /// Canvas sur `context` (cible de `width` × `height` pixels, repère Core Graphics
    /// initial) ; `transform` va du repère logique aux pixels de la cible.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: &'a CGContext,
        width: usize,
        height: usize,
        surface: Option<&'a Bitmap>,
        transform: Affine,
        fonts: &'a mut Fonts,
        text: &'a mut text::Cache,
        caches: &'a mut super::cache::Caches,
    ) -> Self {
        let c = Some(context);
        CGContext::concat_ctm(c, flip(height));
        CGContext::concat_ctm(c, affine(transform));
        CGContext::set_should_smooth_fonts(c, false);
        CGContext::set_line_cap(c, CGLineCap::Butt);
        CGContext::set_line_join(c, CGLineJoin::Miter);
        Self {
            context,
            width,
            height,
            surface,
            transform,
            fonts,
            text,
            caches,
            text_fills: Vec::new(),
            scopes: Vec::new(),
        }
    }
    fn push(&mut self, layer: bool) {
        CGContext::save_g_state(Some(self.context));
        self.scopes.push(Scope {
            transform: self.transform,
            layer,
        });
    }
    fn pop(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            let c = Some(self.context);
            if scope.layer {
                CGContext::end_transparency_layer(c);
            }
            CGContext::restore_g_state(c);
            self.transform = scope.transform;
        }
    }
    /// Referme les portées restées ouvertes (après une erreur).
    pub fn finish(&mut self) {
        while !self.scopes.is_empty() {
            self.pop();
        }
    }
    pub fn run(&mut self, commands: &[DrawCommand]) -> crate::Result<()> {
        let mut index = 0;
        while index < commands.len() {
            let command = &commands[index];
            index += 1;
            match command {
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    let start = index;
                    let mut depth = 1usize;
                    while depth > 0 && index < commands.len() {
                        match commands[index] {
                            DrawCommand::PushFilter { .. } => depth += 1,
                            DrawCommand::PopFilter => depth -= 1,
                            _ => {}
                        }
                        index += 1;
                    }
                    self.filter(*id, &commands[start..index - 1], *bounds, *filters)?;
                }
                DrawCommand::PopFilter => {}
                DrawCommand::PushEffect {
                    transform,
                    opacity,
                    bounds,
                } => {
                    let end = scope_end(commands, index - 1);
                    // Contenu invisible (menus et infobulles masqués) : rien à peindre.
                    if *opacity <= 1. / 512. {
                        index = end + 1;
                        continue;
                    }
                    let layer = *opacity < 1. && !single(&commands[index..end]);
                    self.effect(*transform, *opacity, *bounds, layer);
                }
                other => self.draw(other)?,
            }
        }
        Ok(())
    }
    fn draw(&mut self, command: &DrawCommand) -> crate::Result<()> {
        let c = Some(self.context);
        match command {
            DrawCommand::PushFilter { .. } | DrawCommand::PopFilter => {}
            DrawCommand::Shape {
                rect,
                radii,
                fill,
                stroke,
            } => {
                let width = stroke.map(|(w, _)| w);
                let (inner, radii) = geometry::normalized(*rect, *radii, width);
                let path = geometry::rounded(inner, radii);
                match (fill, stroke) {
                    (_, Some((w, color))) => {
                        let w = w.min(rect.right - rect.left).min(rect.bottom - rect.top);
                        self.set_stroke(*color, w);
                        self.stroke_path(&path);
                    }
                    (Some(fill), None) => self.fill_background(&path, fill, *rect)?,
                    (None, None) => {}
                }
            }
            DrawCommand::RadialGradient {
                rect,
                radius,
                gradient,
            } => self.fill_background(
                &geometry::round_rect(*rect, *radius),
                &Background::Radial(Arc::clone(gradient)),
                *rect,
            )?,
            DrawCommand::Gradient {
                rect,
                radius,
                gradient,
            } => self.fill_background(
                &geometry::round_rect(*rect, *radius),
                &Background::Linear(Arc::clone(gradient)),
                *rect,
            )?,
            DrawCommand::FormattedText {
                rect,
                text,
                format,
                color,
                wrap,
            } => self.paragraph(&text::plain(text, *format), *rect, *color, *wrap)?,
            DrawCommand::RichText {
                rect,
                paragraph,
                color,
                wrap,
            } => self.paragraph(paragraph, *rect, *color, *wrap)?,
            DrawCommand::Text {
                rect,
                text,
                size,
                bold,
                color,
                wrap,
            } => {
                let format = text::simple_format(crate::ui::font::Font::system(*bold), *size);
                self.paragraph(&text::plain(text, format), *rect, *color, *wrap)?;
            }
            DrawCommand::FontText {
                rect,
                text,
                size,
                font,
                color,
                wrap,
            } => {
                let format = text::simple_format(*font, *size);
                self.paragraph(&text::plain(text, format), *rect, *color, *wrap)?;
            }
            DrawCommand::PushEffect { .. } => unreachable!("effects handled by run"),
            DrawCommand::PopEffect
            | DrawCommand::PopShapeClip
            | DrawCommand::PopClip
            | DrawCommand::PopMask => self.pop(),
            DrawCommand::Fill {
                rect,
                radius,
                color,
            } => {
                self.set_fill(*color);
                if *radius > 0. {
                    self.fill_path(&geometry::round_rect(*rect, *radius), false);
                } else {
                    CGContext::fill_rect(c, cg_rect(*rect));
                }
            }
            DrawCommand::Stroke {
                rect,
                radius,
                width,
                color,
            } => {
                self.set_stroke(*color, *width);
                self.stroke_path(&geometry::round_rect(*rect, *radius));
            }
            DrawCommand::Line {
                start,
                end,
                width,
                color,
            } => {
                self.set_stroke(*color, *width);
                self.stroke_path(&geometry::polyline(&[*start, *end], false));
            }
            DrawCommand::Path {
                points,
                closed,
                color,
            } => {
                if points.len() >= 2 {
                    let path = geometry::polyline(points, *closed);
                    if *closed {
                        self.set_fill(*color);
                        self.fill_path(&path, true);
                    } else {
                        self.set_stroke(*color, 2.);
                        self.stroke_path(&path);
                    }
                }
            }
            DrawCommand::Vector {
                rect,
                icon,
                color,
                outline,
            } => {
                if !(icon.width.is_finite()
                    && icon.width > 0.
                    && icon.height.is_finite()
                    && icon.height > 0.)
                {
                    return Err(crate::Error::invalid_input("invalid icon dimensions"));
                }
                let path = geometry::icon(*rect, icon);
                self.set_fill(*color);
                self.fill_path(&path, true);
                if *outline > 0. {
                    self.set_stroke(*color, outline * (rect.right - rect.left) / icon.width);
                    self.stroke_path(&path);
                }
            }
            DrawCommand::Svg { rect, image, color } => self.svg(*rect, image, *color),
            DrawCommand::Image { rect, image } => self.image(*rect, image)?,
            DrawCommand::PushShapeClip { rect, radii } => {
                self.push(false);
                let (rect, radii) = geometry::normalized(*rect, *radii, None);
                CGContext::add_path(c, Some(&geometry::rounded(rect, radii).0));
                CGContext::clip(c);
            }
            DrawCommand::PushClip(rect) => {
                self.push(false);
                CGContext::clip_to_rect(c, cg_rect(*rect));
            }
            DrawCommand::PushMask { bounds, gradient } => {
                self.push(false);
                self.mask(*bounds, gradient)?;
            }
            DrawCommand::PushTextFill { rect, fill } => self.text_fills.push((fill.clone(), *rect)),
            DrawCommand::PopTextFill => {
                self.text_fills.pop();
            }
            DrawCommand::Backdrop {
                rect,
                radii,
                filters,
            } => self.backdrop(*rect, *radii, *filters)?,
        }
        Ok(())
    }
    /// Ouvre une portée d'effet. Un calque n'est nécessaire que pour l'opacité d'un groupe
    /// de plusieurs éléments ; il est limité aux bornes du contenu, comme `contentBounds`
    /// sous Direct2D (sinon Core Graphics alloue et recompose toute la découpe).
    fn effect(&mut self, transform: Affine, opacity: f32, bounds: RectF, layer: bool) {
        let c = Some(self.context);
        self.push(layer);
        CGContext::concat_ctm(c, affine(transform));
        self.transform = transform.then(self.transform);
        if opacity < 1. {
            CGContext::set_alpha(c, CGFloat::from(opacity.max(0.)));
        }
        if layer {
            // SAFETY : contexte valide ; aucun dictionnaire auxiliaire.
            unsafe { CGContext::begin_transparency_layer_with_rect(c, cg_rect(bounds), None) };
        }
    }
    /// Texte mis en page dans `rect`, ou masque de glyphes sous `background-clip: text`.
    fn paragraph(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        rect: RectF,
        color: Color,
        wrap: bool,
    ) -> crate::Result<()> {
        let layout = self
            .text
            .layout(self.fonts, paragraph, rect.right - rect.left, wrap)?;
        match self.text_fills.last().cloned() {
            Some((fill, area)) => self.text_fill(&layout, rect, &fill, area),
            None => {
                layout.draw(self.context, rect, color, None);
                Ok(())
            }
        }
    }
    fn svg(&self, rect: RectF, image: &crate::ui::rendering::svg::SvgImage, color: Color) {
        use crate::ui::rendering::svg::{FillRule, LineCap, LineJoin};
        let c = Some(self.context);
        CGContext::save_g_state(c);
        CGContext::concat_ctm(c, affine(image.placement(rect)));
        for shape in image.shapes {
            let path = geometry::svg(shape);
            if let Some(fill) = shape.fill.resolve(color, shape.fill_opacity) {
                self.set_fill(fill);
                self.fill_path(&path, shape.fill_rule == FillRule::EvenOdd);
            }
            if let Some(stroke) = shape.stroke.resolve(color, shape.stroke_opacity)
                && shape.stroke_width > 0.
            {
                CGContext::set_line_cap(
                    c,
                    match shape.line_cap {
                        LineCap::Butt => CGLineCap::Butt,
                        LineCap::Round => CGLineCap::Round,
                        LineCap::Square => CGLineCap::Square,
                    },
                );
                CGContext::set_line_join(
                    c,
                    match shape.line_join {
                        LineJoin::Miter => CGLineJoin::Miter,
                        LineJoin::Round => CGLineJoin::Round,
                        LineJoin::Bevel => CGLineJoin::Bevel,
                    },
                );
                CGContext::set_miter_limit(c, CGFloat::from(shape.miter_limit));
                self.set_stroke(stroke, shape.stroke_width);
                self.stroke_path(&path);
            }
        }
        CGContext::restore_g_state(c);
    }
}
