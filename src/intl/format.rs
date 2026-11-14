//! Nombres, horloges, durées, temps écoulé et pluriel, selon la langue.
use super::{Locale, Plural};

impl Locale {
    /// Integer with the locale's digit grouping, like `Intl.NumberFormat`.
    ///
    /// ```
    /// use karbon_ui::intl::Locale;
    ///
    /// assert_eq!(Locale::English.number(9440), "9,440");
    /// assert_eq!(Locale::French.number(9440), "9\u{202f}440");
    /// assert_eq!(Locale::Spanish.number(9440), "9440");
    /// ```
    pub fn number(self, value: i64) -> String {
        let data = self.data();
        let digits = value.unsigned_abs().to_string();
        let mut out = String::with_capacity(digits.len() + 8);
        if value < 0 {
            out.push('-');
        }
        let group = digits.len() >= data.min_grouping;
        for (i, c) in digits.chars().enumerate() {
            if group && i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push_str(data.group);
            }
            out.push(c);
        }
        out
    }
    /// Number with at most `fraction_digits` decimals, trailing zeros removed, with the
    /// locale's grouping and decimal separator.
    ///
    /// ```
    /// use karbon_ui::intl::Locale;
    ///
    /// assert_eq!(Locale::French.decimal(44.1, 1), "44,1");
    /// assert_eq!(Locale::English.decimal(96.0, 1), "96");
    /// assert_eq!(Locale::German.decimal(1234.5, 2), "1.234,5");
    /// ```
    pub fn decimal(self, value: f64, fraction_digits: usize) -> String {
        if !value.is_finite() {
            return String::new();
        }
        let text = format!("{:.*}", fraction_digits.min(9), value.abs());
        let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
        let fraction = fraction.trim_end_matches('0');
        let whole = whole.parse::<i64>().unwrap_or(0);
        let negative = value < 0. && (whole != 0 || !fraction.is_empty());
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out += &self.number(whole);
        if !fraction.is_empty() {
            out.push(self.data().decimal);
            out += fraction;
        }
        out
    }
    /// Running time like a player: `8:02`, `1:02:03` (hours only when needed).
    pub fn clock(self, seconds: f64) -> String {
        clock(seconds, false)
    }
    /// Running time with two-digit minutes: `08:02`, `1:02:03`.
    pub fn clock_padded(self, seconds: f64) -> String {
        clock(seconds, true)
    }
    /// Length in words, rounded to the minute: `42 min`, `3 h 05 min`, `25 d 6 h`.
    ///
    /// ```
    /// use karbon_ui::intl::Locale;
    ///
    /// assert_eq!(Locale::English.duration(3. * 3600. + 5. * 60.), "3 h 05 min");
    /// assert_eq!(Locale::French.duration(25. * 86400. + 6. * 3600.), "25 j 6 h");
    /// ```
    pub fn duration(self, seconds: f64) -> String {
        let [d_unit, h_unit, m_unit] = self.data().duration_units;
        let minutes = (seconds.max(0.) / 60.).round() as u64;
        let (d, h, m) = (minutes / 1440, (minutes % 1440) / 60, minutes % 60);
        if d > 0 {
            format!("{} {d_unit} {h} {h_unit}", self.number(d as i64))
        } else if h > 0 {
            format!("{h} {h_unit} {m:02} {m_unit}")
        } else {
            format!("{m} {m_unit}")
        }
    }
    /// Time elapsed since an event, in words, like `Intl.RelativeTimeFormat` with
    /// `numeric: "auto"`: `now`, `8 minutes ago`, `yesterday`, `3 months ago`.
    /// `elapsed_seconds` is how long ago the event happened (its sign is ignored).
    ///
    /// ```
    /// use karbon_ui::intl::Locale;
    ///
    /// assert_eq!(Locale::English.relative_time(8 * 60), "8 minutes ago");
    /// assert_eq!(Locale::French.relative_time(86_400), "hier");
    /// assert_eq!(Locale::German.relative_time(2 * 3600), "vor 2 Stunden");
    /// ```
    pub fn relative_time(self, elapsed_seconds: i64) -> String {
        let data = self.data();
        let s = elapsed_seconds.unsigned_abs() as f64;
        let (n, unit) = if s < 60. {
            return data.now.into();
        } else if s < 3600. {
            ((s / 60.).round(), 0)
        } else if s < 86_400. {
            ((s / 3600.).round(), 1)
        } else if s < 86_400. * 30. {
            let days = (s / 86_400.).round();
            if days == 1. {
                return data.yesterday.into();
            }
            (days, 2)
        } else if s < 86_400. * 365. {
            ((s / (86_400. * 30.)).round(), 3)
        } else {
            ((s / (86_400. * 365.)).round(), 4)
        };
        let n = n as i64;
        let (one, other) = data.units[unit];
        let word = if self.plural(n) == Plural::One {
            one
        } else {
            other
        };
        data.ago
            .replace("{}", &format!("{} {word}", self.number(n)))
    }
    /// Short form of [`relative_time`](Self::relative_time) for tight places: `just now`,
    /// `3 min ago`, `2 h ago`, `4 d ago`.
    pub fn relative_time_short(self, elapsed_seconds: i64) -> String {
        let data = self.data();
        let s = elapsed_seconds.unsigned_abs();
        let [min, hour, day] = data.short_units;
        let part = match s {
            0..60 => return data.just_now.into(),
            60..3600 => format!("{} {min}", s / 60),
            3600..86_400 => format!("{} {hour}", s / 3600),
            _ => format!("{} {day}", self.number((s / 86_400) as i64)),
        };
        data.ago.replace("{}", &part)
    }
    /// Grammatical number of `count` in this language, to pick the singular or plural
    /// translation of a word (French treats 0 as singular).
    ///
    /// ```
    /// use karbon_ui::intl::{Locale, Plural};
    ///
    /// assert_eq!(Locale::French.plural(0), Plural::One);
    /// assert_eq!(Locale::English.plural(0), Plural::Other);
    /// ```
    pub fn plural(self, count: i64) -> Plural {
        match count.unsigned_abs() {
            1 => Plural::One,
            0 if self.data().zero_is_singular => Plural::One,
            _ => Plural::Other,
        }
    }
}

/// Horloge `m:ss` ou `h:mm:ss` ; `pad` : minutes sur deux chiffres sous l'heure.
fn clock(seconds: f64, pad: bool) -> String {
    let s = if seconds.is_finite() {
        seconds.max(0.) as u64
    } else {
        0
    };
    let (h, m, s) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else if pad {
        format!("{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}
