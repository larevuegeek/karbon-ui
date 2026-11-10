use super::*;
// Surface layered utilisée par les tests de `graphics` (chemin `windows::surface`).
#[cfg(test)]
pub(super) use super::present::surface;

/// Implémentation de la frontière [`Backend`](super::Backend) pour Windows.
pub(crate) struct Windows;
impl super::super::Backend for Windows {
    type Offscreen = offscreen::NativeOffscreen;
    fn run_windows(
        windows: Vec<crate::window::WindowDescriptor>,
        exit_policy: crate::window::ExitPolicy,
    ) -> crate::Result<()> {
        window::run_windows(windows, exit_policy)
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
