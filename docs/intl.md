# Traductions, nombres, tailles, durées et dates selon la langue (`karbon_ui::intl`)

L'équivalent de l'API `Intl` du navigateur et d'un `$t` d'i18next pour les cinq langues
intégrées (anglais, français, allemand, espagnol, italien) : traductions de
l'application, séparateurs des nombres, tailles de fichier, durées, temps écoulé, dates
à l'heure locale et pluriel. Sans crate tierce : l'heure locale vient du système
(`SystemTimeToTzSpecificLocalTime` sous Windows, `localtime_r` sous macOS et Linux, UTC
ailleurs), les fichiers JSON sont lus par le lecteur interne de karbon-ui.

## Utilisation

L'application fixe la langue une fois, au lancement puis à chaque changement de
réglage ; les fonctions libres l'utilisent.

```rust
use karbon_ui::intl::{self, Locale};

intl::set_locale(Locale::parse("fr-FR")); // « fr », « de_DE.UTF-8 »… ; inconnue : anglais

intl::number(9440);                 // "9 440"   (en : "9,440", de : "9.440", es : "9440")
intl::decimal(44.1, 1);             // "44,1"    (en : "44.1") ; zéros finaux retirés
intl::bytes(1_572_864);             // "1,5 Mo"  (en : "1.5 MB") ; "700 Mo", "512 o"
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

`bytes` compte en multiples de 1 024, comme les gestionnaires de fichiers : une décimale
sous 10, des entiers au-dessus (« 9,5 Ko », « 12 Mo », « 1 To »). Unités : `o`, `Ko`,
`Mo`, `Go`, `To` en français, `B`, `KB`, `MB`, `GB`, `TB` ailleurs.

## Traductions

Les textes de l'application vivent dans un fichier JSON par langue, au format
d'i18next ou de svelte-i18n : des objets imbriqués dont les clés se joignent par des
points.

```json
{ "nav": { "home": "Accueil" }, "home": { "track_one": "{n} titre", "tracks_n": "{n} titres" } }
```

```rust
use karbon_ui::intl::{self, Catalog, Locale};

let mut catalog = Catalog::new();
catalog.add(Locale::French, include_str!("locales/fr.json"))?; // la première : langue de repli
catalog.add(Locale::English, include_str!("locales/en.json"))?;
intl::set_catalog(catalog);
intl::set_locale(Locale::French);

intl::t("nav.home");                                    // "Accueil"
intl::t_with("album.by", &[("a", "Björk")]);            // "{a}" remplacé
intl::t_count(9440, "home.track_one", "home.tracks_n"); // "9 440 titres"
```

- Une clé absente dans la langue courante est cherchée dans la langue de repli (la
  première ajoutée, ou celle de `Catalog::set_fallback`), puis rendue telle quelle :
  `t("cle.inconnue")` donne `"cle.inconnue"`, comme `$t`. Sans catalogue installé, `t`
  rend la clé.
- `t_with` remplace chaque `{nom}` par sa valeur ; `t_count` choisit la forme selon
  `plural` (en français, 0 est au singulier, comme dans CLDR) et remplace `{n}` par le
  nombre mis en forme.
- Seules les chaînes sont retenues : nombres, booléens, `null` et tableaux sont ignorés.
  Un texte qui n'est pas un objet JSON rend `Error::InvalidInput` et ne change rien.
- Le catalogue est celui du processus, comme la langue ; `Catalog::get` et
  `Catalog::text` servent une langue explicite (tests).
- Les gabarits appellent `t` comme le reste du code : `{t("nav.home")}`, avec
  `use karbon_ui::intl::t;` dans le fichier Rust qui inclut le gabarit.

## Limites

- Cinq langues ; une autre donne l'anglais. Les formats suivent les styles « medium »
  (date) et « short » (heure) de CLDR, en anglais ceux des États-Unis (AM/PM).
- Le temps relatif est toujours au passé (« il y a … ») et arrondi comme
  `Intl.RelativeTimeFormat` : minutes sous l'heure, heures sous le jour, jours sous
  30 jours, mois sous l'année, puis années.
- Deux formes de pluriel (`one`, `other`), suffisantes pour les cinq langues ; pas de
  devises ni de pourcentages : `decimal` et les traductions de l'application les
  composent.
