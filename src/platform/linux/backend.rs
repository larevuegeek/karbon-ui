use super::*;
/// Implémentation de la frontière [`Backend`](super::Backend) pour Linux.
pub(crate) struct Linux;
impl super::super::Backend for Linux {
    type Offscreen = offscreen::NativeOffscreen;
    fn run_windows(
        windows: Vec<crate::window::WindowDescriptor>,
        exit_policy: crate::window::ExitPolicy,
    ) -> crate::Result<()> {
        // Wayland s'il y a un compositeur, sinon X11 ; `KARBON_BACKEND` impose l'un ou
        // l'autre (X11 sous XWayland, par exemple).
        let backend = std::env::var("KARBON_BACKEND").ok();
        let x11 = match backend.as_deref() {
            Some("x11") => true,
            Some("wayland") => false,
            _ => {
                std::env::var_os("WAYLAND_DISPLAY").is_none()
                    && std::env::var_os("DISPLAY").is_some()
            }
        };
        if x11 {
            x11::run_windows(windows, exit_policy)
        } else {
            wayland::run_windows(windows, exit_policy)
        }
    }
    fn offscreen(renderer: crate::window::RenderMode) -> crate::Result<Self::Offscreen> {
        offscreen::NativeOffscreen::new(renderer)
    }
    fn process_sample() -> crate::Result<crate::diagnostics::ProcessSample> {
        offscreen::sample()
    }
    fn write_png(image: &crate::ui::scene::Image, path: &std::path::Path) -> crate::Result<()> {
        imaging::write_png(image, path)
    }
}
