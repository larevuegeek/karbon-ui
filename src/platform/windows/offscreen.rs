//! Services hors écran du backend Windows : moteur de rendu et compteurs du processus.
use super::{ffi::ComApartment, graphics::Renderer, imaging::DrawingEngine};
use crate::{
    diagnostics::ProcessSample,
    platform::Offscreen,
    ui::{
        element::{SizeF, TextMeasurer},
        font::{Font, FontAsset},
        scene::Scene,
        styling::text::TextFormat,
        text_input::TextLine,
    },
};
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX},
        Threading::{GetCurrentProcess, GetProcessTimes},
    },
};

/// Durée FILETIME en unités de 100 ns.
fn ticks(time: FILETIME) -> u64 {
    u64::from(time.dwLowDateTime) | (u64::from(time.dwHighDateTime) << 32)
}
pub(super) fn sample() -> crate::Result<ProcessSample> {
    // SAFETY : pseudo-handle du processus courant, sans fermeture requise.
    let process = unsafe { GetCurrentProcess() };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let mut memory = PROCESS_MEMORY_COUNTERS_EX {
        cb: size,
        ..Default::default()
    };
    // SAFETY : la version étendue commence par PROCESS_MEMORY_COUNTERS ; `size` annonce
    // à Windows la structure complète, valide en écriture.
    if unsafe { GetProcessMemoryInfo(process, (&raw mut memory).cast(), size) } == 0 {
        return Err(super::ffi::last_error("GetProcessMemoryInfo"));
    }
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY : quatre structures locales valides en écriture.
    if unsafe { GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) } == 0 {
        return Err(super::ffi::last_error("GetProcessTimes"));
    }
    Ok(ProcessSample {
        working_set_bytes: memory.WorkingSetSize,
        private_bytes: memory.PrivateUsage,
        peak_working_set_bytes: memory.PeakWorkingSetSize,
        cpu_100ns: ticks(kernel).saturating_add(ticks(user)),
    })
}
/// Moteur hors écran Windows : Direct2D/DirectWrite pour le texte et les pixels, GDI+
/// pour les images. Ordre de destruction : renderer, GDI+, puis l'appartement COM.
pub(crate) struct NativeOffscreen {
    renderer: Renderer,
    engine: DrawingEngine,
    _apartment: ComApartment,
}
impl NativeOffscreen {
    pub(super) fn new(mode: crate::window::RenderMode) -> crate::Result<Self> {
        let apartment = ComApartment::initialize()?;
        let engine = DrawingEngine::initialize()?;
        let renderer = Renderer::new(mode)?;
        Ok(Self {
            renderer,
            engine,
            _apartment: apartment,
        })
    }
}
impl TextMeasurer for NativeOffscreen {
    fn measure_rich(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
    ) -> crate::Result<SizeF> {
        self.renderer.measure_rich(paragraph, width)
    }
    fn rich_text_rects(
        &mut self,
        paragraph: &crate::ui::styling::text::RichParagraph,
        width: f32,
        range: std::ops::Range<usize>,
    ) -> crate::Result<Vec<crate::ui::scene::RectF>> {
        self.renderer.rich_text_rects(paragraph, width, range)
    }
    fn measure_formatted(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<SizeF> {
        self.renderer.measure_formatted(value, format, width)
    }
    fn layout_font_line(&mut self, value: &str, size: f32, font: Font) -> crate::Result<TextLine> {
        self.renderer.layout_font_line(value, size, font)
    }
    fn layout_formatted_line(
        &mut self,
        value: &str,
        format: TextFormat,
    ) -> crate::Result<TextLine> {
        self.renderer.layout_formatted_line(value, format)
    }
    fn layout_wrapped_text(
        &mut self,
        value: &str,
        format: TextFormat,
        width: f32,
    ) -> crate::Result<Vec<crate::ui::text_input::WrappedLine>> {
        self.renderer.layout_wrapped_text(value, format, width)
    }
    fn measure_font(
        &mut self,
        text: &str,
        size: f32,
        font: Font,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.renderer.measure_font(text, size, font, max_width)
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        bold: bool,
        max_width: f32,
    ) -> crate::Result<SizeF> {
        self.renderer.measure_text(text, size, bold, max_width)
    }
    fn layout_line(&mut self, value: &str, size: f32) -> crate::Result<TextLine> {
        self.renderer.layout_line(value, size)
    }
}
impl Offscreen for NativeOffscreen {
    fn label(&self) -> &'static str {
        self.renderer.label()
    }
    fn resources(&self) -> &dyn crate::application::Resources {
        &self.engine
    }
    fn load_fonts(&mut self, fonts: &[FontAsset]) -> crate::Result<()> {
        self.renderer.load_fonts(fonts)
    }
    fn render(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        scale: f32,
    ) -> crate::Result<Vec<u8>> {
        self.renderer.render(scene, width, height, scale)
    }
}
