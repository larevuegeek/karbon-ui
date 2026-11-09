//! Sonde de défilement (`KARBON_SCROLL_PROBE=<secondes>[:<attente>]`) : mêmes mesures sur
//! tous les systèmes, sans outil extérieur et sans toucher à la souris de l'utilisateur.
//! Après l'attente (chargement de la page), le moteur reçoit un cran de molette toutes les
//! 80 ms au centre de la fenêtre, vers le bas puis vers le haut, relève l'écart entre les
//! images présentées, écrit le résumé sur la sortie d'erreur et ferme l'application. La
//! page doit être assez longue : au bout, plus aucune image n'est dessinée (« arrêts »).
use super::*;
use std::time::Duration;

/// Écart entre deux crans de molette.
const NOTCH_EVERY: Duration = Duration::from_millis(80);
/// Un cran, en unités logiques (trois lignes de 100/3, comme sous Windows).
const NOTCH: f32 = 100.;
/// Écart au-delà duquel une image compte pour un à-coup.
const JANK_MS: f32 = 25.;
/// Écart au-delà duquel plus rien ne bougeait (bout de la page) : compté à part.
const STOP_MS: f32 = 250.;
/// Attente avant la mesure, sans valeur donnée.
const DEFAULT_WARMUP_S: u64 = 5;

pub(super) struct ScrollProbe {
    duration: Duration,
    warmup: Duration,
    /// Début de la mesure, connu à la première image.
    start: Option<Instant>,
    next_notch: Option<Instant>,
    last_frame: Option<Instant>,
    /// Écarts entre images, en millisecondes.
    gaps: Vec<f32>,
    finished: bool,
    /// La mesure est finie, l'application reste à fermer.
    exit: bool,
}

/// `30` ou `30:8` : durée de la mesure et attente, en secondes.
fn parse(value: &str) -> Option<(u64, u64)> {
    let (duration, warmup) = match value.split_once(':') {
        Some((duration, warmup)) => (duration.parse().ok()?, warmup.parse().ok()?),
        None => (value.parse().ok()?, DEFAULT_WARMUP_S),
    };
    (duration > 0).then_some((duration, warmup))
}

/// Résumé des écarts `gaps` (millisecondes) relevés pendant `duration`.
fn summary(gaps: &[f32], duration: Duration) -> String {
    if gaps.is_empty() {
        return "aucune image".into();
    }
    let mut sorted: Vec<f32> = gaps.iter().copied().filter(|gap| *gap <= STOP_MS).collect();
    sorted.sort_by(f32::total_cmp);
    let Some(&max) = sorted.last() else {
        return format!("aucune image en mouvement, arrêts {}", gaps.len());
    };
    let percentile = |p: usize| sorted[(sorted.len() - 1) * p / 100];
    format!(
        "{} images, {:.1} images/s, écart médian {:.1} ms, p95 {:.1} ms, max {:.1} ms, à-coups {}, arrêts {}",
        gaps.len() + 1,
        (gaps.len() + 1) as f64 / duration.as_secs_f64(),
        percentile(50),
        percentile(95),
        max,
        sorted.iter().filter(|gap| **gap > JANK_MS).count(),
        gaps.len() - sorted.len(),
    )
}

impl ScrollProbe {
    pub(super) fn from_env() -> Option<Self> {
        let value = std::env::var("KARBON_SCROLL_PROBE").ok()?;
        let (duration, warmup) = parse(&value)?;
        Some(Self::new(
            Duration::from_secs(duration),
            Duration::from_secs(warmup),
        ))
    }

    fn new(duration: Duration, warmup: Duration) -> Self {
        Self {
            duration,
            warmup,
            start: None,
            next_notch: None,
            last_frame: None,
            gaps: Vec::new(),
            finished: false,
            exit: false,
        }
    }

    /// Prochain réveil demandé au runtime.
    fn wake(&self) -> Option<Instant> {
        if self.exit {
            Some(Instant::now())
        } else if self.finished {
            None
        } else {
            self.next_notch
        }
    }

    fn finish_if_due(&mut self, now: Instant) {
        let Some(start) = self.start else {
            return;
        };
        if !self.finished && now >= start + self.duration {
            self.finished = true;
            self.exit = true;
            eprintln!(
                "karbon-ui: sonde de défilement : {}",
                summary(&self.gaps, self.duration)
            );
        }
    }

