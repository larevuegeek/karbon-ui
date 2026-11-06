use super::*;
#[derive(Clone, Default)]
/// Colour overrides for a [`Select`](super::Select) and its list; `None` keeps
/// the theme colour.
pub struct SelectStyle {
    /// Background of the control and of the list.
    pub background: Option<Color>,
    /// Text colour.
    pub foreground: Option<Color>,
    /// Border colour.
    pub border: Option<Color>,
    /// Focus ring and focused border colour (theme accent by default).
    pub focus: Option<Color>,
    /// Background of the active row in the list (theme hover by default).
    pub highlight: Option<Color>,
}
