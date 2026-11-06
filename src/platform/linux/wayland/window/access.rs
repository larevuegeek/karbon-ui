//! La fenêtre vue par le pont AT-SPI (`linux/accessibility/`) : délégué à sa session.
use super::Window;
use crate::{
    platform::linux::accessibility::Host,
    ui::runtime::{AccessibilityAction, UiRuntime},
    window::WindowId,
};

impl Host for Window {
    fn id(&self) -> WindowId {
        self.session.id
    }
    fn title(&self) -> &str {
        self.session.title()
    }
    fn size(&self) -> (i32, i32) {
        self.session.size()
    }
    fn runtime(&self) -> &UiRuntime {
        &self.session.runtime
    }
    fn active(&self) -> bool {
        self.session.active()
    }
    fn act(&mut self, action: AccessibilityAction) -> crate::Result<bool> {
        self.session.act(action)
    }
}
