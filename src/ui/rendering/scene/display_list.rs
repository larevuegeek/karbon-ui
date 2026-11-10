use super::*;
pub(super) use crate::ui::theme::Color;
pub(super) use std::sync::Arc;
/// Ordered list of drawing commands for one frame, consumed by the platform backend.
#[derive(Clone)]
pub struct Scene {
    /// Background colour the surface is cleared to before drawing.
    pub clear: Color,
    /// Commands drawn in order (painter's algorithm).
    pub commands: Vec<DrawCommand>,
}
impl Scene {
    /// Append the commands of `other`, offset by (`dx`, `dy`); its clear colour is ignored.
    pub fn append_translated(&mut self, other: Scene, dx: f32, dy: f32) {
        let rect = |r: &mut RectF| {
            r.left += dx;
            r.right += dx;
            r.top += dy;
            r.bottom += dy;
        };
        for mut command in other.commands {
            match &mut command {
                DrawCommand::PushEffect {
                    transform, bounds, ..
                } => {
                    rect(bounds);
                    *transform = crate::ui::rendering::transform::Affine::translation(-dx, -dy)
                        .then(*transform)
                        .then(crate::ui::rendering::transform::Affine::translation(dx, dy));
                }
                DrawCommand::PushFilter { bounds, .. }
                | DrawCommand::PushMask { bounds, .. }
                | DrawCommand::PushTextFill { rect: bounds, .. }
                | DrawCommand::Backdrop { rect: bounds, .. } => rect(bounds),
                DrawCommand::Vector { rect: r, .. }
                | DrawCommand::Svg { rect: r, .. }
                | DrawCommand::PushShapeClip { rect: r, .. }
                | DrawCommand::Shape { rect: r, .. }
                | DrawCommand::Gradient { rect: r, .. }
                | DrawCommand::RadialGradient { rect: r, .. }
                | DrawCommand::FontText { rect: r, .. }
                | DrawCommand::FormattedText { rect: r, .. }
                | DrawCommand::RichText { rect: r, .. }
                | DrawCommand::Fill { rect: r, .. }
                | DrawCommand::Stroke { rect: r, .. }
                | DrawCommand::Text { rect: r, .. }
                | DrawCommand::Image { rect: r, .. }
                | DrawCommand::PushClip(r) => rect(r),
                DrawCommand::Line { start, end, .. } => {
                    start.x += dx;
                    start.y += dy;
                    end.x += dx;
                    end.y += dy;
                }
                DrawCommand::Path { points, .. } => {
                    for p in points {
                        p.x += dx;
                        p.y += dy;
                    }
                }
                DrawCommand::PopClip
                | DrawCommand::PopEffect
                | DrawCommand::PopFilter
                | DrawCommand::PopShapeClip
                | DrawCommand::PopMask
                | DrawCommand::PopTextFill => (),
            }
            self.commands.push(command);
        }
    }
    /// Empty scene cleared to `clear`.
    pub fn new(clear: Color) -> Self {
        Self {
            clear,
            commands: Vec::new(),
        }
    }
    /// Push single-line text drawn with an explicit font.
    pub fn font_text(
        &mut self,
        rect: RectF,
        text: impl Into<Arc<str>>,
        size: f32,
        font: crate::ui::font::Font,
        color: Color,
    ) {
        self.commands.push(DrawCommand::FontText {
            rect,
            text: text.into(),
            size,
            font,
            color,
            wrap: false,
        });
    }
    /// Check the scene before rendering: balanced push/pop scopes (nesting depth at most
    /// 256), finite non-negative geometry, valid brushes, filters and effects.
    ///
    /// Returns an [`Error::InvalidInput`](crate::Error::InvalidInput) error describing the first invalid command.
    pub fn validate(&self) -> crate::Result<()> {
        let mut stack = Vec::new();
        let mut filter_ids = std::collections::HashSet::new();
        for command in &self.commands {
            match command {
                DrawCommand::PushShapeClip { rect, radii } => {
                    if ![rect.left, rect.top, rect.right, rect.bottom]
                        .into_iter()
                        .chain(radii.iter().copied())
                        .all(f32::is_finite)
                        || radii.iter().any(|r| *r < 0.)
                        || rect.right < rect.left
                        || rect.bottom < rect.top
                    {
                        return Err(crate::Error::invalid_input("invalid shape clip"));
                    }
                    stack.push(3);
                }
                DrawCommand::PopShapeClip if stack.pop() != Some(3) => {
                    return Err(crate::Error::invalid_input("unbalanced shape clip"));
                }
                DrawCommand::PushFilter {
                    id,
                    bounds,
                    filters,
                } => {
                    if !filters.valid()
                        || !filter_ids.insert(*id)
                        || filter_ids.len() > 32
                        || stack.iter().filter(|s| **s == 2).count() >= 8
                        || ![bounds.left, bounds.top, bounds.right, bounds.bottom]
                            .iter()
                            .all(|n| n.is_finite())
                        || bounds.right < bounds.left
                        || bounds.bottom < bounds.top
                    {
                        return Err(crate::Error::invalid_input(
                            "invalid, duplicate or excessive filter scopes",
                        ));
                    }
                    stack.push(2);
                }
                DrawCommand::PopFilter if stack.pop() != Some(2) => {
                    return Err(crate::Error::invalid_input("unbalanced filter scope"));
                }
                DrawCommand::Shape {
                    rect,
                    radii,
                    fill,
                    stroke,
                } => {
                    if fill.is_some() && stroke.is_some() {
                        return Err(crate::Error::invalid_input(
                            "a shape command cannot both fill and stroke; use separate commands",
                        ));
                    }
                    if ![rect.left, rect.top, rect.right, rect.bottom]
                        .into_iter()
                        .chain(radii.iter().copied())
                        .all(f32::is_finite)
                        || radii.iter().any(|n| *n < 0.)
                        || stroke.is_some_and(|(n, _)| !n.is_finite() || n < 0.)
                        || rect.right < rect.left
                        || rect.bottom < rect.top
                    {
                        return Err(crate::Error::invalid_input("invalid rounded shape"));
                    }
                    if let Some(fill) = fill {
                        fill.validate()?;
                    }
                }
                DrawCommand::RadialGradient {
                    rect,
                    radius,
                    gradient,
                } => {
                    gradient.validate()?;
                    if ![rect.left, rect.right, rect.top, rect.bottom, *radius]
                        .iter()
                        .all(|n| n.is_finite())
                        || *radius < 0.
                        || rect.right < rect.left
                        || rect.bottom < rect.top
                    {
                        return Err(crate::Error::invalid_input(
                            "invalid radial gradient bounds",
                        ));
                    }
                }
                DrawCommand::FormattedText { format, .. } => format.validate()?,
                DrawCommand::RichText { paragraph, .. } => paragraph.validate()?,
                DrawCommand::Gradient {
                    rect,
                    radius,
                    gradient,
                } => {
                    gradient.validate()?;
                    if ![rect.left, rect.top, rect.right, rect.bottom, *radius]
                        .iter()
                        .all(|n| n.is_finite())
                        || *radius < 0.
                        || rect.right < rect.left
                        || rect.bottom < rect.top
                    {
                        return Err(crate::Error::invalid_input(
                            "invalid gradient bounds or radius",
                        ));
                    }
                }
                DrawCommand::PushClip(_) => stack.push(0),
                DrawCommand::Svg { rect, image, .. } => {
                    image.validate()?;
                    if ![rect.left, rect.top, rect.right, rect.bottom]
                        .iter()
                        .all(|n| n.is_finite())
                        || rect.right < rect.left
                        || rect.bottom < rect.top
                    {
                        return Err(crate::Error::invalid_input("invalid SVG bounds"));
                    }
                }
                DrawCommand::PushMask { bounds, gradient } => {
                    gradient.validate()?;
                    if ![bounds.left, bounds.top, bounds.right, bounds.bottom]
                        .iter()
                        .all(|n| n.is_finite())
                    {
                        return Err(crate::Error::invalid_input("invalid mask bounds"));
                    }
                    stack.push(4);
                }
                DrawCommand::PopMask if stack.pop() != Some(4) => {
                    return Err(crate::Error::invalid_input("unbalanced mask scope"));
                }
                DrawCommand::PushTextFill { rect, fill } => {
                    fill.validate()?;
                    if ![rect.left, rect.top, rect.right, rect.bottom]
                        .iter()
                        .all(|n| n.is_finite())
                    {
                        return Err(crate::Error::invalid_input("invalid text fill bounds"));
                    }
                    stack.push(5);
                }
                DrawCommand::PopTextFill if stack.pop() != Some(5) => {
                    return Err(crate::Error::invalid_input("unbalanced text fill scope"));
                }
                DrawCommand::Backdrop {
                    rect,
                    radii,
                    filters,
                } => {
                    if !filters.valid()
                        || ![rect.left, rect.top, rect.right, rect.bottom]
                            .into_iter()
                            .chain(radii.iter().copied())
                            .all(f32::is_finite)
                        || radii.iter().any(|r| *r < 0.)
                    {
                        return Err(crate::Error::invalid_input("invalid backdrop filter"));
                    }
                }
                DrawCommand::PushEffect {
                    transform,
                    opacity,
                    bounds,
                } => {
                    if !transform.is_finite()
                        || !opacity.is_finite()
                        || !(0. ..=1.).contains(opacity)
                        || ![bounds.left, bounds.top, bounds.right, bounds.bottom]
                            .iter()
                            .all(|v| v.is_finite())
                        || bounds.right < bounds.left
                        || bounds.bottom < bounds.top
                    {
                        return Err(crate::Error::invalid_input("invalid graphics effect"));
                    }
                    stack.push(1);
                }
                DrawCommand::PopClip if stack.pop() != Some(0) => {
                    return Err(crate::Error::invalid_input(
                        "graphics stack: PopClip without a matching PushClip",
                    ));
                }
                DrawCommand::PopEffect if stack.pop() != Some(1) => {
                    return Err(crate::Error::invalid_input(
                        "graphics stack: PopEffect without a matching PushEffect",
                    ));
                }
                _ => (),
            }
            if stack.len() > 256 {
                return Err(crate::Error::invalid_input("graphics stack too deep"));
            }
        }
        if !stack.is_empty() {
            return Err(crate::Error::invalid_input("unclosed clip"));
        }
        Ok(())
    }
    /// Push a solid rounded-rectangle fill.
    pub fn fill(&mut self, rect: RectF, radius: f32, color: Color) {
        self.commands.push(DrawCommand::Fill {
            rect,
            radius,
            color,
        });
    }
    /// Push wrapped text in the default system font.
    pub fn text(
        &mut self,
        rect: RectF,
        text: impl Into<Arc<str>>,
        size: f32,
        bold: bool,
        color: Color,
    ) {
        self.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            bold,
            color,
            wrap: true,
        });
    }
    /// Push unwrapped text in the default system font.
    pub fn text_single_line(
        &mut self,
        rect: RectF,
        text: impl Into<Arc<str>>,
        size: f32,
        bold: bool,
        color: Color,
    ) {
        self.commands.push(DrawCommand::Text {
            rect,
            text: text.into(),
            size,
            bold,
            color,
            wrap: false,
        });
    }
}
