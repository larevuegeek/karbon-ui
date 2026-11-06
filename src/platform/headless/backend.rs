//! Backend headless : services de plateforme sans fenêtre et moteur hors écran (métriques
//! fixes, substituts d'images, rastériseur logiciel).
use super::{HeadlessTextMeasurer, png, raster, resources};
use crate::ui::{
    element::{SizeF, TextMeasurer},
    font::{Font, FontAsset},
    scene::Scene,
    styling::text::TextFormat,
    text_input::TextLine,
};

/// Libellé du moteur headless, transmis à `ViewContext::backend`.
pub(super) const LABEL: &str = "headless";

/// Backend sans fenêtre : sélectionné comme `Native` hors Windows, macOS et Linux, et
/// utilisé par `HeadlessApp` sur toutes les cibles.
pub(crate) struct Headless;
impl crate::platform::Backend for Headless {
    type Offscreen = HeadlessRenderer;
    fn run_windows(
        _windows: Vec<crate::window::WindowDescriptor>,
        _exit_policy: crate::window::ExitPolicy,
    ) -> crate::Result<()> {
        Err(crate::Error::Unsupported {
            reason: "no interactive backend for this target (Windows, macOS and Linux have one); \
                     use platform::headless::HeadlessApp to run applications without a window",
        })
    }
    fn offscreen(_renderer: crate::window::RenderMode) -> crate::Result<Self::Offscreen> {
        Ok(HeadlessRenderer::default())
    }
    fn process_sample() -> crate::Result<crate::diagnostics::ProcessSample> {
        // Aucun compteur portable sans dépendance : les mesures restent à zéro.
        Ok(crate::diagnostics::ProcessSample {
            working_set_bytes: 0,
            private_bytes: 0,
            peak_working_set_bytes: 0,
            cpu_100ns: 0,
        })
    }
    fn write_png(image: &crate::ui::scene::Image, path: &std::path::Path) -> crate::Result<()> {
        png::write(image, path)
    }
}

/// Moteur hors écran headless : métriques fixes, substituts d'images et rastériseur.
#[derive(Default)]
pub(crate) struct HeadlessRenderer {
    text: HeadlessTextMeasurer,
    resources: resources::HeadlessResources,
}
impl TextMeasurer for HeadlessRenderer {
    fn measure_rich(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
    ) -> crate::Result<SizeF> {
        self.text.measure_rich(paragraph, width)
    }
    fn rich_text_rects(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<crate::ui::scene::RectF>> {
        self.text.rich_text_rects(paragraph, width, range)
    }
    fn measure_formatted(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        self.text.measure_formatted(value, format, width)
    }
    fn layout_line(&mut self, value: &str, size: f32) -> crate::Result<TextLine> {
        self.text.layout_line(value, size)
    }
    fn layout_font_line(&mut self, value: &str, size: f32, font: Font) -> crate::Result<TextLine> {
        self.text.layout_font_line(value, size, font)
    }
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: TextFormat,
    ) -> crate::Result<TextLine> {
        self.text.layout_formatted_line(value, format)
    }
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<crate::ui::text_input::WrappedLine>> {
        self.text.layout_wrapped_text(value, format, width)
    }
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: Font,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.text.measure_font(text, size, font, max_width)
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.text.measure_text(text, size, bold, max_width)
    }
}
impl crate::platform::Offscreen for HeadlessRenderer {
    fn label(&self) -> &'static str {
        LABEL
    }
    fn resources(&self) -> &dyn crate::application::Resources {
        &self.resources
    }
    fn load_fonts(&mut self, fonts: &[FontAsset]) -> crate::Result<()> {
        // Les métriques ne dépendent pas des polices : seules les ressources sont validées.
        if fonts.iter().any(|font| {
            font.family.is_empty()
                || font.family.contains('\0')
                || font.data.is_empty()
                || font.data.len() > 16 * 1024 * 1024
        }) {
            return Err(crate::Error::invalid_input("invalid font asset"));
        }
        Ok(())
    }
    fn render(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(crate::Error::invalid_input("invalid render size"));
        }
        if !(scale.is_finite() && scale > 0.) {
            return Err(crate::Error::invalid_input("invalid render scale"));
        }
        Ok(raster::rasterize(scene, width, height, scale))
    }
}
