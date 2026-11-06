use super::*;
/// Appearance of a [`Tooltip`]; colours set to `None` keep the theme colour.
#[derive(Clone, PartialEq)]
pub struct TooltipStyle {
    /// Background colour.
    pub background: Option<Color>,
    /// Text colour.
    pub foreground: Option<Color>,
    /// Border colour.
    pub border: Option<Color>,
    /// Font size in logical units, 8 to 96 (12 by default).
    pub font_size: f32,
}
impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            background: None,
            foreground: None,
            border: None,
            font_size: 12.,
        }
    }
}
/// Delayed help text shown near an element on hover or keyboard focus.
///
/// Limits checked when the view is validated: non-blank text of at most 16 KiB,
/// a delay of at most 60 s and a maximum width between 40 and 4096.
#[derive(Clone, PartialEq)]
pub struct Tooltip {
    /// Help text.
    pub text: Arc<str>,
    /// Hover delay before showing (500 ms by default).
    pub delay: Duration,
    /// Maximum width in logical units before wrapping (320 by default).
    pub max_width: f32,
    /// Appearance.
    pub style: TooltipStyle,
}
impl Tooltip {
    /// Tooltip with the default delay, width and style.
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        Self {
            text: text.into(),
            delay: Duration::from_millis(500),
            max_width: 320.,
            style: TooltipStyle::default(),
        }
    }
    /// Set the delay before showing (at most 60 s).
    pub fn delay(mut self, value: Duration) -> Self {
        self.delay = value;
        self
    }
    /// Set the maximum width in logical units (40 to 4096).
    pub fn max_width(mut self, value: f32) -> Self {
        self.max_width = value;
        self
    }
    /// Set the appearance.
    pub fn style(mut self, value: TooltipStyle) -> Self {
        self.style = value;
        self
    }
    pub(crate) fn validate(&self) -> crate::Result<()> {
        if self.text.trim().is_empty()
            || self.text.len() > 16384
            || self.delay > Duration::from_secs(60)
            || !self.max_width.is_finite()
            || !(40. ..=4096.).contains(&self.max_width)
            || !self.style.font_size.is_finite()
            || !(8. ..=96.).contains(&self.style.font_size)
        {
            return Err(crate::Error::invalid_input("invalid tooltip configuration"));
        }
        Ok(())
    }
}
