use super::*;
impl UiRuntime {
    /// Earliest instant at which the runtime needs a [`UiEvent::Timer`] (tooltip delay or
    /// animation frame), or `None` if nothing is pending.
    pub fn next_wake(&self) -> Option<Instant> {
        [
            self.components.tooltips.next_wake(),
            self.motions.next_wake(),
            self.interaction.hover_frozen,
            self.smooth_scroll_wake(),
            self.invalidation.graveyard_wake,
            self.frame_work_wake(),
            self.held_rebuild_wake(),
            self.probe_wake(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Tooltip currently shown, if any.
    pub fn tooltip(&self) -> Option<TooltipInfo> {
        self.components.tooltips.snapshot()
    }

    pub(super) fn tooltip_input_mode(&mut self, keyboard: bool) {
        if self.interaction.tooltip_keyboard != keyboard {
            self.interaction.tooltip_keyboard = keyboard;
            if self.components.tooltips.reset() {
                self.invalidation.repaint_semantics();
            }
        }
    }

    pub(super) fn sync_tooltip(&mut self) -> bool {
        let blocked = !self.interaction.window_active
            || self.has_capture()
            || self.components.selects.is_open()
            || self.components.edit_menu.is_open();
        let viewport = self.layer_viewport();
        let (pointer, focus) = self.interaction.tooltip_anchor();
        let changed = self.root.as_ref().is_some_and(|r| {
            self.components.tooltips.sync(
                layers::active(r),
                pointer,
                focus,
                viewport,
                blocked,
                Instant::now(),
            )
        });
        if changed {
            self.invalidation.repaint_semantics();
        }
        changed
    }
}
