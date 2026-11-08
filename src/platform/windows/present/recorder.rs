//! Mesures opt-in du chemin applicatif jusqu'au retour de la soumission : Present
//! (composition) ou UpdateLayeredWindow (layered).
//! Aucun timer, rendu supplémentaire ou attente de composition n'est ajouté.
use super::{super::offscreen::sample, composition::Path};
use crate::{
    diagnostics::{ProcessSample, json_string},
    window::WindowId,
};
use std::{
    collections::VecDeque,
    io::Write,
    time::{Duration, Instant},
};

const SAMPLE_LIMIT: usize = 1024;
/// Écart entre deux présentations au-delà duquel la fenêtre était au repos (pas une
/// image manquée d'un mouvement continu).
const IDLE_GAP_MS: f64 = 100.;
/// Écart qui révèle au moins une image manquée à 60 Hz (une image et demie).
const LATE_GAP_MS: f64 = 25.;
/// Préparation et rendu au-delà desquels une image est signalée sur-le-champ (comme une
/// image présentée en retard, voir `LATE_GAP_MS`).
const SLOW_FRAME_MS: f64 = 10.;
/// `render` inclut la relecture CPU sur le chemin layered seulement.
const STAGES: [&str; 5] = ["prepare", "render", "surface", "submit", "total"];

