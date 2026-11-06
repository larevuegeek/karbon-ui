use super::*;
impl UiRuntime {
    /// Text, selection and geometry of the text input `id`, if it exists.
    pub fn input_snapshot(&self, id: ElementId) -> Option<InputSnapshot> {
        let input = self
            .root
            .as_ref()?
            .find(id)?
            .widget
            .as_ref()?
            .text_input()?;
        self.components.inputs.snapshot(id, input)
    }

    /// Snapshot of the focused text input, if focus is on one.
    pub fn focused_input(&self) -> Option<InputSnapshot> {
        self.input_snapshot(self.interaction.focused?)
    }

    /// Rangées du menu contextuel intégré d'un champ, s'il est ouvert : rectangle et
    /// disponibilité de chaque entrée (couper, copier, coller, tout sélectionner).
    #[cfg(test)]
    pub(crate) fn edit_menu_rows(&self) -> Vec<(RectF, bool)> {
        self.components.edit_menu.rows()
    }

    /// Caret and visible-area geometry of the focused text input, without copying its text
    /// (used to position the IME window).
    pub fn focused_input_geometry(&self) -> Option<crate::ui::text_input::InputGeometry> {
        let id = self.interaction.focused?;
        let input = self
            .root
            .as_ref()?
            .find(id)?
            .widget
            .as_ref()?
            .text_input()?;
        self.components.inputs.geometry(id, input)
    }
}
