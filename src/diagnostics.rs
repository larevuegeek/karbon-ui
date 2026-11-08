//! Offscreen rendering measurements and previews. This protocol does not measure
//! desktop presentation (composition and display).
//!
//! [`render_benchmark`] and [`render_preview`] run an application without opening a
//! window; [`write_png`] saves a rendered image. The Windows backend renders with
//! Direct2D; on other targets the [headless](crate::platform::headless) software
//! rasteriser is used (shapes, lines and images; glyphs drawn as boxes).
use crate::{
    platform::{Backend, Native},
    window::RenderMode,
};
use std::time::Duration;
/// Settings of an offscreen benchmark or preview render.
///
/// Rendering fails with [`Error::InvalidInput`](crate::Error::InvalidInput) unless `width` and `height` are
/// within 200..=4096, `frames` within 1..=2048, `scale` within 0.5..=2.0, and the scaled
/// surface is at most 16,777,216 pixels.
#[derive(Clone, Copy)]
pub struct BenchmarkOptions {
    /// Disable animations and transitions.
    pub reduced_motion: bool,
    /// Reserve and draw the built-in title bar.
    pub title_bar: bool,
    /// Surface width in logical pixels.
    pub width: u32,
    /// Surface height in logical pixels, title bar included.
    pub height: u32,
    /// Number of measured frames per benchmark phase.
    pub frames: usize,
    /// Device scale factor applied to the logical size.
    pub scale: f32,
    /// Rendering backend to use.
    pub renderer: RenderMode,
    /// Theme passed to the application's view.
    pub theme: crate::ui::theme::Theme,
    /// Render with a transparent background.
    pub transparent: bool,
    /// Pointer position in logical pixels for [`render_preview`] and [`inspect_preview`]:
    /// the page is rendered with this point hovered (`:hover` styles). `None` by default.
    pub pointer: Option<(f32, f32)>,
    /// Point clicked (press and release, in logical pixels) at the start of the
    /// `"animation"` phase of [`render_benchmark`], to measure the transition it starts.
    /// `None` by default: the phase measures the animations already running.
    pub click: Option<(f32, f32)>,
}
impl Default for BenchmarkOptions {
    fn default() -> Self {
        Self {
            reduced_motion: false,
            title_bar: true,
            width: 1360,
            height: 920,
            frames: 1024,
            scale: 1.,
            renderer: RenderMode::Auto,
            theme: crate::ui::theme::Theme::DARK,
            transparent: false,
            pointer: None,
            click: None,
        }
    }
}
/// Process memory and CPU counters sampled around a benchmark phase.
///
/// Every counter is zero when the backend cannot sample the process (headless backend).
#[derive(Debug, Clone, Copy)]
pub struct ProcessSample {
    /// Current working set, in bytes.
    pub working_set_bytes: usize,
    /// Private (committed) memory, in bytes.
    pub private_bytes: usize,
    /// Peak working set since the process started, in bytes.
    pub peak_working_set_bytes: usize,
    /// Total user and kernel CPU time of the process, in 100 ns units.
    pub cpu_100ns: u64,
}
/// Measurements of one benchmark phase (for example a static repaint or a scroll).
pub struct BenchmarkPhase {
    /// Phase name: `"static-repaint"`, `"scroll"`, `"rebuild"` (view rebuilt every frame),
    /// `"hover"` (the pointer moves between two rows every frame), `"animation"` (the
    /// clock advances by one 60 Hz frame per frame, after the optional
    /// [`BenchmarkOptions::click`]) or `"wheel"` (smooth wheel notches under a resting
    /// pointer, one every four frames, down then up, on the real clock).
    pub name: &'static str,
    /// Number of measured frames.
    pub frames: usize,
    /// Median frame time (prepare and render), in milliseconds.
    pub p50_ms: f64,
    /// 95th percentile frame time, in milliseconds.
    pub p95_ms: f64,
    /// Slowest frame time, in milliseconds.
    pub max_ms: f64,
    /// Median time spent in the UI runtime alone (input event, rebuild, styles,
    /// layout, scene), rendering and readback excluded, in milliseconds.
    pub prepare_p50_ms: f64,
    /// 95th percentile of the UI runtime time, in milliseconds.
    pub prepare_p95_ms: f64,
    /// Slowest UI runtime time, in milliseconds.
    pub prepare_max_ms: f64,
    /// Wall-clock duration of the phase.
    pub wall: Duration,
    /// Process CPU time spent during the phase, in milliseconds.
    pub cpu_ms: f64,
    /// CPU time as a percentage of one core over the phase's wall time.
    pub cpu_one_core_percent: f64,
    /// Process counters sampled before the phase.
    pub before: ProcessSample,
    /// Process counters sampled after the phase.
    pub after: ProcessSample,
    /// Number of view rebuilds during the phase.
    pub view_rebuilds: u64,
    /// Number of layout passes during the phase.
    pub layouts: u64,
    /// Number of scene repaints during the phase.
    pub scene_paints: u64,
    /// Mean time per frame building the view, in milliseconds.
    pub view_ms: f64,
    /// Mean time per frame resolving styles, in milliseconds.
    pub style_ms: f64,
    /// Mean time per frame in layout, in milliseconds.
    pub layout_ms: f64,
    /// Mean time per frame painting the scene, in milliseconds.
    pub paint_ms: f64,
    /// Frames after which the runtime asked for the next one (its next wake falls within
    /// one 60 Hz frame). A running animation keeps it equal to the number of frames in the
    /// `"animation"` phase; it is zero in the other phases.
    pub frame_requests: u64,
}
/// Result of [`render_benchmark`].
pub struct BenchmarkReport {
    /// Label of the renderer that was used.
    pub backend: &'static str,
    /// Options the benchmark ran with.
    pub options: BenchmarkOptions,
    /// Time spent initialising the renderer and application, warm-up frames included.
    pub initialization: Duration,
    /// Measured phases, in execution order.
    pub phases: Vec<BenchmarkPhase>,
    /// Number of image draw commands in the scene.
    pub image_commands: usize,
    /// Number of distinct images drawn by the scene.
    pub unique_images: usize,
    /// Pixel bytes retained by the distinct images of the scene.
    pub scene_image_bytes: usize,
}
pub(crate) fn json_string(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c <= '\u{1f}' => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}
impl BenchmarkReport {
    /// Serialise the report as JSON (protocol `offscreen-prepare-render-readback-v1`).
    pub fn to_json(&self) -> String {
        let mut output = format!(
            "{{\n  \"protocol\": \"offscreen-prepare-render-readback-v1\",\n  \"backend\": \"{}\",\n  \"debug_build\": {},\n  \"width\": {}, \"height\": {}, \"scale\": {},\n  \"initialization_ms\": {:.3},\n",
            json_string(self.backend),
            cfg!(debug_assertions),
            self.options.width,
            self.options.height,
            self.options.scale,
            self.initialization.as_secs_f64() * 1000.
        );
        output.push_str(&format!("  \"logical_cpus\": {}, \"warmup_frames\": 3,\n  \"image_commands\": {}, \"unique_images\": {}, \"scene_image_bytes\": {},\n  \"theme\": \"{}\", \"transparent\": {},\n  \"phases\": [",std::thread::available_parallelism().map_or(1,usize::from),self.image_commands,self.unique_images,self.scene_image_bytes,
            if self.options.theme==crate::ui::theme::Theme::DARK {"dark"}else if self.options.theme==crate::ui::theme::Theme::LIGHT {"light"}else{"custom"},self.options.transparent));
        for (index, phase) in self.phases.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&format!("\n    {{\"name\": \"{}\", \"frames\": {}, \"p50_ms\": {:.3}, \"p95_ms\": {:.3}, \"max_ms\": {:.3}, \"prepare_p50_ms\": {:.3}, \"prepare_p95_ms\": {:.3}, \"prepare_max_ms\": {:.3}, \"wall_ms\": {:.3}, \"cpu_ms\": {:.3}, \"cpu_one_core_percent\": {:.2}, \"working_set_before_bytes\": {}, \"working_set_after_bytes\": {}, \"private_after_bytes\": {}, \"peak_working_set_bytes\": {}, \"view_rebuilds\": {}, \"layouts\": {}, \"scene_paints\": {}, \"view_ms\": {:.3}, \"style_ms\": {:.3}, \"layout_ms\": {:.3}, \"paint_ms\": {:.3}, \"frame_requests\": {}}}",json_string(phase.name),phase.frames,phase.p50_ms,phase.p95_ms,phase.max_ms,phase.prepare_p50_ms,phase.prepare_p95_ms,phase.prepare_max_ms,phase.wall.as_secs_f64()*1000.,phase.cpu_ms,phase.cpu_one_core_percent,phase.before.working_set_bytes,phase.after.working_set_bytes,phase.after.private_bytes,phase.after.peak_working_set_bytes,phase.view_rebuilds,phase.layouts,phase.scene_paints,phase.view_ms,phase.style_ms,phase.layout_ms,phase.paint_ms,phase.frame_requests));
        }
        output.push_str(&format!(
            "\n  ],\n  \"title_bar\": {},\n  \"reduced_motion\": {}\n}}\n",
            self.options.title_bar, self.options.reduced_motion
        ));
        output
    }
}
pub(crate) fn validate(options: BenchmarkOptions) -> crate::Result<()> {
    if options.width < 200
        || options.height < 200
        || options.width > 4096
        || options.height > 4096
        || !(1..=2048).contains(&options.frames)
        || !options.scale.is_finite()
        || !(0.5..=2.).contains(&options.scale)
        || options.width as f64 * options.height as f64 * f64::from(options.scale).powi(2)
            > 16_777_216.
        || [options.pointer, options.click]
            .into_iter()
            .flatten()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return Err(crate::Error::invalid_input("invalid benchmark options"));
    }
    Ok(())
}
/// Encode `image` as a PNG file at `path`, leaving its pixels unchanged.
///
/// The Windows backend uses the system PNG encoder (GDI+); other targets use the
/// headless backend's small built-in encoder (uncompressed deflate blocks, larger
/// files).
///
/// # Errors
///
/// [`ImageError::InvalidPixels`](crate::ImageError::InvalidPixels) when the image is
/// empty, larger than 16384 pixels per edge or its buffer does not hold
/// `width * height * 4` bytes, [`Error::InvalidInput`](crate::Error::InvalidInput) when
/// `path` contains a NUL character, [`Error::Platform`](crate::Error::Platform) when the
/// native encoder fails and [`Error::Io`](crate::Error::Io) when the file cannot be
/// written by the built-in encoder.
pub fn write_png(image: &crate::ui::scene::Image, path: &std::path::Path) -> crate::Result<()> {
    Native::write_png(image, path)
}
/// Render one frame of the application to pixels, without opening a window.
///
/// Background tasks requested while loading are run to completion first. On Windows the
/// frame is drawn by Direct2D (GPU or WARP); on other targets by the headless software
/// rasteriser, which draws shapes, lines and images but replaces glyphs with boxes (see
/// [`platform::headless`](crate::platform::headless)).
///
/// # Errors
///
/// [`Error::InvalidInput`](crate::Error::InvalidInput) for out-of-range
/// [`BenchmarkOptions`], any error returned by
/// [`Application::load`](crate::application::Application::load),
/// [`Error::Unsupported`](crate::Error::Unsupported) when the application opens windows
/// (its other window commands are ignored: the simulated window keeps its size),
/// [`Error::TimedOut`](crate::Error::TimedOut) when its background tasks take more than
/// ten seconds, and [`Error::Platform`](crate::Error::Platform) when rendering fails.
pub fn render_preview(
    options: BenchmarkOptions,
    application: impl crate::application::Application,
) -> crate::Result<crate::ui::scene::Image> {
    crate::platform::offscreen::render_preview::<Native>(options, Box::new(application))
}
/// Lay out one frame of the application without opening a window, then hand the laid
/// out tree to `inspect` (element bounds, classes and computed styles).
///
/// Text is measured by the native backend, as for [`render_preview`], so the bounds are
/// those of the rendered image. Nothing is rasterised. Useful to compare a page with a
/// reference layout.
///
/// # Errors
///
/// Same as [`render_preview`].
pub fn inspect_preview<R>(
    options: BenchmarkOptions,
    application: impl crate::application::Application,
    inspect: impl FnOnce(&crate::ui::element::Element) -> R,
) -> crate::Result<R> {
    crate::platform::offscreen::inspect_preview::<Native, R>(
        options,
        Box::new(application),
        inspect,
    )
}
/// Render the application offscreen for a fixed number of frames and measure frame
/// times, CPU and memory.
///
/// Three warm-up frames are rendered first, then static repaint, scroll, rebuild, hover
/// and animation phases of [`BenchmarkOptions::frames`] frames each. The animation phase
/// runs on a simulated clock advanced by 16.67 ms per frame (a timer event, then the
/// frame), as in a window paced by a 60 Hz display. On targets without a native backend
/// the headless software rasteriser is measured and the [`ProcessSample`] counters are
/// zero.
///
/// # Errors
///
/// Same as [`render_preview`].
pub fn render_benchmark(
    options: BenchmarkOptions,
    application: impl crate::application::Application,
) -> crate::Result<BenchmarkReport> {
    crate::platform::offscreen::render_benchmark::<Native>(options, Box::new(application))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn benchmark_limits_reject_unbounded_runs_and_excessive_pixel_allocations() {
        assert!(validate(Default::default()).is_ok());
        for options in [
            BenchmarkOptions {
                frames: 0,
                ..Default::default()
            },
            BenchmarkOptions {
                frames: 2049,
                ..Default::default()
            },
            BenchmarkOptions {
                scale: f32::NAN,
                ..Default::default()
            },
            BenchmarkOptions {
                pointer: Some((f32::INFINITY, 0.)),
                ..Default::default()
            },
            BenchmarkOptions {
                click: Some((0., f32::NAN)),
                ..Default::default()
            },
            BenchmarkOptions {
                width: 4096,
                height: 4096,
                scale: 2.,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                validate(options),
                Err(crate::Error::InvalidInput { .. })
            ));
        }
    }
}
