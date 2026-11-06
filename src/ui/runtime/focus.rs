use super::*;
impl UiRuntime {
    pub(super) fn radio_key(&mut self, key: Key) -> bool {
        let Some(next) = self.interaction.radio_target(self.root.as_ref(), key) else {
            return false;
        };
        self.interaction.focus_keyboard(next);
        self.activate(next);
        self.reveal_element(next);
        self.invalidation.repaint = true;
        true
    }

    pub(super) fn move_tab(&mut self, shift: bool) -> bool {
        let Some(next) = self.interaction.tab_target(self.root.as_ref(), shift) else {
            return false;
        };
        self.interaction.focus_keyboard(next);
        self.reveal_virtual(next);
        self.reveal_focus();
        self.invalidation.repaint = true;
        true
    }
}
