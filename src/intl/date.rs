//! Dates à l'heure locale : conversion par le système (règles d'heure d'été comprises),
//! puis mise en forme courte de chaque langue.
use super::Locale;
use std::time::{SystemTime, UNIX_EPOCH};

/// Calendar date and time of day, as shown to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateTime {
    /// Year (`2026`).
    pub year: i32,
    /// Month, `1..=12`.
    pub month: u8,
    /// Day of the month, `1..=31`.
    pub day: u8,
    /// Hour, `0..=23`.
    pub hour: u8,
    /// Minute, `0..=59`.
    pub minute: u8,
}

impl DateTime {
    /// `time` in Coordinated Universal Time.
    pub fn utc(time: SystemTime) -> Self {
        Self::from_unix(unix_seconds(time))
    }
    /// `time` in the computer's time zone, daylight saving time included (UTC on
    /// platforms without time zone support).
    pub fn local(time: SystemTime) -> Self {
        let seconds = unix_seconds(time);
        local(seconds).unwrap_or_else(|| Self::from_unix(seconds))
    }
    fn from_unix(seconds: i64) -> Self {
        let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
        let (year, month, day) = civil_from_days(days);
        Self {
            year: year as i32,
            month,
            day,
            hour: (rest / 3600) as u8,
            minute: ((rest % 3600) / 60) as u8,
        }
    }
}

impl Locale {
    /// Date and time in the computer's time zone, in the locale's medium date and short
    /// time styles: `Nov 7, 2026, 4:20 AM`, `7 nov. 2026, 04:20`, `07.11.2026, 04:20`.
    pub fn date_time(self, time: SystemTime) -> String {
        self.format_date_time(DateTime::local(time))
    }
    /// [`date_time`](Self::date_time) of a date already converted.
    ///
    /// ```
    /// use karbon_ui::intl::{DateTime, Locale};
    ///
    /// let date = DateTime { year: 2026, month: 11, day: 7, hour: 4, minute: 20 };
    /// assert_eq!(Locale::French.format_date_time(date), "7 nov. 2026, 04:20");
    /// assert_eq!(Locale::English.format_date_time(date), "Nov 7, 2026, 4:20 AM");
    /// ```
    pub fn format_date_time(self, d: DateTime) -> String {
        let month = self.data().months[usize::from(d.month.clamp(1, 12) - 1)];
        let (day, year, hour, minute) = (d.day, d.year, d.hour, d.minute);
        match self {
            Self::English => {
                let (h12, half) = match hour {
                    0 => (12, "AM"),
                    1..=11 => (hour, "AM"),
                    12 => (12, "PM"),
                    _ => (hour - 12, "PM"),
                };
                format!("{month} {day}, {year}, {h12}:{minute:02} {half}")
            }
            Self::German => format!("{day:02}.{:02}.{year}, {hour:02}:{minute:02}", d.month),
            Self::Spanish => format!("{day} {month} {year}, {hour}:{minute:02}"),
            Self::French | Self::Italian => {
                format!("{day} {month} {year}, {hour:02}:{minute:02}")
            }
        }
    }
}

/// Secondes depuis l'époque Unix, négatives avant 1970.
fn unix_seconds(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// Jours depuis 1970-01-01 → (année, mois, jour) du calendrier grégorien proleptique
/// (algorithme de Howard Hinnant).
fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(windows)]
fn local(seconds: i64) -> Option<DateTime> {
    use windows_sys::Win32::{
        Foundation::SYSTEMTIME, System::Time::SystemTimeToTzSpecificLocalTime,
    };
    let utc = DateTime::from_unix(seconds);
    let rest = seconds.rem_euclid(60);
    let universal = SYSTEMTIME {
        wYear: u16::try_from(utc.year).ok()?,
        wMonth: u16::from(utc.month),
        wDayOfWeek: 0,
        wDay: u16::from(utc.day),
        wHour: u16::from(utc.hour),
        wMinute: u16::from(utc.minute),
        wSecond: rest as u16,
        wMilliseconds: 0,
    };
    let mut local = universal;
    // SAFETY : deux SYSTEMTIME valides sur la pile ; fuseau nul = fuseau actif du système.
    let ok = unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &universal, &mut local) };
    (ok != 0).then(|| DateTime {
        year: i32::from(local.wYear),
        month: local.wMonth as u8,
        day: local.wDay as u8,
        hour: local.wHour as u8,
        minute: local.wMinute as u8,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn local(seconds: i64) -> Option<DateTime> {
    let time = libc::time_t::try_from(seconds).ok()?;
    // SAFETY : `tm` est entièrement écrit par `localtime_r` quand il réussit.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::localtime_r(&time, &mut tm) };
    (!result.is_null()).then(|| DateTime {
        year: tm.tm_year + 1900,
        month: (tm.tm_mon + 1) as u8,
        day: tm.tm_mday as u8,
        hour: tm.tm_hour as u8,
        minute: tm.tm_min as u8,
    })
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn local(_seconds: i64) -> Option<DateTime> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unix_seconds_become_calendar_dates() {
        let at = |s: i64| DateTime::from_unix(s);
        assert_eq!(
            at(0),
            DateTime {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0
            }
        );
        // 2026-11-07 04:20 UTC.
        assert_eq!(
            at(1_794_025_200),
            DateTime {
                year: 2026,
                month: 11,
                day: 7,
                hour: 4,
                minute: 20
            }
        );
        // 29 février d'une année bissextile, et la veille de 1970.
        assert_eq!((at(951_782_400).month, at(951_782_400).day), (2, 29));
        assert_eq!(at(-60).year, 1969);
    }
}
