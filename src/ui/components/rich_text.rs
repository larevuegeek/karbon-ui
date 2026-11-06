//! Paragraphe enrichi : portions stylées et liens activables dans un seul nœud de texte.
use super::*;
use crate::ui::{
    font::Font,
    scene::PointF,
    styling::text::{RichParagraph, SpanStyle, TextFormat, TextSpan},
};
use std::{cell::RefCell, rc::Rc};

/// One run of a [`RichText`]: a piece of text, its style and an optional link action.
pub struct TextRun {
    /// Text of the run.
    pub text: String,
    /// Style overrides of the run.
    pub style: SpanStyle,
    /// Message emitted when the run is activated; makes the run a link.
    pub link: Option<Action>,
}

/// Wrapping text made of styled runs (bold, italics, underline, strike-through, colour,
/// size, family) and links, laid out as one paragraph and exposed as one text node.
///
/// Links become focusable inline children of the element built by
/// [`element`](Self::element): they are reached with Tab, activated with a click,
/// Enter/Space or the accessibility *Invoke* command, and emit their message. Link runs
/// without an explicit colour use the theme accent.
///
/// The base size, font and colour follow the same rules as [`Text`]; computed CSS
/// typography applies to the whole paragraph.
///
/// ```
/// use karbon_ui::ui::{prelude::*, styling::text::SpanStyle};
///
/// #[derive(Clone)]
/// enum Msg {
///     Open,
/// }
/// let paragraph: Element = RichText::new()
///     .text("Hello ")
///     .styled("you", SpanStyle::new().bold())
///     .text(", read the ")
///     .link("guide", Msg::Open)
///     .element("intro");
/// assert_eq!(paragraph.children.len(), 1); // the link
/// ```
pub struct RichText {
    runs: Vec<TextRun>,
    /// Font size in logical units (15 by default).
    pub size: f32,
    /// Explicit font; `None` uses the system font.
    pub font: Option<Font>,
    /// Explicit colour, overriding the theme colour.
    pub color: Option<Color>,
    css: Option<Box<crate::ui::styling::ComputedStyle>>,
    value: Arc<str>,
    links: Vec<Rc<RefCell<Vec<RectF>>>>,
    role: Role,
}
impl Default for RichText {
    fn default() -> Self {
        Self::new()
    }
}
impl RichText {
    /// Empty paragraph with the default size (15) in the system font.
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            size: 15.,
            font: None,
            color: None,
            css: None,
            value: Arc::from(""),
            links: Vec::new(),
            role: Role::Text,
        }
    }
    /// Sets the accessibility role ([`Role::Text`] by default, [`Role::Heading`] for titles).
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
    /// Appends a run.
    pub fn run(mut self, run: TextRun) -> Self {
        if run.link.is_some() {
            self.links.push(Rc::default());
        }
        let mut value = String::with_capacity(self.value.len() + run.text.len());
        value.push_str(&self.value);
        value.push_str(&run.text);
        self.value = value.into();
        self.runs.push(run);
        self
    }
    /// Appends plain text.
    pub fn text(self, text: impl Into<String>) -> Self {
        self.styled(text, SpanStyle::new())
    }
    /// Appends text with `style`.
    pub fn styled(self, text: impl Into<String>, style: SpanStyle) -> Self {
        self.run(TextRun {
            text: text.into(),
            style,
            link: None,
        })
    }
    /// Appends an underlined link emitting a clone of `message` when activated.
    pub fn link<M: Clone + 'static>(self, text: impl Into<String>, message: M) -> Self {
        self.styled_link(text, SpanStyle::new().underline(), message)
    }
    /// Appends a link with an explicit `style`.
    pub fn styled_link<M: Clone + 'static>(
        self,
        text: impl Into<String>,
        style: SpanStyle,
        message: M,
    ) -> Self {
        self.run(TextRun {
            text: text.into(),
            style,
            link: Some(Action::new(message)),
        })
    }
    /// Sets the base font size in logical units.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Sets the base font.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }
    /// Sets the base font weight (100–900), starting from the system font if none is set.
    pub fn weight(self, weight: u16) -> Self {
        let mut font = self.font.unwrap_or(Font::system(false));
        font.weight = weight;
        self.font(font)
    }
    /// Sets the base colour, overriding the theme colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// Whole text, as read by assistive technologies.
    pub fn plain_text(&self) -> &str {
        &self.value
    }
    /// Runs of the paragraph, in order.
    pub fn runs(&self) -> &[TextRun] {
        &self.runs
    }
    /// Number of link runs.
    pub fn link_count(&self) -> usize {
        self.links.len()
    }
    /// Inline elements representing the links, with identities derived from `id`; the
    /// element holding this widget must own exactly these children, in this order.
    /// [`element`](Self::element) does it for you.
    pub fn link_elements(&self, id: ElementId) -> Vec<Element> {
        let mut geometry = self.links.iter();
        self.runs
            .iter()
            .filter_map(|run| run.link.clone().map(|action| (run, action)))
            .enumerate()
            .map(|(index, (run, action))| {
                Element::custom(
                    id.child("link").item(index as u64).0,
                    LinkTarget {
                        action,
                        name: Arc::from(run.text.as_str()),
                        fragments: Rc::clone(geometry.next().unwrap()),
                    },
                )
            })
            .collect()
    }
    /// Builds the element: a leaf holding the paragraph, plus one inline child per link.
    pub fn element(self, id: impl Into<ElementId>) -> Element {
        let id = id.into();
        let links = self.link_elements(id);
        let mut element = Element::custom(id.0, self);
        element.children = links;
        element
    }
    fn plain(&self) -> bool {
        self.runs.len() <= 1
            && self
                .runs
                .first()
                .is_none_or(|r| r.link.is_none() && r.style.is_plain())
    }
    fn format(&self) -> TextFormat {
        let font = self.font.unwrap_or(Font::system(false));
        let typography = self.css.as_ref().map(|c| c.text).unwrap_or_default();
        TextFormat::new(typography, font, self.size)
    }
    /// Paragraphe dont les liens sans couleur prennent `accent`. La casse
    /// (`text-transform`) est appliquée portion par portion : les plages restent alignées.
    fn paragraph(&self, accent: Color) -> RichParagraph {
        self.layout(accent).0
    }
    /// Paragraphe et plage d'octets de chaque portion dans son texte (transformé).
    fn layout(&self, accent: Color) -> (RichParagraph, Vec<std::ops::Range<usize>>) {
        use crate::ui::styling::TextTransform;
        let format = self.format();
        let transform = format.transform;
        let mut text = String::new();
        let mut spans = Vec::new();
        let mut ranges = Vec::with_capacity(self.runs.len());
        let mut at = 0;
        let mut word_start = true;
        for run in &self.runs {
            let end = match transform {
                TextTransform::None => at + run.text.len(),
                TextTransform::Capitalize if !word_start => {
                    // Le mot continue depuis la portion précédente : pas de capitale ici.
                    let split = run.text.find(char::is_whitespace).unwrap_or(run.text.len());
                    text.push_str(&run.text[..split]);
                    text.push_str(&transform.apply(&run.text[split..]));
                    text.len()
                }
                _ => {
                    text.push_str(&transform.apply(&run.text));
                    text.len()
                }
            };
            if let Some(c) = run.text.chars().last() {
                word_start = c.is_whitespace();
            }
            ranges.push(at..end);
            let mut style = run.style;
            if run.link.is_some() && style.color.is_none() {
                style.color = Some(accent);
            }
            if !style.is_plain() && end > at {
                spans.push(TextSpan {
                    range: at..end,
                    style,
                });
            }
            at = end;
        }
        let paragraph = RichParagraph {
            text: if transform == TextTransform::None {
                Arc::clone(&self.value)
            } else {
                text.into()
            },
            format,
            spans,
        };
        (paragraph, ranges)
    }
    fn validate_font(&self) -> crate::Result<()> {
        if self.font.is_some_and(|f| {
            !(1..=999).contains(&f.weight) || f.family.is_empty() || f.family.contains('\0')
        }) {
            return Err(crate::Error::invalid_input("invalid font family or weight"));
        }
        crate::ui::style::font_size(self.size)
    }
}
impl Widget for RichText {
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        crate::ui::styling::text::update(&mut self.css, style)
    }
    fn validate(&self) -> crate::Result<()> {
        self.validate_font()?;
        if self.plain() {
            return Ok(());
        }
        self.paragraph(Color::rgb(0, 0, 0)).validate()
    }
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        self.validate_font()?;
        if self.value.is_empty() {
            // Un texte vide (bloc conditionnel inactif) n'occupe aucune place.
            return Ok(SizeF::default());
        }
        let width = available.width.max(1.);
        let measured = if self.plain() {
            // Même chemin que `Text` : métriques identiques pour un texte simple.
            let format = self.format();
            crate::ui::styling::text::measure(text, &self.value, format, width)?
        } else {
            text.measure_rich(&self.paragraph(Color::rgb(0, 0, 0)), width)?
        };
        Ok(super::text::fit_width(measured, width))
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        let color = self
            .css
            .as_ref()
            .and_then(|c| c.text.color)
            .or(self.color)
            .unwrap_or(ctx.theme.text);
        let clip = !self.css.as_ref().is_some_and(|c| c.overflow_visible);
        if self.plain() {
            crate::ui::styling::text::draw(
                scene,
                ctx.rect,
                Arc::clone(&self.value),
                self.format().clip(clip),
                color,
                true,
            );
            return;
        }
        let mut paragraph = self.paragraph(ctx.theme.accent);
        paragraph.format = paragraph.format.clip(clip);
        let wrap = paragraph.format.wraps();
        scene.commands.push(DrawCommand::RichText {
            rect: ctx.rect,
            paragraph: Arc::new(paragraph),
            color,
            wrap,
        });
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: self.role,
            name: &self.value,
            enabled: true,
        }
    }
    fn inline_children(&self) -> usize {
        self.links.len()
    }
    fn inline_boxes(
        &mut self,
        area: RectF,
        text: &mut dyn TextMeasurer,
    ) -> crate::Result<Vec<RectF>> {
        if self.links.is_empty() {
            return Ok(Vec::new());
        }
        let (paragraph, ranges) = self.layout(Color::rgb(0, 0, 0));
        let mut boxes = Vec::with_capacity(self.links.len());
        let mut links = self.links.iter();
        for (run, range) in self.runs.iter().zip(ranges) {
            if run.link.is_none() {
                continue;
            }
            let fragments = text
                .rich_text_rects(&paragraph, width(area).max(1.), range)?
                .into_iter()
                .map(|r| {
                    RectF::new(
                        area.left + r.left,
                        area.top + r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                    )
                })
                .collect::<Vec<_>>();
            let union = fragments
                .iter()
                .copied()
                .reduce(|a, b| RectF {
                    left: a.left.min(b.left),
                    top: a.top.min(b.top),
                    right: a.right.max(b.right),
                    bottom: a.bottom.max(b.bottom),
                })
                .unwrap_or(RectF::new(area.left, area.top, 0., 0.));
            *links.next().unwrap().borrow_mut() = fragments;
            boxes.push(union);
        }
        Ok(boxes)
    }
}
impl crate::ui::template::typography::TextSize for RichText {
    fn text_size(self, size: f32) -> Self {
        self.size(size)
    }
}
impl crate::ui::template::typography::TextColor for RichText {
    fn text_color(self, color: Color) -> Self {
        self.color(color)
    }
}
impl crate::ui::template::typography::TextFont for RichText {
    fn text_font(self, font: Font) -> Self {
        self.font(font)
    }
}
impl crate::ui::template::typography::TextFamily for RichText {
    fn text_family(self, family: &'static str) -> Self {
        let weight = self.font.map_or(400, |f| f.weight);
        self.font(Font::new(family, weight))
    }
}
impl crate::ui::template::typography::TextWeight for RichText {
    fn text_weight(self, weight: u16) -> Self {
        self.weight(weight)
    }
}

/// Lien d'un texte enrichi : enfant en ligne focalisable dont les fragments
/// (une boîte par ligne) sont fournis par le paragraphe au layout.
struct LinkTarget {
    action: Action,
    name: Arc<str>,
    fragments: Rc<RefCell<Vec<RectF>>>,
}
impl Widget for LinkTarget {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF::default())
    }
    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        if ctx.state.focused {
            for r in self.fragments.borrow().iter() {
                scene.commands.push(DrawCommand::Stroke {
                    rect: RectF {
                        left: r.left - 2.,
                        top: r.top - 1.,
                        right: r.right + 2.,
                        bottom: r.bottom + 1.,
                    },
                    radius: 3.,
                    width: 2.,
                    color: ctx.theme.accent,
                });
            }
        }
    }
    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Link,
            name: &self.name,
            enabled: true,
        }
    }
    fn hit_test(&self, _: RectF, point: PointF) -> bool {
        let fragments = self.fragments.borrow();
        fragments.is_empty()
            || fragments
                .iter()
                .any(|r| crate::ui::layout::contains(*r, point.x, point.y))
    }
}
