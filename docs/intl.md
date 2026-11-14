# Nombres, durées et dates selon la langue (`karbon_ui::intl`)

L'équivalent de l'API `Intl` du navigateur pour les cinq langues intégrées
(anglais, français, allemand, espagnol, italien) : séparateurs des nombres, durées,
temps écoulé, dates à l'heure locale et pluriel. Sans crate tierce : l'heure locale
vient du système (`SystemTimeToTzSpecificLocalTime` sous Windows, `localtime_r` sous
macOS et Linux, UTC ailleurs).

## Utilisation

L'application fixe la langue une fois, au lancement puis à chaque changement de
réglage ; les fonctions libres l'utilisent.

```rust
use karbon_ui::intl::{self, Locale};

intl::set_locale(Locale::parse("fr-FR")); // « fr », « de_DE.UTF-8 »… ; inconnue : anglais

intl::number(9440);                 // "9 440"   (en : "9,440", de : "9.440", es : "9440")
intl::decimal(44.1, 1);             // "44,1"    (en : "44.1") ; zéros finaux retirés
intl::clock(482.);                  // "8:02"    (au-delà d'une heure : "1:02:03")
intl::clock_padded(482.);           // "08:02"
intl::duration(3. * 3600. + 300.);  // "3 h 05 min" (en : "3 h 05 min", de : "3 Std. 05 Min.")
intl::relative_time(8 * 60);        // "il y a 8 minutes" ; "hier" ; "maintenant"
intl::relative_time_short(3 * 60);  // "il y a 3 min" ; "à l’instant"
intl::date_time(SystemTime::now()); // "7 nov. 2026, 04:20" (en : "Nov 7, 2026, 4:20 AM")
intl::plural(n);                    // Plural::One / Plural::Other
```

Chaque fonction existe aussi en méthode de `Locale` pour une langue explicite
(`Locale::German.number(9440)`), sans toucher à la langue courante : c'est la forme à
employer dans les tests.

`relative_time` prend des secondes écoulées (le signe est ignoré) ;
`intl::seconds_since(instant)` les calcule depuis un `SystemTime`. `DateTime::local` et
`DateTime::utc` donnent la date du calendrier, `Locale::format_date_time` la met en forme.

## Ce qui reste à l'application

Les mots de l'application (« 12 titres ») restent dans ses traductions ; `plural`
choisit la forme. En français, 0 est au singulier (« 0 titre »), comme dans CLDR.

```rust
let key = if intl::plural(n) == Plural::One { "tracks_one" } else { "tracks_n" };
tr(key).replace("{n}", &intl::number(n))
```

## Limites

- Cinq langues ; une autre donne l'anglais. Les formats suivent les styles « medium »
  (date) et « short » (heure) de CLDR, en anglais ceux des États-Unis (AM/PM).
- Le temps relatif est toujours au passé (« il y a … ») et arrondi comme
  `Intl.RelativeTimeFormat` : minutes sous l'heure, heures sous le jour, jours sous
  30 jours, mois sous l'année, puis années.
- Pas de devises, de pourcentages ni d'unités de taille : `decimal` et les traductions
  de l'application les composent.
