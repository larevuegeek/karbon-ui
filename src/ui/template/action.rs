use crate::{application::Action, ui::element::*};
/// Clickable layout area. The native widget remains responsible for its own input protocol.
pub(super) struct ActionArea {
    inner: Option<Box<dyn Widget>>,
    command: Action,
    /// `false` : bouton riche `disabled` (ni clic, ni focus, `:disabled` en CSS).
    enabled: bool,
}
impl ActionArea {
    pub fn new(inner: Option<Box<dyn Widget>>, command: Action, enabled: bool) -> Self {
        Self {
            inner,
            command,
            enabled,
        }
    }
}
impl Widget for ActionArea {
    fn apply_style(&mut self, style: &crate::ui::styling::ComputedStyle) -> bool {
        self.inner.as_mut().is_some_and(|w| w.apply_style(style))
    }
    fn validate(&self) -> crate::Result<()> {
        if self.inner.as_ref().is_some_and(|w| {
            w.text_input().is_some()
                || w.slider().is_some()
                || w.choice().is_some()
                || w.select().is_some()
                || w.modal().is_some()
                || w.popover().is_some()
        }) {
            return Err(crate::Error::invalid_input(
                "on_click cannot replace a stateful widget input protocol",
            ));
        }
        self.inner.as_ref().map_or(Ok(()), |w| w.validate())
    }
    fn measure(&self, available: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        self.inner
            .as_ref()
            .map_or(Ok(SizeF::default()), |w| w.measure(available, text))
    }
    fn paint(&self, c: &PaintContext, s: &mut crate::ui::scene::Scene) {
        if let Some(inner) = &self.inner {
            inner.paint(c, s);
        }
    }
    fn inline_children(&self) -> usize {
        self.inner.as_ref().map_or(0, |w| w.inline_children())
    }
    fn inline_boxes(
        &mut self,
        area: crate::ui::scene::RectF,
        text: &mut dyn TextMeasurer,
    ) -> crate::Result<Vec<crate::ui::scene::RectF>> {
        self.inner
            .as_mut()
            .map_or(Ok(Vec::new()), |w| w.inline_boxes(area, text))
    }
    fn action(&self) -> Option<Action> {
        self.semantics().enabled.then(|| self.command.clone())
    }
    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Button,
            name: self.inner.as_ref().map_or("Action", |w| w.semantics().name),
            enabled: self.enabled && self.inner.as_ref().is_none_or(|w| w.semantics().enabled),
        }
    }
}
