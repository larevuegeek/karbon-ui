//! Formats de chaque langue, comparés à ceux d'`Intl` dans un navigateur.
use super::*;

#[test]
fn numbers_follow_each_language() {
    let cases = [
        (Locale::English, "9,440", "1,234,567", "-12,000", "44.1"),
        (
            Locale::French,
            "9\u{202f}440",
            "1\u{202f}234\u{202f}567",
            "-12\u{202f}000",
            "44,1",
        ),
        (Locale::German, "9.440", "1.234.567", "-12.000", "44,1"),
        // L'espagnol ne groupe qu'à partir de cinq chiffres.
        (Locale::Spanish, "9440", "1.234.567", "-12.000", "44,1"),
        (Locale::Italian, "9.440", "1.234.567", "-12.000", "44,1"),
    ];
    for (locale, small, big, negative, decimal) in cases {
        assert_eq!(locale.number(9440), small, "{locale:?}");
        assert_eq!(locale.number(1_234_567), big, "{locale:?}");
        assert_eq!(locale.number(-12_000), negative, "{locale:?}");
        assert_eq!(locale.decimal(44.1, 1), decimal, "{locale:?}");
        assert_eq!(locale.number(602), "602");
        assert_eq!(locale.decimal(96., 1), "96");
    }
    assert_eq!(Locale::English.decimal(-0.04, 1), "0");
}

#[test]
fn clocks_and_durations() {
    let l = Locale::English;
    assert_eq!(l.clock(482.83), "8:02");
    assert_eq!(l.clock_padded(482.83), "08:02");
    assert_eq!(l.clock(3723.), "1:02:03");
    assert_eq!(l.clock(f64::NAN), "0:00");
    assert_eq!(
        Locale::French.duration(3. * 3600. + 13. * 60.),
        "3 h 13 min"
    );
    assert_eq!(Locale::French.duration(42. * 60.), "42 min");
    assert_eq!(
        Locale::French.duration(25. * 86_400. + 6. * 3600.),
        "25 j 6 h"
    );
    assert_eq!(
        Locale::English.duration(25. * 86_400. + 6. * 3600.),
        "25 d 6 h"
    );
    assert_eq!(
        Locale::German.duration(3. * 3600. + 5. * 60.),
        "3 Std. 05 Min."
    );
    assert_eq!(Locale::Italian.duration(2. * 86_400.), "2 g 0 h");
}

#[test]
fn relative_times_in_each_language() {
    let minutes = 8 * 60;
    let cases = [
        (
            Locale::English,
            "now",
            "8 minutes ago",
            "1 hour ago",
            "yesterday",
            "3 months ago",
        ),
        (
            Locale::French,
            "maintenant",
            "il y a 8 minutes",
            "il y a 1 heure",
            "hier",
            "il y a 3 mois",
        ),
        (
            Locale::German,
            "jetzt",
            "vor 8 Minuten",
            "vor 1 Stunde",
            "gestern",
            "vor 3 Monaten",
        ),
        (
            Locale::Spanish,
            "ahora",
            "hace 8 minutos",
            "hace 1 hora",
            "ayer",
            "hace 3 meses",
        ),
        (
            Locale::Italian,
            "ora",
            "8 minuti fa",
            "1 ora fa",
            "ieri",
            "3 mesi fa",
        ),
    ];
    for (locale, now, mins, hour, yesterday, months) in cases {
        assert_eq!(locale.relative_time(20), now);
        assert_eq!(locale.relative_time(minutes), mins);
        assert_eq!(locale.relative_time(3600), hour);
        assert_eq!(locale.relative_time(86_400), yesterday);
        assert_eq!(locale.relative_time(90 * 86_400), months);
    }
    assert_eq!(Locale::French.relative_time(400 * 86_400), "il y a 1 an");
    assert_eq!(Locale::English.relative_time(-minutes), "8 minutes ago");
}

#[test]
fn short_relative_times() {
    assert_eq!(Locale::French.relative_time_short(30), "à l’instant");
    assert_eq!(Locale::French.relative_time_short(3 * 60), "il y a 3 min");
    assert_eq!(Locale::French.relative_time_short(4 * 86_400), "il y a 4 j");
    assert_eq!(Locale::English.relative_time_short(2 * 3600), "2 h ago");
    assert_eq!(Locale::German.relative_time_short(3 * 60), "vor 3 Min.");
    assert_eq!(Locale::Italian.relative_time_short(4 * 86_400), "4 g fa");
}

#[test]
fn dates_in_each_language() {
    let date = DateTime {
        year: 2026,
        month: 11,
        day: 7,
        hour: 16,
        minute: 5,
    };
    let cases = [
        (Locale::English, "Nov 7, 2026, 4:05 PM"),
        (Locale::French, "7 nov. 2026, 16:05"),
        (Locale::German, "07.11.2026, 16:05"),
        (Locale::Spanish, "7 nov 2026, 16:05"),
        (Locale::Italian, "7 nov 2026, 16:05"),
    ];
    for (locale, expected) in cases {
        assert_eq!(locale.format_date_time(date), expected);
    }
    let midnight = DateTime { hour: 0, ..date };
    assert_eq!(
        Locale::English.format_date_time(midnight),
        "Nov 7, 2026, 12:05 AM"
    );
}

#[test]
fn plurals_and_tags() {
    assert_eq!(Locale::French.plural(1), Plural::One);
    assert_eq!(Locale::French.plural(2), Plural::Other);
    assert_eq!(Locale::German.plural(1), Plural::One);
    assert_eq!(Locale::German.plural(0), Plural::Other);
    for locale in Locale::ALL {
        assert_eq!(Locale::parse(locale.tag()), locale);
    }
    assert_eq!(Locale::parse(""), Locale::English);
}

#[test]
fn the_local_time_is_a_valid_date() {
    let now = DateTime::local(std::time::SystemTime::now());
    assert!((1..=12).contains(&now.month) && (1..=31).contains(&now.day));
    assert!(now.hour < 24 && now.minute < 60 && now.year >= 2024);
}
