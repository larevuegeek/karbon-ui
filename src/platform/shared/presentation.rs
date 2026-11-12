//! Affichage réel des images pendant la sonde de défilement (`KARBON_SCROLL_PROBE`). La
//! sonde du runtime date les envois d'images ; le système dit quand chacune a paru à
//! l'écran (`wp_presentation` sous Wayland, `MTLDrawable.presentedTime` sous macOS), ou
//! qu'elle a été écartée (remplacée avant d'être montrée). Le résumé, écrit à la fermeture
//! de la fenêtre, distingue ainsi un envoi en retard d'une image vraiment manquée à
//! l'écran.

/// Écart au-delà duquel un affichage compte pour un à-coup (comme la sonde).
const JANK_MS: f64 = 25.;
/// Écart au-delà duquel plus rien ne bougeait : compté à part (comme la sonde).
const STOP_MS: f64 = 250.;

/// Instants d'affichage relevés pour une fenêtre.
#[derive(Default)]
pub(in crate::platform) struct Shown {
    /// Instants d'affichage, horloge du système, en millisecondes.
    times: Vec<f64>,
    /// Période de rafraîchissement de l'écran, en millisecondes (0 : inconnue).
    refresh: f64,
    discarded: usize,
}

impl Shown {
    /// Image affichée à l'instant `at` (millisecondes) sur un écran rafraîchi toutes les
    /// `refresh` millisecondes.
    pub fn shown(&mut self, at: f64, refresh: f64) {
        self.times.push(at);
        self.refresh = refresh;
    }

    /// Image remplacée avant d'avoir été montrée.
    pub fn discarded(&mut self) {
        self.discarded += 1;
    }

    /// Résumé des affichages de la mesure (ses dernières secondes) sur la sortie d'erreur ;
    /// `source` nomme l'interface du système qui les a donnés.
    pub fn report(&self, source: &str) {
        let duration = std::env::var("KARBON_SCROLL_PROBE")
            .ok()
            .and_then(|v| v.split(':').next()?.parse::<f64>().ok());
        let Some(duration) = duration else {
            return;
        };
        let Some(line) = self.summary(duration) else {
            return;
        };
        eprintln!("karbon-ui: sonde, affichage ({source}) : {line}");
    }

    /// Résumé des `duration` dernières secondes ; `None` sans image en mouvement.
    fn summary(&self, duration: f64) -> Option<String> {
        let &last = self.times.last()?;
        let shown: Vec<f64> = self
            .times
            .iter()
            .copied()
            .filter(|t| *t >= last - duration * 1e3)
            .collect();
        let gaps: Vec<f64> = shown.windows(2).map(|w| w[1] - w[0]).collect();
        let mut moving: Vec<f64> = gaps.iter().copied().filter(|g| *g <= STOP_MS).collect();
        moving.sort_by(f64::total_cmp);
        let &max = moving.last()?;
        let percentile = |p: usize| moving[(moving.len() - 1) * p / 100];
        // Rafraîchissements sans image nouvelle entre deux affichages.
        let missed: f64 = if self.refresh > 0. {
            moving
                .iter()
                .map(|g| ((g / self.refresh).round() - 1.).max(0.))
                .sum()
        } else {
            0.
        };
        Some(format!(
            "{} images, écart médian {:.1} ms, p95 {:.1} ms, max {:.1} ms, à-coups {}, rafraîchissements manqués {missed}, écartées {}, période {:.2} ms",
            shown.len(),
            percentile(50),
            percentile(95),
            max,
            moving.iter().filter(|g| **g > JANK_MS).count(),
            self.discarded,
            self.refresh,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missed_refreshes_are_counted_from_the_period() {
        let mut shown = Shown::default();
        for at in [0., 16.7, 33.4, 66.8, 83.5] {
            shown.shown(at, 16.7);
        }
        shown.discarded();
        assert_eq!(
            shown.summary(30.).as_deref(),
            Some(
                "5 images, écart médian 16.7 ms, p95 16.7 ms, max 33.4 ms, à-coups 1, rafraîchissements manqués 1, écartées 1, période 16.70 ms"
            )
        );
    }

    #[test]
    fn only_the_last_seconds_are_summarised() {
        let mut shown = Shown::default();
        for at in [0., 5_000., 5_016., 5_032.] {
            shown.shown(at, 0.);
        }
        assert!(shown.summary(1.).unwrap().starts_with("3 images"));
        assert_eq!(Shown::default().summary(1.), None);
    }
}
