use super::*;
/// Numeric slider controlled by the application (range 0–100, step 1 by default).
///
/// Invariants checked when the view is validated: finite `min < max`, `value`
/// within the range, and `0 < step <= max - min`.
pub struct Slider {
    /// Displayed value; must lie within `min..=max`.
    pub value: f64,
    /// Lower bound.
    pub min: f64,
    /// Upper bound.
    pub max: f64,
    /// Positive increment between reachable values.
    pub step: f64,
    /// Accessible name.
    pub label: Arc<str>,
    pub(crate) on_change: Handler<f64>,
    pub(crate) on_commit: Option<Handler<f64>>,
    /// Horizontal or vertical orientation.
    pub axis: Axis,
    /// Whether the slider accepts input.
    pub enabled: bool,
    /// Whether the value is shown and focusable but cannot be changed.
    pub read_only: bool,
    /// Appearance overrides.
    pub style: SliderStyle,
    pub(crate) css: Option<Box<crate::ui::styling::ComputedStyle>>,
}
impl Slider {
    /// Slider showing `value`; provisional values (drag, keys) are sent through
    /// `on_change`, for example `Msg::Volume` with `Volume(f64)`.
    pub fn new<M: 'static>(value: f64, on_change: impl Fn(f64) -> M + 'static) -> Self {
        Self {
            value,
            min: 0.,
            max: 100.,
            step: 1.,
            label: "Slider".into(),
            on_change: Handler::new(on_change),
            on_commit: None,
            axis: Axis::Horizontal,
            enabled: true,
            read_only: false,
            style: SliderStyle::default(),
            css: None,
        }
    }

    /// Set the bounds (`min < max`, both finite).
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set the increment between reachable values (`0 < step <= max - min`).
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set the accessible name.
    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = label.into();
        self
    }

    /// Final value of an interaction (pointer release, end of key repeat or an
    /// accessibility command), sent only when it differs from the starting value.
    pub fn on_commit<M: 'static>(mut self, on_commit: impl Fn(f64) -> M + 'static) -> Self {
        self.on_commit = Some(Handler::new(on_commit));
        self
    }

    /// Set the orientation.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Disable the slider: it is greyed out and ignores input.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Keep the slider focusable but prevent changing its value.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Override appearance.
    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    pub(crate) fn validate(&self) -> crate::Result<()> {
        let span = self.max - self.min;
        if !self.min.is_finite()
            || self
                .style
                .track_thickness
                .is_some_and(|v| !v.is_finite() || v <= 0. || v > 128.)
            || !self.max.is_finite()
            || !span.is_finite()
            || span <= 0.
            || !self.value.is_finite()
            || !(self.min..=self.max).contains(&self.value)
            || !self.step.is_finite()
            || self.step <= 0.
            || self.step > span
            || self.min + self.step <= self.min
            || self.max - self.step >= self.max
        {
            return Err(crate::Error::invalid_input(
                "invalid slider range, value or step",
            ));
        }
        Ok(())
    }

    /// Clamp `value` to the range and snap it to the nearest step from `min`;
    /// `max` is always reachable even when the range is not a multiple of `step`.
    ///
    /// An invalid range (`min > max`, or a bound that is not a number) returns `value`
    /// unchanged.
    pub fn snapped(&self, value: f64) -> f64 {
        if self
            .min
            .partial_cmp(&self.max)
            .is_none_or(|order| order.is_gt())
        {
            return value;
        }
        let value = value.clamp(self.min, self.max);
        if value == self.max || value == self.min {
            return value;
        }
        let tick = (self.min + ((value - self.min) / self.step).round() * self.step)
            .clamp(self.min, self.max);
        if self.max - value <= (tick - value).abs() {
            self.max
        } else {
            tick
        }
    }

    pub(super) fn shifted(&self, value: f64, count: i32) -> f64 {
        let ratio = (value - self.min) / self.step;
        let nearest = ratio.round();
        let ratio = if (ratio - nearest).abs() <= f64::EPSILON * ratio.abs().max(1.) * 4. {
            nearest
        } else {
            ratio
        };
        let tick = if count > 0 {
            ratio.floor() + count as f64
        } else {
            ratio.ceil() + count as f64
        };
        (self.min + tick * self.step).clamp(self.min, self.max)
    }

    pub(crate) fn info(&self, value: f64) -> RangeInfo {
        RangeInfo {
            value,
            min: self.min,
            max: self.max,
            small_change: self.step,
            large_change: (self.step * 10.).min(self.max - self.min),
            read_only: self.read_only,
            axis: self.axis,
        }
    }

    pub(super) fn config(&self) -> Config {
        Config {
            min: self.min,
            max: self.max,
            step: self.step,
            axis: self.axis,
        }
    }
}