    /// Une image vient d'être présentée.
    fn presented(&mut self, now: Instant) {
        if self.finished {
            return;
        }
        let start = match self.start {
            Some(start) => start,
            None => {
                let start = now + self.warmup;
                self.start = Some(start);
                self.next_notch = Some(start);
                start
            }
        };
        if now < start {
            return;
        }
        if let Some(last) = self.last_frame.replace(now) {
            self.gaps
                .push(now.saturating_duration_since(last).as_secs_f32() * 1e3);
        }
        self.finish_if_due(now);
    }

    /// Delta vertical du cran de molette dû à `now`, s'il y en a un.
    fn notch(&mut self, now: Instant) -> Option<f32> {
        self.finish_if_due(now);
        if self.finished {
            return None;
        }
        let mut next = self.next_notch.filter(|next| *next <= now)?;
        while next <= now {
            next += NOTCH_EVERY;
        }
        self.next_notch = Some(next);
        let start = self.start?;
        Some(if now < start + self.duration / 2 {
            NOTCH
        } else {
            -NOTCH
        })
    }
}

impl UiRuntime {
    pub(super) fn probe_wake(&self) -> Option<Instant> {
        self.invalidation.probe.as_ref().and_then(ScrollProbe::wake)
    }

    /// Envoie le cran de molette dû, au centre de la fenêtre.
    pub(super) fn probe_tick(&mut self, now: Instant) {
        let Some(context) = self.context else {
            return;
        };
        let Some(delta_y) = self
            .invalidation
            .probe
            .as_mut()
            .and_then(|probe| probe.notch(now))
        else {
            return;
        };
        self.event(UiEvent::Wheel {
            position: PointF {
                x: context.width / 2.,
                y: context.height / 2.,
            },
            delta_x: 0.,
            delta_y,
        });
    }

    /// `true` une fois, quand la mesure vient de finir : l'application doit se fermer.
    pub(super) fn probe_take_exit(&mut self) -> bool {
        self.invalidation
            .probe
            .as_mut()
            .is_some_and(|probe| std::mem::take(&mut probe.exit))
    }

    /// Une image vient d'être présentée à l'écran.
    pub(crate) fn frame_presented(&mut self, now: Instant) {
        if let Some(probe) = &mut self.invalidation.probe {
            probe.presented(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_and_optional_warmup_are_read_from_the_variable() {
        assert_eq!(parse("30"), Some((30, DEFAULT_WARMUP_S)));
        assert_eq!(parse("30:8"), Some((30, 8)));
        assert_eq!(parse("0"), None);
        assert_eq!(parse("trente"), None);
    }

    #[test]
    fn the_summary_counts_gaps_above_the_jank_threshold() {
        let gaps = [16., 17., 33., 16., 50.];
        assert_eq!(
            summary(&gaps, Duration::from_secs(1)),
            "6 images, 6.0 images/s, écart médian 17.0 ms, p95 33.0 ms, max 50.0 ms, à-coups 2, arrêts 0"
        );
        assert_eq!(
            summary(&[16., 2000.], Duration::from_secs(1)),
            "3 images, 3.0 images/s, écart médian 16.0 ms, p95 16.0 ms, max 16.0 ms, à-coups 0, arrêts 1"
        );
        assert_eq!(summary(&[], Duration::from_secs(1)), "aucune image");
    }

    #[test]
    fn notches_go_down_then_up_and_the_probe_ends_once() {
        let mut probe = ScrollProbe::new(Duration::from_secs(10), Duration::from_secs(1));
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        assert_eq!(probe.notch(t0), None);
        probe.presented(t0);
        assert_eq!(probe.notch(at(500)), None);
        assert_eq!(probe.notch(at(1000)), Some(NOTCH));
        assert_eq!(probe.notch(at(1001)), None);
        assert_eq!(probe.notch(at(7000)), Some(-NOTCH));
        probe.presented(at(5000));
        probe.presented(at(5016));
        probe.presented(at(11000));
        assert_eq!(probe.gaps.len(), 2);
        assert!(probe.wake().is_some());
        assert!(std::mem::take(&mut probe.exit));
        assert_eq!(probe.notch(at(12000)), None);
        assert!(probe.wake().is_none());
    }
}