pub(super) struct Recorder {
    id: WindowId,
    path: Path,
    started: Instant,
    before: Option<ProcessSample>,
    samples: VecDeque<[f64; 5]>,
    /// Écarts entre présentations successives d'un mouvement continu (millisecondes).
    intervals: VecDeque<f64>,
    last_present: Option<Instant>,
    /// Écarts qui ont manqué au moins une image (voir `LATE_GAP_MS`).
    late_frames: u64,
    pub frames: u64,
    pub failures: u64,
    pub recoveries: u64,
    pub surface_creations: u64,
    size_changes: u64,
    last_size: (u32, u32),
    last_scale: f32,
}
impl Recorder {
    /// Nouveau segment de mesures pour la même fenêtre et le même chemin.
    pub fn restart(&self) -> Self {
        Self::new(self.id, self.path)
    }
    pub fn new(id: WindowId, path: Path) -> Self {
        Self {
            id,
            path,
            started: Instant::now(),
            before: sample().ok(),
            samples: VecDeque::with_capacity(SAMPLE_LIMIT),
            intervals: VecDeque::with_capacity(SAMPLE_LIMIT),
            last_present: None,
            late_frames: 0,
            frames: 0,
            failures: 0,
            recoveries: 0,
            surface_creations: 0,
            size_changes: 0,
            last_size: (0, 0),
            last_scale: 1.,
        }
    }
    pub fn record(&mut self, timings: [Duration; 5], size: (u32, u32), scale: f32) {
        if self.frames > 0 && self.last_size != size {
            self.size_changes += 1;
        }
        self.frames += 1;
        self.last_size = size;
        self.last_scale = scale;
        if self.samples.len() == SAMPLE_LIMIT {
            self.samples.pop_front();
        }
        self.samples
            .push_back(timings.map(|time| time.as_secs_f64() * 1000.));
    }
    /// Note une présentation ; renvoie l'écart avec la précédente quand la fenêtre était
    /// en mouvement continu.
    pub fn presented(&mut self, at: Instant) -> Option<f64> {
        let gap = self
            .last_present
            .replace(at)
            .map(|last| at.duration_since(last).as_secs_f64() * 1000.)
            .filter(|gap| *gap <= IDLE_GAP_MS)?;
        if gap > LATE_GAP_MS {
            self.late_frames += 1;
        }
        if self.intervals.len() == SAMPLE_LIMIT {
            self.intervals.pop_front();
        }
        self.intervals.push_back(gap);
        Some(gap)
    }
    /// Signale sur-le-champ une image lente ou présentée en retard : étapes de la fenêtre
    /// (`timings`, comme `record`), attente de sa place dans la file de présentation (la
    /// cadence de l'écran, pas du travail), étapes du moteur (vue, styles, mise en page,
    /// peinture) et écart avec la présentation précédente.
    pub fn slow_frame(
        &self,
        timings: [Duration; 5],
        wait: Duration,
        engine: [Duration; 4],
        gap: Option<f64>,
    ) {
        let ms = |d: Duration| d.as_secs_f64() * 1000.;
        let late = gap.is_some_and(|g| g > LATE_GAP_MS);
        if ms(timings[0]) + ms(timings[1]) < SLOW_FRAME_MS && !late {
            return;
        }
        let gap = gap.map_or("null".into(), |g| format!("{g:.3}"));
        let _ = writeln!(
            std::io::stdout().lock(),
            "Slow frame : {{\"window_id\":{},\"prepare_ms\":{:.3},\"view_ms\":{:.3},\"style_ms\":{:.3},\"layout_ms\":{:.3},\"paint_ms\":{:.3},\"render_ms\":{:.3},\"submit_ms\":{:.3},\"wait_ms\":{:.3},\"gap_ms\":{gap}}}",
            self.id.0,
            ms(timings[0]),
            ms(engine[0]),
            ms(engine[1]),
            ms(engine[2]),
            ms(engine[3]),
            ms(timings[1]),
            ms(timings[3]),
            ms(wait),
        );
    }
    pub fn report(&self, backend: &str, reason: &str) -> String {
        let after = sample().ok();
        self.report_at(backend, reason, self.started.elapsed(), after)
    }
    fn report_at(
        &self,
        backend: &str,
        reason: &str,
        elapsed: Duration,
        after: Option<ProcessSample>,
    ) -> String {
        let mut output = format!(
            "{{\"protocol\":\"window-present-v4\",\"window_id\":{},\"presentation\":\"{}\",\"backend\":\"{}\",\"reason\":\"{}\",\"debug_build\":{},\"frames\":{},\"failed_paints\":{},\"renderer_recoveries\":{},\"surface_creations\":{},\"size_changes\":{},\"last_width_px\":{},\"last_height_px\":{},\"last_scale\":{},\"retained_samples\":{},\"sample_limit\":{},\"session_wall_ms\":{:.3},\"stages\":{{",
            self.id.0,
            self.path.label(),
            json_string(backend),
            json_string(reason),
            cfg!(debug_assertions),
            self.frames,
            self.failures,
            self.recoveries,
            self.surface_creations,
            self.size_changes,
            self.last_size.0,
            self.last_size.1,
            self.last_scale,
            self.samples.len(),
            SAMPLE_LIMIT,
            elapsed.as_secs_f64() * 1000.
        );
        for (index, name) in STAGES.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            let mut times: Vec<_> = self.samples.iter().map(|sample| sample[index]).collect();
            times.sort_by(f64::total_cmp);
            output.push_str(&format!("\"{name}\":{}", summary(&times)));
        }
        let mut gaps: Vec<_> = self.intervals.iter().copied().collect();
        gaps.sort_by(f64::total_cmp);
        output.push_str(&format!(
            "}},\"intervals\":{},\"late_frames\":{},\"process\":",
            summary(&gaps),
            self.late_frames
        ));
        if let (Some(before), Some(after)) = (self.before, after) {
            let cpu_ms = after.cpu_100ns.saturating_sub(before.cpu_100ns) as f64 / 10_000.;
            output.push_str(&format!("{{\"cpu_ms\":{:.3},\"cpu_one_core_percent\":{:.3},\"working_set_before_bytes\":{},\"working_set_after_bytes\":{},\"private_after_bytes\":{},\"peak_working_set_bytes\":{}}}",
                cpu_ms, cpu_ms / elapsed.as_secs_f64().max(f64::EPSILON) / 10.,
                before.working_set_bytes, after.working_set_bytes, after.private_bytes, after.peak_working_set_bytes));
        } else {
            output.push_str("null");
        }
        output.push('}');
        output
    }
    pub fn emit(&self, backend: &str, reason: &str) {
        // Un terminal fermé ne doit pas faire paniquer le callback natif de fermeture.
        let _ = writeln!(
            std::io::stdout().lock(),
            "Presentation : {}",
            self.report(backend, reason)
        );
    }
}
fn summary(sorted: &[f64]) -> String {
    if sorted.is_empty() {
        return "null".into();
    }
    let at = |percent: f64| sorted[(sorted.len() as f64 * percent).ceil() as usize - 1];
    format!(
        "{{\"p50_ms\":{:.3},\"p95_ms\":{:.3},\"max_ms\":{:.3}}}",
        at(0.5),
        at(0.95),
        sorted.last().unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples_stay_bounded_and_percentiles_only_cover_the_retained_window() {
        let mut recorder = Recorder::new(WindowId(4), Path::Composition);
        recorder.record([Duration::from_secs(99); 5], (300, 200), 1.);
        for index in 0..SAMPLE_LIMIT {
            recorder.record([Duration::from_millis(index as u64 + 1); 5], (600, 400), 2.);
        }
        assert_eq!(recorder.frames, 1025);
        assert_eq!(recorder.size_changes, 1);
        assert_eq!(recorder.samples.len(), SAMPLE_LIMIT);
        assert_eq!(recorder.samples.capacity(), SAMPLE_LIMIT);
        let report = recorder.report_at("test\"backend", "test", Duration::from_secs(10), None);
        assert!(report.contains("\"p50_ms\":512.000,\"p95_ms\":973.000,\"max_ms\":1024.000"));
        assert!(report.contains("test\\\"backend"));
        assert!(report.contains("\"presentation\":\"composition\","));
        assert!(report.ends_with("\"process\":null}"));
        let restarted = recorder.restart();
        assert_eq!((restarted.frames, restarted.id), (0, WindowId(4)));
        assert_eq!(restarted.path, Path::Composition);
    }
    #[test]
    fn session_cpu_includes_idle_and_reports_process_wide_memory() {
        let mut recorder = Recorder::new(WindowId(8), Path::Layered);
        let sample = ProcessSample {
            working_set_bytes: 100,
            private_bytes: 80,
            peak_working_set_bytes: 150,
            cpu_100ns: 0,
        };
        recorder.before = Some(sample);
        let report = recorder.report_at(
            "test",
            "idle",
            Duration::from_secs(10),
            Some(ProcessSample {
                cpu_100ns: 20_000_000,
                ..sample
            }),
        );
        assert!(report.contains("\"cpu_ms\":2000.000,\"cpu_one_core_percent\":20.000"));
        assert!(report.contains("\"prepare\":null"));
        assert!(report.contains("\"working_set_after_bytes\":100"));
        assert_eq!(recorder.frames, 0);
    }
}
