//! Le stylet et la place de la fenêtre sur l'écran, simulés : ce qu'une fenêtre dit en
//! plus de la place du pointeur (`PointerEvent::pressure`, `tilt`, `screen`).
use super::HeadlessApp;
use crate::{
    application::Application,
    ui::{
        runtime::{PointerButton, PointerPhase},
        scene::PointF,
    },
};

/// A pen on a tablet, for [`HeadlessApp::raw_pen`]: its pressure from `0.0` to `1.0` and
/// its tilt in degrees along x and y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pen {
    /// From `0.0` (barely touching) to `1.0`.
    pub pressure: f32,
    /// Degrees from the vertical, along x and y.
    pub tilt: (f32, f32),
}

impl<A: Application> HeadlessApp<A> {
    /// Like [`raw_pointer`](Self::raw_pointer), the input coming from `pen` instead of a
    /// mouse: the handler reads its pressure and tilt.
    ///
    /// # Errors
    ///
    /// The errors of [`prepare`](Self::prepare).
    pub fn raw_pen(
        &mut self,
        phase: PointerPhase,
        button: Option<PointerButton>,
        at: PointF,
        pen: Pen,
    ) -> crate::Result<()> {
        self.ui.set_pointer_extras(
            Some((pen.pressure, pen.tilt)),
            self.screen_origin,
            self.screen_ratio,
        );
        let none = super::RawModifiers::default();
        let done = self.raw_pointer(phase, button, at, none);
        let (origin, ratio) = (self.screen_origin, self.screen_ratio);
        self.ui.set_pointer_extras(None, origin, ratio);
        done
    }

    /// Put the window at `origin` on the screen (in the units of
    /// [`WindowChange::Position`](crate::window::WindowChange::Position)), its logical
    /// pixels being `ratio` of these units (the scale of its screen over the scale of the
    /// main one): the raw pointer inputs that follow carry their place on the screen in
    /// [`PointerEvent::screen`](crate::ui::runtime::PointerEvent::screen). The window is
    /// at the origin, with a ratio of 1, until then.
    pub fn place_on_screen(&mut self, origin: (f32, f32), ratio: f32) {
        self.screen_origin = origin;
        self.screen_ratio = ratio;
        self.ui.set_pointer_extras(None, origin, ratio);
    }
}
