//! Traduction des commandes de la scène en appels Cairo, avec la pile des portées
//! (clips, effets, masques) et des remplissages du texte.
//!
//! Cairo n'a pas d'opacité globale : l'opacité d'un groupe réduit à un seul élément est
//! portée par `alpha` et appliquée à chaque source (couleurs, dégradés, images) ; un
//! groupe de plusieurs éléments est dessiné dans un groupe Cairo recomposé à son opacité.
use super::super::{bitmap::Bitmap, fonts::Fonts};
use super::{geometry, paint, text};
use crate::{
    platform::{
        linux::ffi::cairo::{
            CAP_BUTT, CAP_ROUND, CAP_SQUARE, Context, JOIN_BEVEL, JOIN_MITER, JOIN_ROUND, Matrix,
        },
        shared::{normalized, scope_end, single},
    },
    ui::{
        rendering::{
            brush::{Background, LinearGradient},
            transform::Affine,
        },
        scene::{DrawCommand, RectF},
        theme::Color,
    },
};
use std::sync::Arc;

/// Nature d'une portée ouverte : ce que sa fermeture doit recomposer.
enum Kind {
    /// État sauvegardé seulement (découpe, transformation).
    Plain,
    /// Groupe recomposé à cette opacité.
    Layer(f32),
    /// Groupe recomposé à travers l'alpha d'un dégradé posé sur `bounds` (`mask-image`).
    Mask(Arc<LinearGradient>, RectF),
}
/// Portée ouverte : transformation et opacité à restaurer.
struct Scope {
    transform: Affine,
    alpha: f32,
    kind: Kind,
}
pub(super) struct Canvas<'a> {
    pub context: &'a Context<'a>,
    /// Taille de la cible en pixels.
    pub width: usize,
    pub height: usize,
    /// Pixels de la cible relus par `backdrop-filter` ; absents dans une couche de filtre.
    pub surface: Option<&'a Bitmap>,
    /// Repère logique courant vers les pixels de la cible.
    pub transform: Affine,
    /// Opacité appliquée à chaque source dessinée (groupe d'un seul élément).
    pub alpha: f32,
    pub fonts: &'a mut Fonts,
    pub text: &'a mut text::Cache,
    pub caches: &'a mut super::cache::Caches,
    pub text_fills: Vec<(Background, RectF)>,
    scopes: Vec<Scope>,
}
impl<'a> Canvas<'a> {
    /// Canvas sur `context` (repère initial : pixels de la cible) ; `transform` va du
    /// repère logique aux pixels de la cible.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: &'a Context<'a>,
        width: usize,
        height: usize,
        surface: Option<&'a Bitmap>,
        transform: Affine,
        fonts: &'a mut Fonts,
        text: &'a mut text::Cache,
        caches: &'a mut super::cache::Caches,
    ) -> Self {
        context.transform(Matrix::from_affine(transform));
        context.set_line_cap(CAP_BUTT);
        context.set_line_join(JOIN_MITER);
        Self {
            context,
            width,
            height,
            surface,
            transform,
            alpha: 1.,
            fonts,
            text,
            caches,
            text_fills: Vec::new(),
            scopes: Vec::new(),
        }
    }
    fn push(&mut self, kind: Kind) {
        self.context.save();
        self.scopes.push(Scope {
            transform: self.transform,
            alpha: self.alpha,
            kind,
        });
    }
    fn pop(&mut self) {
        let Some(scope) = self.scopes.pop() else {
            return;
        };
        let c = self.context;
        match scope.kind {
            Kind::Plain => {}
            Kind::Layer(opacity) => {
                c.pop_group_to_source();
                c.paint_with_alpha(opacity.into());
            }
            Kind::Mask(gradient, bounds) => {
                c.pop_group_to_source();
                match paint::linear(&gradient, bounds, 1., true) {
                    Ok(mask) => c.mask(&mask),
                    Err(_) => c.paint(),
                }
            }
        }
        c.restore();
        self.transform = scope.transform;
        self.alpha = scope.alpha;
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
        let c = self.context;
        match command {
            DrawCommand::PushFilter { .. } | DrawCommand::PopFilter => {}
            DrawCommand::Shape {
                rect,
                radii,
                fill,
                stroke,
            } => {
                let width = stroke.map(|(w, _)| w);
                let (inner, radii) = normalized(*rect, *radii, width);
                match (fill, stroke) {
                    (_, Some((w, color))) => {
                        let w = w.min(rect.right - rect.left).min(rect.bottom - rect.top);
                        self.set_stroke(*color, w);
                        geometry::rounded(c, inner, radii);
                        c.stroke();
                    }
                    (Some(fill), None) => {
                        self.fill_background(|c| geometry::rounded(c, inner, radii), fill, *rect)?;
                    }
                    (None, None) => {}
                }
            }
            DrawCommand::RadialGradient {
                rect,
                radius,
                gradient,
            } => self.fill_background(
                |c| geometry::round_rect(c, *rect, *radius),
                &Background::Radial(Arc::clone(gradient)),
                *rect,
            )?,
            DrawCommand::Gradient {
                rect,
                radius,
                gradient,
            } => self.fill_background(
                |c| geometry::round_rect(c, *rect, *radius),
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
                geometry::round_rect(c, *rect, *radius);
                self.fill_path(false);
            }
            DrawCommand::Stroke {
                rect,
                radius,
                width,
                color,
            } => {
                self.set_stroke(*color, *width);
                geometry::round_rect(c, *rect, *radius);
                c.stroke();
            }
            DrawCommand::Line {
                start,
                end,
                width,
                color,
            } => {
                self.set_stroke(*color, *width);
                geometry::polyline(c, &[*start, *end], false);
                c.stroke();
            }
            DrawCommand::Path {
                points,
                closed,
                color,
            } => {
                if points.len() >= 2 {
                    geometry::polyline(c, points, *closed);
                    if *closed {
                        self.set_fill(*color);
                        self.fill_path(true);
                    } else {
                        self.set_stroke(*color, 2.);
                        c.stroke();
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
                self.set_fill(*color);
                geometry::icon(c, *rect, icon);
                self.fill_path(true);
                if *outline > 0. {
                    self.set_stroke(*color, outline * (rect.right - rect.left) / icon.width);
                    geometry::icon(c, *rect, icon);
                    c.stroke();
                }
            }
            DrawCommand::Svg { rect, image, color } => self.svg(*rect, image, *color),
            DrawCommand::Image { rect, image } => self.image(*rect, image)?,
            DrawCommand::PushShapeClip { rect, radii } => {
                self.push(Kind::Plain);
                let (rect, radii) = normalized(*rect, *radii, None);
                geometry::rounded(c, rect, radii);
                c.clip();
            }
            DrawCommand::PushClip(rect) => {
                self.push(Kind::Plain);
                geometry::rectangle(c, *rect);
                c.clip();
            }
            DrawCommand::PushMask { bounds, gradient } => {
                self.push(Kind::Mask(Arc::clone(gradient), *bounds));
                c.push_group();
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
    /// Ouvre une portée d'effet. Un groupe n'est nécessaire que pour l'opacité d'un
    /// ensemble de plusieurs éléments ; il est borné au contenu, comme `contentBounds`
    /// sous Direct2D (sinon Cairo alloue et recompose toute la découpe).
    fn effect(&mut self, transform: Affine, opacity: f32, bounds: RectF, layer: bool) {
        let c = self.context;
        let outer = self.alpha;
        self.push(if layer {
            Kind::Layer(opacity * outer)
        } else {
            Kind::Plain
        });
        c.transform(Matrix::from_affine(transform));
        self.transform = transform.then(self.transform);
        if layer {
            geometry::rectangle(c, bounds);
            c.clip();
            c.push_group();
            self.alpha = 1.;
        } else {
            self.alpha = outer * opacity.clamp(0., 1.);
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
                layout.draw(self.context, rect, color, None, self.alpha);
                Ok(())
            }
        }
    }
    fn svg(&self, rect: RectF, image: &crate::ui::rendering::svg::SvgImage, color: Color) {
        use crate::ui::rendering::svg::{FillRule, LineCap, LineJoin};
        let c = self.context;
        c.save();
        c.transform(Matrix::from_affine(image.placement(rect)));
        for shape in image.shapes {
            if let Some(fill) = shape.fill.resolve(color, shape.fill_opacity) {
                self.set_fill(fill);
                geometry::svg(c, shape);
                self.fill_path(shape.fill_rule == FillRule::EvenOdd);
            }
            if let Some(stroke) = shape.stroke.resolve(color, shape.stroke_opacity)
                && shape.stroke_width > 0.
            {
                c.set_line_cap(match shape.line_cap {
                    LineCap::Butt => CAP_BUTT,
                    LineCap::Round => CAP_ROUND,
                    LineCap::Square => CAP_SQUARE,
                });
                c.set_line_join(match shape.line_join {
                    LineJoin::Miter => JOIN_MITER,
                    LineJoin::Round => JOIN_ROUND,
                    LineJoin::Bevel => JOIN_BEVEL,
                });
                c.set_miter_limit(shape.miter_limit.into());
                self.set_stroke(stroke, shape.stroke_width);
                geometry::svg(c, shape);
                c.stroke();
            }
        }
        c.restore();
    }
}
