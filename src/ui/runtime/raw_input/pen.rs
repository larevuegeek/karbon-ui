//! Ce que le backend sait du pointeur en plus de sa place dans la fenêtre : la pression
//! et l'inclinaison d'un stylet (Windows Ink), et la place de la fenêtre sur l'écran, d'où
//! `PointerEvent::screen` (un panneau qu'on sort de la fenêtre s'ouvre sous le pointeur).
//! Le backend les pose avant chaque entrée ; sans eux, un stylet vaut une souris (pression
//! 1) et la fenêtre est au coin de l'écran.
use super::super::*;

/// La pression et l'inclinaison du stylet qui a fait l'entrée en cours, et le coin de la
/// zone client sur l'écran.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointerExtras {
    /// De 0 à 1 ; 1 pour une souris.
    pub pressure: f32,
    /// En degrés depuis la verticale, selon x et y ; nulle pour une souris.
    pub tilt: (f32, f32),
    /// Le coin en haut à gauche de la zone client, dans les unités de
    /// `WindowChange::Position`, et le rapport de ces unités aux unités de la fenêtre
    /// (l'échelle de son écran sur celle de l'écran principal).
    pub origin: (f32, f32),
    pub ratio: f32,
}

impl Default for PointerExtras {
    fn default() -> Self {
        Self {
            pressure: 1.,
            tilt: (0., 0.),
            origin: (0., 0.),
            ratio: 1.,
        }
    }
}

impl PointerExtras {
    /// La place sur l'écran du point `position` de la fenêtre.
    pub(crate) fn on_screen(&self, position: PointF) -> (f32, f32) {
        (
            self.origin.0 + position.x * self.ratio,
            self.origin.1 + position.y * self.ratio,
        )
    }
}

/// Ce qu'un stylet dit d'une entrée : sa pression de 0 à 1, son inclinaison en degrés.
pub(crate) type PenSample = (f32, (f32, f32));

/// La pression d'un stylet lue par Windows (0 à 1024), ramenée de 0 à 1.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn pen_pressure(raw: u32) -> f32 {
    (raw as f32 / 1024.).clamp(0., 1.)
}

impl UiRuntime {
    /// Le backend dit ce qu'il sait du stylet (`None` : une souris) et où est la zone
    /// client sur l'écran, avant de donner l'entrée suivante.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn set_pointer_extras(
        &mut self,
        pen: Option<PenSample>,
        origin: (f32, f32),
        ratio: f32,
    ) {
        let (pressure, tilt) = pen.unwrap_or((1., (0., 0.)));
        self.raw_pointer.extras = PointerExtras {
            pressure,
            tilt,
            origin,
            ratio,
        };
    }
}
