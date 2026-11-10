pub(super) use super::super::super::fonts::Fonts;
use super::*;
pub(super) use crate::{
    platform::shared::{TextCache, Typesetter},
    ui::{element::SizeF, styling::text::RichParagraph},
};
/// Mises en page Core Text retenues tant qu'elles servent, et tailles mesurées.
pub(crate) type Cache = TextCache<Fonts>;
impl Typesetter for Fonts {
    type Layout = Layout;
    fn typeset(
        &mut self,
        paragraph: RichParagraph,
        width: f32,
        wrap: bool,
    ) -> crate::Result<Layout> {
        layout::build(self, paragraph, width, wrap)
    }
    fn size(layout: &Layout) -> SizeF {
        layout.size
    }
    fn left(layout: &Layout) -> f32 {
        layout.lines.iter().map(|l| l.x).fold(0f32, f32::min)
    }
}
