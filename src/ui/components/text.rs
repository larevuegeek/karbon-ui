use super::*;
use crate::ui::{font::Font, style};
/// Rounds a measured text width up to the next whole logical pixel, so laying the text
/// out again at the width the layout hands back never wraps it: positions lose bits in
/// `f32` (`right - left` can be a hair below the measured width) and DirectWrite breaks
/// a line, even inside a word, as soon as it is wider than the box. The width never
/// exceeds what was available (the text was measured, and wrapped, at that width).
pub(crate) fn fit_width(measured: SizeF, available: f32) -> SizeF {
    if measured.width <= 0. {
        return measured;
    }
    SizeF {
        width: (measured.width + 0.01)
            .ceil()
            .min(available.max(measured.width)),
        height: measured.height,
    }
}
/// Wrapping text label. Colour precedence: [`Text::color`], then the
/// [`TextStyle`] colour, then the theme text colour.
pub struct Text {
    /// Displayed text, also used as the accessible name.
    pub value: Arc<str>,
    /// Font size in logical units (15 by default).
    pub size: f32,
    /// Whether the system font is drawn bold (ignored when `font` sets a weight).
    pub bold: bool,
    /// Explicit font; `None` uses the system font.
    pub font: Option<Font>,
    /// Explicit colour, overriding the style and theme colours.
    pub color: Option<Color>,
    /// Optional typography recipe.
    pub style: Option<Arc<TextStyle>>,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
}
impl Text {
    /// Text with the default size (15) in the system font and theme colour.
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self {
            value: value.into(),
            size: 15.,
            bold: false,
            font: None,
            color: None,
            style: None,
            css: None,
        }
    }
    /// Set the font size in logical units.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Draw the text bold (weight 700 when an explicit font is set).
    pub fn bold(mut self) -> Self {
        self.bold = true;
        if let Some(font) = &mut self.font {
            font.weight = 700;
        }
        self
    }
    /// Use an explicit font; weights of 600 and above also mark the text bold.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        let font = font.into();
        self.bold = font.weight >= 600;
        self.font = Some(font);
        self
    }
    /// Set the font weight (100–900), starting from the system font if none is set.
    pub fn weight(self, weight: u16) -> Self {
        let mut font = self.font.unwrap_or(Font::system(self.bold));
        font.weight = weight;
        self.font(font)
    }
    /// Set an explicit colour, overriding the style and theme colours.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// Apply a typography recipe: copies its size and weight and clears any
    /// explicit colour so the style colour applies.
    pub fn style(mut self, style: impl Into<Arc<TextStyle>>) -> Self {
        let style = style.into();
        self.size = style.font_size;
        self.bold = style.bold;
        if let Some(font) = &mut self.font {
            font.weight = if style.bold { 700 } else { 400 };
        }
        self.color = None;
        self.style = Some(style);
        self
    }
}
impl Widget for Text {
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        crate::ui::styling::text::update(&mut self.css, style)
    }
    fn validate(&self) -> crate::Result<()> {
        if self.font.is_some_and(|f| {
            !(1..=999).contains(&f.weight) || f.family.is_empty() || f.family.contains('\0')
        }) {
            return Err(crate::Error::invalid_input("invalid font family or weight"));
        }
        style::font_size(self.size)
    }
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        self.validate()?;
        let width = available.width.max(1.);
        let measured = if let Some(css) = &self.css {
            let format = crate::ui::styling::text::TextFormat::new(
                css.text,
                self.font.unwrap_or(Font::system(self.bold)),
                self.size,
            );
            crate::ui::styling::text::measure(text, &self.value, format, width)?
        } else if let Some(font) = self.font {
            text.measure_font(&self.value, self.size, font, width)?
        } else {
            text.measure_text(&self.value, self.size, self.bold, width)?
        };
        Ok(fit_width(measured, width))
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        let color = self
            .color
            .or_else(|| {
                self.style
                    .as_ref()
                    .and_then(|s| s.color)
                    .map(|c| c.resolve(ctx.theme))
            })
            .unwrap_or(ctx.theme.text);
        if let Some(css) = &self.css {
            crate::ui::styling::text::draw(
                scene,
                ctx.rect,
                Arc::clone(&self.value),
                crate::ui::styling::text::TextFormat::new(
                    css.text,
                    self.font.unwrap_or(Font::system(self.bold)),
                    self.size,
                )
                .clip(!css.overflow_visible),
                css.text.color.unwrap_or(color),
                true,
            );
            return;
        }
        if let Some(font) = self.font {
            scene.commands.push(DrawCommand::FontText {
                rect: ctx.rect,
                text: Arc::clone(&self.value),
                size: self.size,
                font,
                color,
                wrap: true,
            });
            return;
        }
        scene.text(
            ctx.rect,
            Arc::clone(&self.value),
            self.size,
            self.bold,
            color,
        );
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Text,
            name: &self.value,
            enabled: true,
        }
    }
}
