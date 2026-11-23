//! Les données d'un éditeur de courbes, tenues par l'application : des courbes faites de
//! clés (un instant, une valeur, deux pentes, une façon d'aller à la suivante), et ce
//! qu'elles valent entre deux clés.
use crate::ui::theme::Color;

/// How a curve goes from a key to the next one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyMode {
    /// Smoothly, following the slopes of both keys (Hermite).
    #[default]
    Smooth,
    /// In a straight line.
    Linear,
    /// Stays at the value of the key until the next one.
    Constant,
}

/// A key of a [`Curve`]: a time, a value, the slopes on both sides (value per time
/// unit), and how the curve goes on to the next key.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CurveKey {
    /// Time (seconds, or any unit of the horizontal axis).
    pub time: f32,
    /// Value.
    pub value: f32,
    /// Slope arriving at the key.
    pub in_slope: f32,
    /// Slope leaving the key.
    pub out_slope: f32,
    /// How the curve goes on to the next key.
    pub mode: KeyMode,
}

impl CurveKey {
    /// A smooth key at `time` with `value`, flat on both sides.
    pub fn new(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            ..Self::default()
        }
    }
}

/// A curve of a [`CurveEditor`](super::CurveEditor): its name, its colour and its keys,
/// in the order of time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curve {
    /// Its name (« Position X », « Couple »).
    pub name: String,
    /// Its colour; the accent of the palette by default.
    pub color: Option<Color>,
    /// Its keys, in the order of time.
    pub keys: Vec<CurveKey>,
}

impl Curve {
    /// The value of the curve at `time`: the value of the first key before it, of the
    /// last one after it, and between two keys according to the mode of the first.
    ///
    /// ```
    /// use karbon_ui::ui::editor::{Curve, CurveKey, KeyMode};
    ///
    /// let ramp = Curve {
    ///     keys: vec![CurveKey { mode: KeyMode::Linear, ..CurveKey::new(0., 0.) }, CurveKey::new(2., 10.)],
    ///     ..Curve::default()
    /// };
    /// assert_eq!(ramp.value_at(1.), 5.);
    /// assert_eq!(ramp.value_at(-1.), 0.);
    /// ```
    pub fn value_at(&self, time: f32) -> f32 {
        let keys = &self.keys;
        let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
            return 0.;
        };
        if time <= first.time {
            return first.value;
        }
        if time >= last.time {
            return last.value;
        }
        let next = keys.partition_point(|k| k.time <= time).min(keys.len() - 1);
        segment_value(&keys[next - 1], &keys[next], time)
    }
}

/// La valeur entre les clés `a` et `b` à `time`.
pub(super) fn segment_value(a: &CurveKey, b: &CurveKey, time: f32) -> f32 {
    let span = b.time - a.time;
    if span <= 0. {
        return b.value;
    }
    let t = ((time - a.time) / span).clamp(0., 1.);
    match a.mode {
        KeyMode::Constant => a.value,
        KeyMode::Linear => a.value + (b.value - a.value) * t,
        KeyMode::Smooth => {
            let (t2, t3) = (t * t, t * t * t);
            let h00 = 2. * t3 - 3. * t2 + 1.;
            let h10 = t3 - 2. * t2 + t;
            let h01 = -2. * t3 + 3. * t2;
            let h11 = t3 - t2;
            h00 * a.value + h10 * span * a.out_slope + h01 * b.value + h11 * span * b.in_slope
        }
    }
}
