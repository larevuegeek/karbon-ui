//! Services hors écran du backend Linux : moteur de rendu et compteurs du processus.
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

/// Valeur en kio d'un champ de `/proc/self/status` (`VmRSS:   1234 kB`), en octets.
fn status_bytes(status: &str, field: &str) -> Option<usize> {
    let line = status.lines().find(|l| l.starts_with(field))?;
    let kib = line[field.len()..]
        .trim_start_matches(':')
        .split_whitespace()
        .next()?
        .parse::<usize>()
        .ok()?;
    Some(kib * 1024)
}
/// Mémoire résidente (courante et maximale), mémoire anonyme résidente (privée) et temps
/// CPU du processus, en unités de 100 ns comme sous Windows.
pub(super) fn sample() -> crate::Result<ProcessSample> {
    // Sans procfs (conteneur minimal), `getrusage` donne au moins le pic résident.
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    // SAFETY : `rusage` est une structure C de champs entiers : tout à zéro est valide.
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
    // `ru_maxrss` est en kio sous Linux.
    let peak = status_bytes(&status, "VmHWM")
        .unwrap_or((times.ru_maxrss.max(0) as usize).saturating_mul(1024));
    let resident = status_bytes(&status, "VmRSS").unwrap_or(peak);
    Ok(ProcessSample {
        working_set_bytes: resident,
        private_bytes: status_bytes(&status, "RssAnon").unwrap_or(resident),
        peak_working_set_bytes: peak,
        cpu_100ns: ticks(times.ru_utime).saturating_add(ticks(times.ru_stime)),
    })
}
/// Moteur hors écran Linux : Pango pour le texte, Cairo pour les pixels et gdk-pixbuf
/// pour les images. En mode `Gpu` explicite, les pixels viennent du renderer Vulkan
/// (aperçus et exports à comparer au chemin Cairo) ; sans Vulkan, ou si une image échoue,
/// Cairo dessine. Les modes `Auto` et `Software` restent sur Cairo (exports de référence).
pub(crate) struct NativeOffscreen {
    renderer: Renderer,
    gpu: Option<super::vulkan::Renderer>,
    imaging: Imaging,
}
impl NativeOffscreen {
    pub(super) fn new(mode: crate::window::RenderMode) -> crate::Result<Self> {
        let gpu = match mode {
            crate::window::RenderMode::Gpu => match super::vulkan::Renderer::new(false) {
                Ok(gpu) => Some(gpu),
                Err(error) => {
                    eprintln!("karbon-ui: Vulkan indisponible ({error}), rendu Cairo");
                    None
                }
            },
            _ => None,
        };
        Ok(Self {
            renderer: Renderer::new()?,
            gpu,
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
        if self.gpu.is_some() {
            super::vulkan::Renderer::LABEL
        } else {
            Renderer::LABEL
        }
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
        if let Some(gpu) = &mut self.gpu {
            match gpu.render(
                &mut self.renderer,
                scene,
                width as usize,
                height as usize,
                scale,
            ) {
                Ok(pixels) => return Ok(pixels),
                Err(error) => eprintln!("karbon-ui: image Vulkan refusée ({error}), rendu Cairo"),
            }
        }
        self.renderer.render(scene, width, height, scale)
    }
}
