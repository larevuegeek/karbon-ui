//! Contexte de dessin Direct2D et ses caches ; traduction des commandes de la scène.
use super::*;

pub(super) struct Canvas {
    pub(super) shapes: shapes::Cache,
    pub(super) svgs: svg::Cache,
    pub(super) target: ID2D1DeviceContext,
    pub(super) brush: ID2D1SolidColorBrush,
    pub(super) bitmaps: HashMap<usize, (Arc<Image>, ID2D1Bitmap)>,
    pub(super) gradients: gradients::Cache,
}

impl Canvas {
    pub(super) fn new(target: ID2D1DeviceContext) -> crate::Result<Self> {
        // SAFETY: contexte Direct2D valide ; couleur lue pendant l'appel seulement.
        let brush = unsafe {
            target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.,
                    g: 1.,
                    b: 1.,
                    a: 1.,
                },
                None,
            )
        }
        .check("CreateSolidColorBrush")?;
        // Grayscale : ClearType ne convient pas aux surfaces à alpha transparent.
        // SAFETY: contexte Direct2D valide, hors passe de dessin.
        unsafe { target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE) };

        Ok(Self {
            target,
            brush,
            bitmaps: HashMap::new(),
            gradients: Default::default(),
            shapes: Default::default(),
            svgs: Default::default(),
        })
    }
    /// Images, formes, SVG et dégradés de `scene`, créés avant la passe de dessin.
    pub(super) fn prepare(&mut self, factory: &ID2D1Factory, scene: &Scene) -> crate::Result<()> {
        self.prepare_images(scene)?;
        self.shapes.prepare(factory, scene)?;
        self.svgs.prepare(factory, scene)?;
        self.gradients.prepare(&self.target, scene)
    }
    pub(super) fn color(&self, color: Color) {
        // SAFETY: pinceau valide ; couleur lue pendant l'appel seulement.
        unsafe { self.brush.SetColor(&color_f(color)) };
    }
    /// Texte simple (DrawText), partagé par `Text` et `FontText`.
    fn plain_text(
        &self,
        format: &IDWriteTextFormat,
        brush: &ID2D1Brush,
        rect: RectF,
        text: &str,
        wrap: bool,
    ) -> crate::Result<()> {
        // SAFETY: format DirectWrite valide, possédé par le cache.
        unsafe { format.SetWordWrapping(wrapping(wrap)) }.check("SetWordWrapping")?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: passe de dessin ouverte ; texte, format, rectangle et pinceau vivants.
        unsafe {
            self.target.DrawText(
                &utf16,
                format,
                &rect_f(rect),
                brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            )
        };
        Ok(())
    }
    pub(super) fn draw(
        &self,
        scene: &Scene,
        factory: &ID2D1Factory,
        formats: &HashMap<(u32, Font), IDWriteTextFormat>,
        typography: &typography::Cache,
        scopes: &mut Vec<effects::Scope>,
        filters: &filters::Cache,
    ) -> crate::Result<()> {
        let t = &self.target;
        let b = &self.brush;
        let mut filter_depth = 0usize;
        // Remplissages du texte actifs (`background-clip: text`).
        let mut text_fills: Vec<(crate::ui::rendering::brush::Background, RectF)> = Vec::new();
        let text_brush =
            |fills: &[(crate::ui::rendering::brush::Background, RectF)]| -> &ID2D1Brush {
                use crate::ui::rendering::brush::Background;
                match fills.last() {
                    Some((Background::Linear(g), r)) => self.gradients.brush(g, *r).into(),
                    Some((Background::Radial(g), r)) => self.gradients.radial(g, *r).into(),
                    Some((Background::Solid(c), _)) => {
                        self.color(*c);
                        b.into()
                    }
                    None => b.into(),
                }
            };
        for command in &scene.commands {
            match command {
                DrawCommand::PushFilter { id, .. } => {
                    if filter_depth == 0 {
                        filters.draw(&self.target, *id);
                    }
                    filter_depth += 1;
                    continue;
                }
                DrawCommand::PopFilter => {
                    filter_depth -= 1;
                    continue;
                }
                _ if filter_depth > 0 => continue,
                _ => {}
            }
            match command {
                DrawCommand::PushFilter { .. } | DrawCommand::PopFilter => {
                    unreachable!("scopes handled above")
                }
                DrawCommand::Shape {
                    rect,
                    radii,
                    fill,
                    stroke,
                } => {
                    use crate::ui::rendering::brush::Background;
                    let brush: &ID2D1Brush = match fill {
                        Some(Background::Linear(v)) => self.gradients.brush(v, *rect).into(),
                        Some(Background::Radial(v)) => self.gradients.radial(v, *rect).into(),
                        Some(Background::Solid(color)) => {
                            self.color(*color);
                            b.into()
                        }
                        None => {
                            self.color(stroke.map_or(Color::rgba(0, 0, 0, 0), |(_, c)| c));
                            b.into()
                        }
                    };
                    self.shapes.draw(t, brush, *rect, *radii, *stroke);
                }
                DrawCommand::RadialGradient {
                    rect,
                    radius,
                    gradient,
                } => {
                    let brush = self.gradients.radial(gradient, *rect);
                    // SAFETY: passe de dessin ouverte ; rectangle et pinceau vivants.
                    unsafe { t.FillRoundedRectangle(&rounded(*rect, *radius), brush) };
                }
                DrawCommand::FormattedText {
                    rect,
                    text,
                    format,
                    color,
                    wrap,
                } => {
                    self.color(*color);
                    typography.draw(t, text_brush(&text_fills), *rect, text, *format, *wrap);
                }
                DrawCommand::RichText {
                    rect,
                    paragraph,
                    color,
                    wrap,
                } => {
                    self.color(*color);
                    typography.draw_rich(t, text_brush(&text_fills), *rect, paragraph, *wrap)?;
                }
                DrawCommand::Gradient {
                    rect,
                    radius,
                    gradient,
                } => {
                    let brush = self.gradients.brush(gradient, *rect);
                    // SAFETY: passe de dessin ouverte ; rectangle et pinceau vivants.
                    unsafe { t.FillRoundedRectangle(&rounded(*rect, *radius), brush) };
                }
                DrawCommand::PushEffect {
                    transform,
                    opacity,
                    bounds,
                } => effects::push(t, scopes, *transform, *opacity, *bounds),
                DrawCommand::PopEffect => effects::pop(t, scopes.pop().expect("validated scene")),
                DrawCommand::Fill {
                    rect,
                    radius,
                    color,
                } => {
                    self.color(*color);
                    // SAFETY: passe de dessin ouverte ; rectangle et pinceau vivants.
                    unsafe { t.FillRoundedRectangle(&rounded(*rect, *radius), b) };
                }
                DrawCommand::Stroke {
                    rect,
                    radius,
                    width,
                    color,
                } => {
                    self.color(*color);
                    // SAFETY: passe de dessin ouverte ; rectangle et pinceau vivants, style par défaut.
                    unsafe { t.DrawRoundedRectangle(&rounded(*rect, *radius), b, *width, None) };
                }
                DrawCommand::Line {
                    start,
                    end,
                    width,
                    color,
                } => {
                    self.color(*color);
                    // SAFETY: passe de dessin ouverte ; pinceau vivant, style par défaut.
                    unsafe { t.DrawLine(vector(*start), vector(*end), b, *width, None) };
                }
                DrawCommand::Path {
                    points,
                    closed,
                    color,
                } => {
                    if points.len() < 2 {
                        continue;
                    }
                    self.color(*color);
                    let geometry = create_path(factory, points, *closed)?;
                    // SAFETY: passe de dessin ouverte ; géométrie fermée et pinceau vivants.
                    unsafe {
                        if *closed {
                            t.FillGeometry(&geometry, b, None);
                        } else {
                            t.DrawGeometry(&geometry, b, 2., None);
                        }
                    }
                }
                DrawCommand::Text {
                    rect,
                    text,
                    size,
                    bold,
                    color,
                    wrap,
                } => {
                    self.color(*color);
                    let format = &formats[&(size.to_bits(), Font::system(*bold))];
                    self.plain_text(format, text_brush(&text_fills), *rect, text, *wrap)?;
                }
                DrawCommand::Vector {
                    rect,
                    icon,
                    color,
                    outline,
                } => {
                    self.icon(factory, *rect, icon, *color, *outline)?;
                }
                DrawCommand::Svg { rect, image, color } => {
                    self.svgs.draw(t, b, *rect, image, *color);
                }
                DrawCommand::FontText {
                    rect,
                    text,
                    size,
                    font,
                    color,
                    wrap,
                } => {
                    self.color(*color);
                    let format = &formats[&(size.to_bits(), *font)];
                    self.plain_text(format, text_brush(&text_fills), *rect, text, *wrap)?;
                }
                DrawCommand::Image { rect, image } => {
                    self.image(*rect, image);
                }
                DrawCommand::PushShapeClip { rect, radii } => {
                    effects::mask(t, scopes, *rect, self.shapes.geometry(*rect, *radii));
                }
                DrawCommand::PopShapeClip => {
                    if let Some(scope) = scopes.pop() {
                        effects::pop(t, scope);
                    }
                }
                DrawCommand::PushClip(rect) => {
                    // SAFETY: passe de dessin ouverte ; dépilé par PopClip ou en fin de passe.
                    unsafe { t.PushAxisAlignedClip(&rect_f(*rect), D2D1_ANTIALIAS_MODE_ALIASED) };
                    scopes.push(effects::Scope::Clip);
                }
                DrawCommand::PopClip => {
                    // SAFETY: la scène validée garantit un PushClip correspondant.
                    unsafe { t.PopAxisAlignedClip() };
                    scopes.pop();
                }
                DrawCommand::PushMask { bounds, gradient } => {
                    let brush = self.gradients.brush(gradient, *bounds);
                    effects::opacity_mask(t, scopes, *bounds, brush.into());
                }
                DrawCommand::PopMask => {
                    if let Some(scope) = scopes.pop() {
                        effects::pop(t, scope);
                    }
                }
                DrawCommand::PushTextFill { rect, fill } => text_fills.push((fill.clone(), *rect)),
                DrawCommand::PopTextFill => {
                    text_fills.pop();
                }
                DrawCommand::Backdrop {
                    rect,
                    radii,
                    filters,
                } => self.backdrop(*rect, *radii, *filters)?,
            }
        }
        Ok(())
    }
}
