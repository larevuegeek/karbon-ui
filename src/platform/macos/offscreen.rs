//! Services hors écran du backend macOS : moteur de rendu et compteurs du processus.
use super::{graphics::Renderer, imaging::Imaging};
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

/// Mémoire résidente (courante et maximale), empreinte physique et temps CPU du
/// processus, en unités de 100 ns comme sous Windows.
#[allow(deprecated)] // `mach_task_self` : libc renvoie vers `mach2`, crate tierce exclue.
pub(super) fn sample() -> crate::Result<ProcessSample> {
    // SAFETY : structure C de nombres, valide à zéro.
    let mut info: libc::mach_task_basic_info = unsafe { std::mem::zeroed() };
    let mut count = (std::mem::size_of::<libc::mach_task_basic_info>()
        / std::mem::size_of::<libc::natural_t>())
        as libc::mach_msg_type_number_t;
    // SAFETY : tâche courante ; `info` est valide en écriture pour `count` mots.
    let status = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            (&raw mut info).cast(),
            &mut count,
        )
    };
    if status != libc::KERN_SUCCESS {
        return Err(crate::Error::Platform {
            operation: "task_info",
            code: status,
        });
    }
    // SAFETY : structure C de nombres, valide à zéro.
    let mut usage: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    // SAFETY : processus courant ; la structure v2 est valide en écriture.
    let footprint = if unsafe {
        libc::proc_pid_rusage(
            libc::getpid(),
            libc::RUSAGE_INFO_V2,
            (&raw mut usage).cast(),
        )
    } == 0
    {
        usage.ri_phys_footprint
    } else {
        info.resident_size
    };
    // SAFETY : structure C de nombres, valide à zéro.
    let mut times: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY : sortie locale valide en écriture.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut times) } != 0 {
        return Err(crate::Error::Platform {
            operation: "getrusage",
            code: std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
        });
    }
    let ticks = |t: libc::timeval| {
        (t.tv_sec.max(0) as u64)
            .saturating_mul(10_000_000)
            .saturating_add(t.tv_usec.max(0) as u64 * 10)
    };
    Ok(ProcessSample {
        working_set_bytes: info.resident_size as usize,
        private_bytes: footprint as usize,
        peak_working_set_bytes: info.resident_size_max as usize,
        cpu_100ns: ticks(times.ru_utime).saturating_add(ticks(times.ru_stime)),
    })
}
/// Moteur hors écran macOS : Core Text pour le texte, Core Graphics pour les pixels et
/// ImageIO pour les images.
pub(crate) struct NativeOffscreen {
    renderer: Renderer,
    imaging: Imaging,
}
impl NativeOffscreen {
    pub(super) fn new(_mode: crate::window::RenderMode) -> crate::Result<Self> {
        // Core Graphics rastérise sur le CPU : les modes GPU et logiciel sont identiques.
        Ok(Self {
            renderer: Renderer::new()?,
            imaging: Imaging,
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
        Renderer::LABEL
    }
    fn resources(&self) -> &dyn crate::application::Resources {
        &self.imaging
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
