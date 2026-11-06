# RustMusic en templates HTML

Deuxième reproduction de l’accueil de RustMusic, cette fois **traduite depuis les
sources Svelte + Tailwind** de l’application (`src/routes/+layout.svelte`,
`+page.svelte`, `src/lib/components/**`, `src/app.css`) vers la syntaxe HTML
compilée (`.ui` + CSS). Elle remplace l’ancien exemple natif `rustmusic`, construit
avec l’API Rust à partir d’une capture ([archive](archive/rustmusic-native.md)).

Données statiques, aucune lecture audio : le but est de comparer le rendu et les
ressources avec l’application Tauri, pas de refaire sa logique.

```powershell
cargo run --release --example rustmusic_html
cargo run --release --example rustmusic_html -- --renderer=gpu --present=composition
```

Fenêtre de 1440 × 1040 unités logiques (minimum 1100 × 760), barre de titre
personnalisée (Réduire / Agrandir / Fermer câblés, glisser non câblé). `--size LxH`
change la taille ; `--renderer`, `--present`, `--profile`… passent à
`WindowOptions::from_args`.

## Aperçus

```powershell
cargo run --release --example rustmusic_html -- --export artifacts/rustmusic-html-1280.png --size 1280x900 --renderer=software
cargo run --release --example rustmusic_html -- --export artifacts/rustmusic-html-gpu.png --size 1280x900 --renderer=gpu
cargo run --release --example rustmusic_html -- --export artifacts/rustmusic-html-full.png --size 1440x2700 --scale 1
cargo run --release --example rustmusic_html -- --open profile --export artifacts/rustmusic-html-menu-profile.png --size 1280x900
cargo run --release --example rustmusic_html -- --hover 433,676 --export survol.png --size 1280x900
cargo run --release --example rustmusic_html -- --sidebar collapsed --open library --export artifacts/rustmusic-html-collapsed-library-1280x900.png --size 1280x900
```

`--open sleep|profile|source|open|library|display|mix|output|queue|lyrics|network|whatsnew|search`
ouvre un menu avant l’export ou le `--dump` (`search` tape « que ») ; `--hover X,Y`
pose le pointeur (unités logiques) pour rendre les styles `:hover`
(`BenchmarkOptions::pointer`) ; `--sidebar collapsed|expanded` replie la barre
latérale (réglage `sidebar_collapsed`, dépliée par défaut). Les options se
combinent entre elles et avec `--size`, `--scale`, `--export`, `--dump`, et
s’appliquent aussi à la fenêtre.

L’export rend à l’échelle 1,5 par défaut (`--scale`), celle des captures de
référence de l’application Tauri. Les px CSS de Tailwind sont des unités logiques,
sans facteur d’échelle dans le code.

Les captures se produisent localement dans `artifacts/`, qui n’est pas suivi par git
(volumineux, et les planches de comparaison reproduisent des captures de RustMusic
avec de vraies pochettes) :

- `rustmusic-html-<largeur>.png` à 960 × 700, 1100 × 760, 1280 × 900, 1440 × 1040 et
  1600 × 1000 (logiciel, WARP) ; `rustmusic-html-gpu.png` (1280 × 900 en GPU),
  `rustmusic-html.png` (1440 × 1040 par défaut), `rustmusic-html-full.png` (page
  entière 1440 × 2700 à l’échelle 1) ;
- barre repliée : `rustmusic-html-collapsed-<taille>.png`, et la liste des sources ou
  des bibliothèques ouverte à droite de son bouton
  (`rustmusic-html-collapsed-source-1280x900.png`, `…-library-…`). Pas de capture
  Tauri de cet état ;
- menus ouverts (1280 × 900) : `rustmusic-html-menu-<nom>.png`, et la planche
  référence / Karbon de chacun : `rustmusic-html-menu-<nom>-compare.png` (référence à
  gauche ou en haut) ; survols : `rustmusic-html-hover-<nom>-compare.png` (`mix`,
  `nav`, `playlist`, `queue-row`).

### État reproduit

L’état est **celui des captures de référence** de l’application Tauri
(2026-11-03) : « Stairway To Heaven » en pause dans le lecteur, sa file (Hornbeam,
The National Anthem, Strategie D'un Pion, Andy Warhol…), les quatre mix du jour,
les playlists de la barre latérale, thème sombre **en contraste élevé**
(`<html class="dark" data-contrast="high">` : textes `--sb-mu #a3aca7`,
`--lc-mu #b0b9b4`, voile des mix noir, accents blancs). Les pochettes viennent du
cache de RustMusic (`%APPDATA%/com.larevuegeek.rustmusic/covers`, lu sans écriture)
par [`tools/prepare-covers.py`](../examples/rustmusic_html/tools/prepare-covers.py) ;
elles ont été identifiées en comparant les captures aux 1 166 pochettes du cache.
Les sections sous la ligne de flottaison (playlists, genres, ajouts récents,
récemment joués) gardent les pochettes PNG reprises de l’ancien exemple natif
(voir `examples/rustmusic_html/assets/SOURCES.md`).

## Comparer avec l’application Tauri

```powershell
cargo build --release --example rustmusic_html
pwsh examples/rustmusic_html/tools/fidelity.ps1 -Reference <dossier des captures> [-Out target/rustmusic-fidelity]
python -I examples/rustmusic_html/tools/compare.py REF.json karbon.json --ref-png REF.png --png karbon.png --crop 0,760,640,140
```

- `--dump fichier.json` (exemple) écrit chaque élément mis en page : bornes logiques,
  classes, texte (et texte des descendants d’un conteneur), police calculée,
  `letter-spacing`, couleurs, rayon et bordure, au format des captures CDP de Tauri.
  Il s’appuie sur `diagnostics::inspect_preview` (mise en page hors écran, texte
  mesuré par DirectWrite comme l’export) et `Element::classes` /
  `Element::computed_style`.
- `compare.py` apparie les éléments par texte (casse et espaces ignorés, préfixe pour
  les textes tronqués ; le plus proche en position et en taille) et les boîtes sans
  texte par la table `CHROME` (fragment du chemin de la référence ; côté Karbon, la
  liste de classes qui commence par celles du fragment, ou une classe nommée dans la
  table). Il liste les écarts
  de position, de taille, de police, d’espacement et de rayon, du pire au meilleur, et
  fait des planches référence / Karbon (`--crop x,y,l,h` en unités logiques).
- `fidelity.ps1` enchaîne export, dump et comparaison aux cinq tailles.
- `menus.ps1` exporte chaque menu (`--open`) à 1280 × 900 et le compare à sa capture
  (`compare.py --only-new home.json --menu <nom>` : seuls les éléments absents de la
  page sans menu, et les boîtes de la table `MENU_CHROME` propres à ce menu), avec une
  planche par menu.

### Fidélité mesurée (2026-11-03)

Écarts des éléments appariés, en unités logiques (pire de dx, dy, dh, et dw pour une
boîte). Avant : `@media` décalées, barres de défilement contournées ; après :
`@container`, barres fines, icônes SVG exactes.

| Taille | Appariés | ≤ 2 px avant → après | Médiane | Écart max avant → après | Pires écarts restants |
| --- | --- | --- | --- | --- | --- |
| 960 × 700 | 91 / 91 | 97 % → 98 % | 0,7 | 27,7 → 2,7 | barre du hero 2,7 (temps en JetBrains Mono), bouton rond 2,4 |
| 1100 × 760 | 120 / 120 | 96 % → 97 % | 0,7 | 19,7 → 2,9 | « Ctrl K » 2,9 (Consolas), barre du hero 2,7 |
| 1280 × 900 | 147 / 147 | 96 % → 97 % | 0,7 | 19,7 → 2,9 | « Ctrl K » 2,9, nom du fichier 2,9, pastille 2,4 |
| 1440 × 1040 | 151 / 151 | 97 % → 98 % | 0,7 | 19,7 → 2,7 | barre du hero 2,7, nom du fichier 2,6 |
| 1600 × 1000 | 151 / 151 | 97 % → 98 % | 0,7 | 19,7 → 2,7 | barre du hero 2,7, nom du fichier 2,6 |

Les écarts max d’avant (19,7 / 27,7) étaient la liste de la barre latérale, élargie
sous le bord rogné ; elle a maintenant sa largeur de Tauri.

Depuis, le moteur arrondit les bordures au pixel physique comme Chromium (1 px CSS =
0,67 px logique à 150 %). Le décalage vertical de la barre latérale et la ligne
« Led Zeppelin · … · 1971 » du hero, qui finissait par « 19… », ont disparu :

| Taille | ≤ 2 px | Médiane | Max |
| --- | --- | --- | --- |
| 960 × 700 | 100 % | 0,0 | 2,0 |
| 1100 × 760 | 100 % | 0,0 | 2,0 |
| 1280 × 900 | 100 % | 0,0 | 2,0 |
| 1440 × 1040 | 99 % | 0,0 | 2,6 |
| 1600 × 1000 | 99 % | 0,0 | 2,6 |

Le thème reprend aussi les gris du contraste élevé sombre (`--neutral-300` à `500`) :
flèches Retour / Suivant, icônes des menus et boutons de fenêtre ont la teinte de
Tauri.

## Correspondance avec les sources Svelte

Un `.ui` par composant Svelte, au même chemin relatif (le préfixe
`lib/components/ui/` est aplati). La page entière est un seul `view()` fait
d’`{@include}`, comme les composants Svelte imbriqués.

| Svelte | Traduction |
| --- | --- |
| `routes/+layout.svelte` | `ui/layout.ui` ; barre latérale, page et lecteur par `{@include}` |
| `routes/+page.svelte` | `ui/page.ui` |
| `ui/titlebar/Titlebar.svelte` | composant Rust `Titlebar` + `ui/titlebar/Titlebar.ui` (slot = `children`) |
| `ui/logo/LogoRustMusic.svelte` | `ui/logo/LogoRustMusic.ui` (le SVG refait en boîtes positionnées) |
| `search/SearchAutocomplete`, `player/SleepTimerButton`, `whatsnew/WhatsNewButton`, `ui/input/SwitchTheme`, `header/ProfilSelectorInput` | includes du même nom |
| `sidebar/Sidebar`, `SourceSwitcher`, `SidebarNav`, `SidebarFooter`, `library/common/LibrarySelector` | includes du même nom ; listes des sources et des bibliothèques dans `SourceMenu.ui` / `LibraryMenu.ui`, incluses par les deux placements (dépliée, repliée) |
| `sidebar/SidebarTitle`, `SidebarRow`, `SidebarButton`, `SidebarThumbnail`, `PlaylistItem` | composants Rust ; les `{#snippet}` deviennent des slots nommés (`actions`, `vignette`) |
| `home/HomeHeader`, `HomeResume`, `HomeMixes`, `HomePlaylists`, `HomeGenres`, `HomeRecentlyAdded`, `HomeRecents` | includes du même nom (cartes de la file en `{#each}` dans HomeResume, comme en Svelte) |
| `mix/MixCard`, `playlist/PlaylistCard` | composants Rust (variante affichée seulement) |
| `ui/carousel/Carousel` | balisage inline dans `HomePlaylists.ui` |
| `player/Player`, `PlayerProgressBar`, `PlayerSoundBar`, `NetworkMenu` | includes du même nom (menus compris) |
| `header/ProfilSelectorInput`, `player/SleepTimerButton`, `sidebar/SourceSwitcher`, `library/common/LibrarySelector` | includes du même nom, menus compris |
| `ui/contextmenu/OpenZoneMenu` + `LibraryTabsOptions`, `PlaylistZoneMenu` | `ui/sidebar/OpenZoneMenu.ui`, `PlaylistZoneMenu.ui` |
| `ui/menu/Menu` + `MenuItem` (actions d’un mix) | dans `ui/mix/MixCard.ui` |
| `ui/text/Badge`, `ui/input/ToggleSwitch` | classes `.badge`, `.interrupteur` de `menus.css` |

Non traduits : `HomeWelcome` / `HomeRemoteWelcome` (autres états de l’accueil), la
rangée des décennies de HomeMixes (sous le lecteur à toutes les tailles), les panneaux
file d’attente (`queue/QueuePanel`) et paroles (`lyrics/LyricsPanel`) — leurs boutons
du lecteur basculent seulement leur état —, la popin des nouveautés
(`whatsnew/WhatsNewPopin`), les sous-menus (gestion d’une bibliothèque,
`LibraryManageMenu`), les toasts.

| Fichier | Rôle |
| --- | --- |
| `rustmusic_html.rs` | Arguments, export PNG, `--dump`, fenêtre |
| `rustmusic_html/mod.rs` | `Msg`, état (menus et choix compris), `view()` |
| `rustmusic_html/menus.rs` | `Menu` (menu ouvert, noms de `--open`), `Veille`, résultats de recherche et libellés calculés |
| `rustmusic_html/components.rs` | `Icon` et les composants réutilisés |
| `rustmusic_html/icons.rs`, `icons/<jeu>/<nom>.svg` | Généré par `tools/icons.py` : nom Iconify → `<svg src>` compilé |
| `rustmusic_html/color.rs` | Dégradés par élément (genres, playlists), `hue()`, `oklch()` |
| `rustmusic_html/data/` | Données statiques par zone de l’écran : `home.rs` (accueil), `sidebar.rs` (barre latérale, bibliothèques, sources), `player.rs` (lecteur, sorties), `header.rs` (profil, index de recherche) |
| `rustmusic_html/dump.rs` | Mise en page en JSON (`--dump`) |
| `rustmusic_html/assets.rs`, `assets/` | Polices (`fonts/`), pochettes (`covers/`) ; sources et licences dans `assets/SOURCES.md` |
| `rustmusic_html/styles/*.css` | `theme.css` (jetons), `layout.css`, `sidebar.css`, `home.css`, `player.css`, `menus.css` |
| `rustmusic_html/tools/` | `icons.py`, `prepare-covers.py`, `compare.py`, `fidelity.ps1`, `menus.ps1` |
| `rustmusic_html/tests.rs` | Construction à deux tailles, textes, états, icônes générées, menus et leurs choix, suggestions, couleurs ; barre repliée (bascule, largeurs, éléments présents ou absents, noms et infobulles, menus à droite de la colonne, largeur animée) |

## Traduction Tailwind → CSS

Depuis que le compilateur comprend [Tailwind](tailwind.md), `styles/app.css` importe
`tailwindcss` (avec le `@theme` de RustMusic, les gris du contraste élevé et
`@custom-variant dark (&)`) et **tous les templates reprennent les classes Tailwind de
leur composant Svelte**, copiées dans l’ordre de la source (`compare.py` apparie les
boîtes par le début de leur liste de classes, comme les chemins des captures).

- Classes conditionnelles du Svelte (`{cond ? 'a' : 'b'}`, `class:x`) : `class:nom={…}`
  par jeton ; un jeton qui contient `( ) [ ] / #` (interdit dans un nom `class:`) passe
  par une branche `{#if}` par état, le contenu commun dans un include
  (`SourceOption.ui`, `LibraryInfos.ui`, `LibraryManage.ui`, `SortieLigne.ui`,
  `NetworkRow.ui`, `SearchResult.ui`, `HomeRecentsAlbum.ui`).
- `<span>`/`<p>`/`<a>` de texte : `<div>` quand ils sont voisins (deux éléments en ligne
  voisins forment un seul paragraphe), `<span>` gardé seul dans son conteneur ; un
  libellé de `<button>` texte seul est mis dans un `<div>` (sinon bouton natif).
- Restent écrites à la main les classes propres d’`app.css` de RustMusic et des blocs
  `<style>` : `.sidebar`, `.smart-scroll`, `.sb-liste-fondu`, `.sb-vignette`,
  `.sb-pied-carte`, `.sb-avatar`, `.sb-point` (sidebar.css), `.home-maquette`,
  `.scrollbar-app`, `.hero-reprise`, `.hero-voile`, `.ensuite-panneau`, `.mix-voile`,
  `.mix-accent`, `.accent-teinte`, `.genre-*`, `.recent-vedette`, `.vedette-voile`
  (home.css), `.lecteur`, `.lecteur-pochette`, `.lecteur-lecture`, `.lecteur-dossier`
  (player.css), `.rg-field` (menus.css), en valeurs du contraste élevé sombre ; les
  jetons (`--sb-*`, `--lc-*`, `--rg-*`) restent en `:root` (theme.css).
- Écarts gardés, chacun commenté dans son template : `.menu` (entrée des menus en fondu
  seul, police de base) et la police Figtree des menus du mix et du lecteur
  (`.mix-menu`, `.lecteur-menu`, une classe de la feuille l’emporte sur un utilitaire) ;
  la gouttière de 15 px de la liste de recherche (`.recherche-liste`) ; `w-auto` sur les
  libellés et `text-center` des boutons (feuille par défaut du navigateur), `-mt-*` et
  `mt-[9.5px]` à la place de `items-baseline` (ligne de base approchée), `aspect-square`
  à la place de `h-full` dans une cellule de grille, retrait du conteneur défilant de la
  page passé au `@container`, `rounded-md`/`rounded-2xl` pour `rounded-[inherit]`,
  `opacity-0` pour `invisible`, décalages de @4xl du bouton lecture des mix gardés
  partout, `font-(--font-mono)` pour la règle `.lecteur .font-mono` d’app.css,
  quelques `!` pour l’ordre des états actifs du lecteur.
- Les commentaires `TODO-TW` signalent les jetons de la source encore refusés par le
  compilateur, avec son message : `md:relative`/`md:z-auto` et la translation de la
  barre latérale, `snap-*` du carrousel, `[&::-webkit-scrollbar]:hidden`,
  `wrap-anywhere hyphens-auto`, `right-2 bottom-2 @4xl:right-3.5` (décalages
  conditionnels), `cursor-zoom-in`, `invisible`, `align-middle`, `rounded-[inherit]`,
  `fill-*` du logo, la transition du halo de recherche (`box-shadow`).

### Conversion (2026-11-04)

Feuilles écrites à la main : **1 993 → 320 lignes** (`theme.css` 100 → 115 avec les
jetons des menus, `app.css` 40 → 45, `layout.css` 160 → 5, `sidebar.css` 292 → 40,
`home.css` 593 → 51, `player.css` 261 → 22, `menus.css` 547 → 42).

Vérification par export (déterministe : deux exports d’un même exécutable sont
identiques au bit près) de 26 états à chaque étape — accueil aux cinq tailles, page
entière 1440 × 2700, dix menus, quatre survols, barre repliée (1280 × 900, 960 × 700,
avec `--open source` et `--open library`) — et d’une centaine d’états supplémentaires
(survols, menus survolés, minuteur actif, variantes de l’état). Chaque zone a d’abord été
convertie au pixel près (valeurs arbitraires pour les couleurs mesurées), puis des
commits séparés ont rétabli les jetons de la source, vérifiés sur les captures Tauri :

| Changement voulu | Exports touchés | Face aux captures |
| --- | --- | --- |
| Couleurs Tailwind 4 des listes source / bibliothèque, chevrons `neutral-600` du contraste élevé | menus source, bibliothèque, « … » Ouvrir, options d’affichage ; barre repliée avec liste ouverte (écart max 23/255, 82 sur les chevrons) | « Nouvelle » (5,223,114) comme Tauri, chevrons (164,164,164) comme Tauri |
| Anneau du logo radio `ring-black/8 dark:ring-white/8` | accueil 1440, 1600, page entière (max 13) | non visible dans les captures |
| Panneau du minuteur en couleurs Tailwind 4, icônes `neutral-500` | menu minuteur | gris 150 comme Tauri (165 avant) |
| `--rg-mu`/`--rg-tx` du contraste élevé, flou `backdrop-blur-sm` du « … » des mix | menu et survol des mix (max 37) | icônes du menu (176,185,180) comme Tauri, erreur moyenne 9,6 → 7,6 ; survol 6,5 → 4,8 |
| Mise en page de la source sous la ligne de flottaison (description des playlists, flèche du carrousel, badge « Auto », icône des genres, `gap-6` des récents) | page entière seulement | aucune capture |
| Anneau `ring-1` peint au-dessus de l’ombre (ordre de Tailwind) | cartes des mix, flèche du carrousel (max 3) | — |
| Badge BÊTA de la sortie audio en `tracking-[0.08em]` hérité | menu sortie audio | 97 % → 100 % à moins de 2 px, max 3,3 → 0,9 px |
| Texte indicatif de la recherche en `dark:placeholder:text-[#8b948f]` (le moteur le peignait dans la couleur atténuée du thème) | tous les exports sans recherche tapée (max 13) | écart moyen aux pixels du texte 11,7 → 6,0 |

Barre repliée (sans capture Tauri), avant / après : aucun déplacement ; seuls changent
le texte indicatif de la recherche (max 13/255) et, liste ouverte, les couleurs
Tailwind 4 des listes (max 23/255).

Fidélité (`fidelity.ps1`, `menus.ps1`) : identique avant et après, sauf la sortie audio
(meilleure) ; voir les tableaux plus bas. Survols (écart moyen à la capture dans la
zone qui change) : mix 2,80 → 2,76, tuile de navigation 4,80, ligne de playlist 2,13,
ligne de la file 3,52 (inchangés).

`compare.py` apparie une boîte de la table `CHROME`/`MENU_CHROME` par la classe
sémantique d’avant ou par le début de la liste de classes de la source (celle du
chemin de la référence) ; les boîtes de menu sont propres à leur menu (`--menu`, passé
par `menus.ps1`) ; une boîte dont le texte est porté par un paragraphe anonyme est
appariée par l’élément qui porte le texte. Quelques entrées dont la liste de la source
ne peut pas être écrite telle quelle (positionnement des popovers) gardent un nom de
classe : `w-56`, `w-64`, `w-66`, `w-96`, `w-100`, `rg-field`, `favori`, `sb-pied-carte`.

### Requêtes de conteneur

`.home-container` porte `container-type: inline-size` (le `@container
max-w-[1400px]` du Svelte) ; son retrait `px-10`, qui défile avec la page, est sa
marge intérieure : sa boîte de contenu, interrogée par les requêtes, fait
`min(1400, largeur − 80)`. Chaque `@Nxl:` de Tailwind devient
`@container (width >= N rem)`. Règles de base = conteneur étroit (« mobile first »).

| Tailwind | `@container` | Effets |
| --- | --- | --- |
| `@xl` | `(width >= 36rem)` | mix sur 2 colonnes (cartes horizontales), genres sur 3 |
| `@2xl` | `(width >= 42rem)` | hero `160px 1fr` + panneau « Ensuite » dessous (en flex), pochette pleine largeur, file sur 2 colonnes |
| `@3xl` | `(width >= 48rem)` | en-tête sur deux colonnes (titre 34 px, chiffres et bouton à droite), descriptions de section, hero `176px 1fr 250px` (titre 32 px), album en bouton rond, ajouts récents sur 4, récents `340px 1fr` |
| `@4xl` | `(width >= 56rem)` | mix en 4 carrés (voile et libellés sur la pochette, bouton 48 px), carrousel 176 px, ajouts récents sur 6 |
| `@5xl` | `(width >= 64rem)` | hero `220px 1fr 260px`, `gap-8 p-8`, titre 38 px, méta 18 px, durées de la file |
| `@6xl` | `(width >= 72rem)` | nom des mix 26 px, genres sur 6, récents `400px 1fr`, 4 albums |
| `@7xl` | `(width >= 80rem)` | hero `260px 1fr 300px`, titre 44 px, méta 19 px, libellé « Lire l’album… », 5 albums |

Les points de rupture de fenêtre de Tailwind restent des `@media` : `max-lg`
(lecteur sur deux lignes, profil sans nom, pas de « Ctrl K »), `max-[1250px]`
(±10 s, pastille de sortie, volume de 88 px), `max-[1080px]` (arrêt, débit),
`max-md`.

### Barres de défilement

| Svelte (WebView2 lit `::-webkit-scrollbar`) | Traduction |
| --- | --- |
| Page : `.scrollbar-app` (10 px, pouce `rgba(255,255,255,.18)`) | `scrollbar-width: thin; scrollbar-color: rgba(255,255,255,.18) transparent` (gouttière de 10 px réservée seulement si la page déborde) |
| Barre latérale et liste des playlists : `smart-scroll` (8 px, pouce invisible hors survol) | `scrollbar-width: thin`, pouce transparent, `::-webkit-scrollbar { width: 8px }` |
| Carrousel : `[scrollbar-width:none]` | `scrollbar-width: none` |
| Bibliothèques, résultats de recherche : barre classique | `thin` (réservée au débordement), 15 px pour la recherche |

Retirés : le retrait droit de 38 px de la page, la liste de la barre latérale élargie
sous le bord rogné (marge −28 px) et le retrait de 8 px en `@media (max-height…)` (la
barre latérale défile maintenant au lieu de rogner).

## Barre latérale repliée

Le bouton de la barre de titre qui suit le logo (`Titlebar.svelte`) bascule
`MusicApp::sidebar_collapsed` (`Msg::ToggleSidebar`), traduction du réglage
`sidebar_collapsed` lu par `Sidebar.svelte` et ses composants (`replie`). Replié,
le bouton montre `left-panel-open-outline-rounded` et « Déplier la barre » ; déplié,
`left-panel-close-outline-rounded` et « Replier la barre ». `--sidebar collapsed`
exporte cet état.

| Svelte (`replie`) | Traduction |
| --- | --- |
| `Sidebar` : `w-19 items-center pt-3 pb-3` au lieu de `w-72 px-4 pt-4 pb-3`, `transition-[width] duration-200` | `.sidebar.replie` (76 px, `align-items: center`, `padding: 12px 0`), `transition: width 200ms cubic-bezier(0.4, 0, 0.2, 1)` sur `.sidebar` : la largeur est animée image par image, la zone principale et ses `@container` suivent |
| Haut `items-center` ; Fichier / Dossier et leur « … » masqués (`!replie`) | `.sidebar-top.replie` ; rangée et menu `Ouvrir` absents |
| `SourceSwitcher`, `LibrarySelector` : icône seule (`p-1.75`, `flex-none`), `title` | `.source-switcher.replie`, `.library-selector.replie` (52 × 52) ; `title` et nom accessible « Changer de source / de bibliothèque » |
| Liste des sources : `left: r.right + 8 ; top: r.top ; width: 300px` | `k-popover placement="right-start" offset="8"`, 300 px |
| Liste des bibliothèques : `left: r.right + 8 ; top: r.top ; width: 272px` | `placement="right-start" offset="8"`, `.biblio-menu-replie` (272 px) au lieu de `match-width` |
| `SidebarNav` : `grid-cols-[48px] gap-1.5 justify-center`, tuiles `w-12 h-12` sans libellé, `title` | `.sb-nav.replie` (grille d’une colonne de 48 px centrée par `justify-content`), `.sb-tile.replie` ; `title` et `aria-label` |
| Milieu : `flex-1 min-h-0 w-full items-center gap-2 py-2`, sans barre (`scrollbar-width: none`) ni fondu, sans `SidebarTitle` (Playlists, Écoute en ligne) ni « … » | `.sb-list-replie` / `.sb-list-replie-content` ; titres, boutons d’action et menu « Options d’affichage » absents |
| `SidebarRow` / `PlaylistItem` : `flex justify-center`, bouton `p-0`, vignette `w-11 h-11` (anneau `ring-2` si active), ni texte ni « … », `title` | prop `replie` : `.sb-row.replie`, `.sb-row-thumb-replie` (44 px), titre en `title` et `aria-label` ; logo radio en 44 px |
| `SidebarFooter` : colonne `items-center gap-1 pt-2.5 border-t`, boutons « grands » `w-12 h-12` (dossiers, rescan et son point `top-2.5 right-2.5` cerclé de `--sb-bg`), avatar `mt-1` | `.sb-footer-replie`, `.sb-footer-grand`, `.sb-footer-rescan-point`, `.sb-footer-profil` ; titres du Svelte (« Dossiers synchronisés », « Rescanner la bibliothèque — Scannée il y a 4 j », « David — Profil actif · changer ») |
| Mode « Organiser » coupé quand la barre est repliée | non traduit (le mode lui-même ne l’est pas) |

Écarts et limites :

- Les menus « … » (Fichier / Dossier, Options d’affichage) n’ont plus de bouton une
  fois la barre repliée, comme dans le Svelte ; s’ils étaient ouverts, le repli les
  ferme (`Msg::ToggleSidebar`). Source et bibliothèque restent ouvertes et passent à
  droite de leur bouton. La liste des sources chevauche de 4 px le bord droit de la
  colonne (bouton 52 px centré dans 76 px, + 8 px), comme le calcul du Svelte.
- À 1280 × 900, la colonne (source, six tuiles, bibliothèque, pied) laisse 55 px à la
  liste des playlists : une vignette visible, les autres au défilement. À 700 px de
  haut, toute la barre déborde et défile (`overflow-y-auto` de `Sidebar.svelte`) : la
  gouttière de 8 px de `smart-scroll` décale la colonne de 4 px vers la gauche,
  comme dans WebView2.
- `.library-row` (`flex gap-2`) garde la largeur de la colonne au lieu de se réduire à
  son contenu (rangée flex non réduite dans une colonne `items-center`, voir plus
  bas) : `justify-content: center` y centre le bouton, même rendu.
- Le point du rescan est posé dans le bouton plutôt que sur un `span` relatif qui
  l’entoure : même boîte de 48 px, et il ne prend pas les clics (`pointer-events` n’est
  pas pris en charge).
- `aria-label` ajouté aux boutons réduits à leur icône (tuiles, vignettes, source,
  bibliothèque, pied) : le moteur ne tire pas de nom accessible de `title`.
- Pas de capture Tauri de l’état replié : vérifié sur les aperçus et par les tests
  (largeurs 76 / 288 px, centrage, présence et absence des éléments, infobulle de la
  source, menus à `r.right + 8`, largeur intermédiaire pendant l’animation).
- Épinglés (`SidebarPins` replié : albums en vignettes, artistes en ronds de 44 px)
  non traduits, comme la section dépliée.

## Menus, suggestions et survols

Un seul menu ouvert à la fois (`MusicApp::menu`, `Msg::Menu` / `Msg::CloseMenu`),
déclaré à côté de son déclencheur en `k-popover` (`menu`, `listbox`, `combobox` ou
panneau simple quand des interrupteurs doivent laisser le panneau ouvert). Les choix
modifient l’état local : mode et temps restant du minuteur (bouton vert « 30:00 »),
source et bibliothèque affichées sur leurs boutons, placement et sections de la
bibliothèque (tuiles de la navigation, « En haut » les retire), rangée Fichier /
Dossier masquable, lignes « Titres likés » / « Récemment joués » de la barre latérale,
sortie audio (pastille de la ligne d’état), exclusif / DoP, services réseau (pastille
« Réseau »). Les autres entrées (Paramètres, Statistiques, Nouveau tirage…) ferment
seulement le menu.

Fidélité face aux captures `menu-*.json` (1280 × 900, `tools/menus.ps1`, éléments du
menu seulement) :

| Menu | Capture | Appariés | ≤ 2 px | Médiane | Max | Remarques |
| --- | --- | --- | --- | --- | --- | --- |
| Minuteur de veille | 01 | 15 | 100 % | 1,0 | 1,8 | |
| Menu du profil | 02 | 11 | 100 % | 0,0 | 0,0 | |
| Changer de source | 03 | 13 | 100 % | 0,0 | 0,0 | |
| « … » Ouvrir (sections, Fichier/Dossier) | 04 | 51 | 94 % | 0,0 | 4,0 | libellés des placements : ligne de 14 px au lieu de 10 (accent de « À ») |
| Changer de bibliothèque | 05 | 21 | 95 % | 0,0 | 2,3 | |
| Options d’affichage (Playlists « … ») | 06 | 21 | 100 % | 0,0 | 0,1 | |
| Actions du mix | 07 | 8 | 62 % | 0,0 | 4,0 | la capture montre la carte survolée (pochette relevée de 4 px) ; le menu lui-même est à ≤ 1 px |
| Sortie audio | 08 | 33 | 97 % → 100 % | 0,1 | 3,3 → 0,9 | badge « BÊTA » : `tracking-[0.08em]` hérité du titre, comme la source (3,3 px plus étroit avant) |
| File d’attente | 09 | — | — | — | — | non traduit |
| Paroles | 10 | — | — | — | — | non traduit |
| Lecteurs réseau | 11 | 21 | 100 % | 0,0 | 0,2 | interrupteur relevé de l’espace du jambage, comme le `inline-block` de Chromium |
| Nouveautés | 12 | — | — | — | — | non traduit |
| Suggestions de recherche | search | 33 | 100 % | 0,6 → 0,0 (appariement par l’élément qui porte le texte) | 1,4 | pas de bouton natif « × » du champ `type=search` ni d’anneau de focus dans l’export |

Dans les exports, la première entrée focalisable d’un menu modal porte le style
`:focus-visible` (focus initial sans pointeur) ; Tauri, ouvert à la souris, ne
surligne rien.

**Recherche** : dès deux caractères, la liste des résultats (index statique des
résultats de « que », sous-chaîne sans casse ni accents) s’ouvre sous le champ en
`k-popover combobox` : la frappe continue dans le champ, `onkeydown` déplace la
surbrillance (Haut/Bas), Entrée ou un clic choisit, Échap ou un clic ailleurs ferme.
Vignettes tirées du cache de pochettes de RustMusic (`prepare-covers.py`), icône du
type à défaut ; badge MORCEAU / ALBUM / ARTISTE / COMPOSITEUR ; 400 px au plus, avec
défilement et « Voir tous les résultats ».

**Survols** (`--hover`, planches `rustmusic-html-hover-*-compare.png`) : carte de mix
(bouton « … » et bouton lecture, pochette relevée), tuile de navigation, ligne de
playlist (« … »), ligne de la file « Ensuite » (lecture sur la vignette) — identiques
aux captures. Infobulles : tous les `title` visibles du Svelte, y compris sous un
élément transformé (bouton lecture, flèche du carrousel, pochettes « Reprendre
l’album »).

## Contournements retirés

| Avant | Maintenant |
| --- | --- |
| Composant `<Truncate>` (ellipse mesurée) | `<p class="… truncate">`, `line-clamp` |
| Texte écrit en capitales (`to_uppercase()`, « EN PAUSE ») | `text-transform: uppercase` + `letter-spacing` |
| Couleurs converties en hex, `color.rs` pour les teintes | `oklch()`, `color-mix()`, `currentColor`, `var(--h)` / `var(--c)` hérités |
| Bordures et anneaux en ombres internes d’1 px | `border-bottom`, `border-right`, `border-top` réels |
| `margin: 10px 0px 0px 0px` | `margin-top: 10px` |
| `{#each data::LISTE.iter() as x}` | `{#each data::LISTE as x}` |
| Sections construites par méthodes (`{@element self.home(&cx)}`) | `{@include}` |
| `<div onclick>` pour icône + texte | `<button>` à contenu riche |
| `aria-hidden` retiré | rétabli sur les décors |
| Un parent porte `style:color` d’un composant | `style:--x` sur le composant |
| Hauteurs `calc((100vw - …) / n)` pour `aspect-square` | `aspect-ratio: 1` |
| Hero, mix et file réduits à la variante 1440 px | toutes les variantes responsives |
| `radial-gradient(ellipse farthest-side…)`, couleur médiane du logo | `radial-gradient(80% 170% at 0% 50%, …)` ; dégradé du logo réévalué (couleur finale) |
| Point actif sous les boutons en `<div>` | `::after` |
| Interrupteur de thème par `left` | `transform: translateX()` avec transition |
| `CssVariables` pour le volume | propriété héritée `style:--volume` |
| `@media` décalées de 378 px | `@container` |
| Pack `icons.bin` de contours Material Symbols pleins, substituts des icônes à trait | `<svg src>` exacts générés depuis `@iconify-json` |
| Retrait de 38 px, liste élargie sous le bord rogné, retrait de 8 px à petite hauteur | `scrollbar-width: thin`, `::-webkit-scrollbar { width: 8px }` |
| Composant `<StartTruncate>` | `direction: rtl` + `truncate` |
| `title` omis sous un élément transformé | rétabli |

## Contournements restants et manques du moteur

| Manque | Contournement |
| --- | --- |
| SVG : `text`, dégradés `url(#…)` | logo refait en boîtes |
| `top/right/bottom/left` dans une requête (`@4xl:right-3.5`) | décalages de @4xl gardés pour le bouton lecture des mix (survol seulement) |
| `display: flex` dans une requête remet la direction en ligne | `flex-direction` répété (`@4xl:flex @4xl:flex-col`) |
| `position` et `z-index: auto` dans une requête, transformation d’un conteneur qui porte une zone défilante | barre latérale en `relative` (état `md` de la source, la fenêtre fait au moins 960 px) |
| Arrêts de dégradé `calc(100% - 22px)` | pourcentages (92,5 %, 95 %) |
| Ligne de base des éléments flex approchée (`items-baseline`) | `items-start` et marges mesurées (`mt-[9.5px]`, `mt-[6.6px]`) |
| Barre de défilement posée dans le retrait du conteneur défilant | retrait de la page porté par le `@container` intérieur |
| Canal `calc()` dans une couleur constante (dégradé, ombre, bordure d’un côté) : « Unresolved var() in color » | chromas du lecteur calculées pour `--ls 0.07`, formule en commentaire |
| Couleur venue d’une variable sur un paragraphe enrichi (`RichText`) | couleur sur le conteneur |
| `content` conditionnel | `::after` sur les deux bascules, coloré à l’état actif |
| `height: 100%` dans une cellule de grille vise la grille | `aspect-ratio: 1` pour les mosaïques |
| Rangée flex `width: auto` non réduite dans une colonne `items-center` ; paragraphe en ligne flex étiré | `justify-content: center` (transport), `width: auto` sur les libellés, `text-align: center` (feuille par défaut des boutons) |
| Enfant positionné rogné à la boîte de son hôte | pastille du volume sortie du rail de 5 px |
| `transform` sur `<img>` | conteneur intermédiaire |
| Dégradés par élément (`var(--h)` dans un dégradé) | `color.rs` (`genre_tile`, `playlist_fond`) |
| `filter: blur(60px)` en direct sur un élément coupé par le défilement : le flou s’assombrit vers le bord du clip (960 × 700) | non corrigé ; Tauri utilise une pochette pré-floutée |
| Animation d’échelle sur une couche (popover) refusée | entrée des menus en fondu seul (Svelte : `scale` 0,97 → 1) |
| `gap` posé sur le `k-popover` lui-même sans effet sur ses enfants | conteneur interne qui porte le flex et le gap (liste des sources) |
| Texte rogné à sa boîte de ligne (`leading-none` coupe l’accent de « À ») | ligne de 14 px reprise par des marges négatives |
| `scrollbar-color` non conditionnel (`.smart-scroll:hover` montre le pouce dans Tauri) | pouce de la barre latérale toujours transparent |
| Barre classique de WebView2 (flèches, piste grise) | barre fine du moteur, 15 px de gouttière |
| Entrée `menu`/`listbox` : `onclose` après le message | interrupteurs dans un panneau simple (sans navigation par flèches) |
| Rôles `menuitemradio` / `menuitemcheckbox`, `aria-checked`, `aria-activedescendant` | `menuitem` / `option` simples ; la suggestion en surbrillance n’est pas annoncée |
| Sous-menus en cascade | « … » de gestion d’une bibliothèque sans menu |
| Tiroirs latéraux plein écran avec voile flouté (file, paroles) et popin des nouveautés | non traduits |
| `:focus-visible` sur la première entrée à l’ouverture (exports sans pointeur) | non corrigé |
| `input type="number"` (minuteur personnalisé) | champ texte |

### Icônes

`tools/icons.py` relève les noms Iconify écrits dans les templates et `data/`
(`"lucide:plus"`, `"material-symbols:home-outline-rounded"`…), lit chaque icône dans
les paquets locaux `@iconify-json/<jeu>/icons.json` de RustMusic (alias suivis, sans
réseau), écrit `icons/<jeu>/<nom>.svg` tel que `@iconify/svelte` le rend (viewBox du
jeu, corps inchangé) et génère `icons.rs`, la correspondance nom → `<svg src>`
compilé. Le composant `<Icon icon="…" width={…}>` reste la traduction de celui du
Svelte : taille `width` × `width`, `currentColor` = couleur CSS héritée (survol
compris). `epais` ajoute le trait de `.sb-icone` (`stroke: currentColor;
stroke-width: .55; stroke-linejoin: round`) aux icônes de la navigation, de la source
et de la bibliothèque (liste `THICK` du script).

Icônes rétablies (substituts Material Symbols retirés) : `lucide:plus`,
`lucide:refresh-cw`, `lucide:disc-3`, `mynaui:heart`, `heroicons:chevron-left/right`,
`mynaui:search`, `tabler:moon` / `tabler:sun`, `tabler:alarm-snooze`,
`lucide:picture-in-picture-2`, les genres en Phosphor duotone (`ph:*-duotone` de
`genreIcon.ts`), `mynaui:lightning` (playlist Grunge), le chevron du profil (SVG en
ligne `m6 9 6 6 6-6`) ; plus toutes celles des menus (lucide, mynaui, material-symbols,
material-symbols-light). Aucune icône manquante dans les paquets locaux.

## Mesure face à l'application Tauri (2026-11-03)

Même machine, fenêtre 1280 × 900 (taille de `tauri.conf.json`), page d'accueil,
aucune lecture. Trois lancements par application : 15 s de stabilisation, puis
mémoire de toute l'arborescence de processus et CPU au repos sur 30 s.
Exécutables release : `RustMusic.exe` (Tauri 2 + Svelte, WebView2) et les
exemples `rustmusic_html` et `rustmusic` (Karbon UI, présentation DirectComposition ;
l’exemple natif `rustmusic` a été retiré depuis).

| | Processus | Working set | Mémoire privée | CPU au repos (1 cœur) |
| --- | --- | --- | --- | --- |
| Tauri — total | 7 | ~945 Mo | ~840 Mo | 7–8 % |
| Tauri — WebView2 seule (coût de l'UI) | 6 | ~480 Mo | ~417 Mo | ~7,4 % (GPU 5,8 + rendu 1,6) |
| Karbon UI — page HTML/CSS | 1 | ~116 Mo | ~167 Mo | ~0,03 % |
| Karbon UI — page en API Rust | 1 | ~100 Mo | ~96 Mo | ~0,02 % |

Lecture :
- La comparaison juste est **WebView2 seule contre Karbon UI** : le processus
  principal Tauri (~470 Mo) contient le vrai moteur audio et la bibliothèque,
  absents de la maquette native.
- L'interface native coûte **4 à 5 fois moins de mémoire** (working set) que la
  seule WebView2, et ne consomme pratiquement aucun CPU au repos. Le CPU de
  Tauri au repos vient du processus GPU et du rendu de la WebView : une animation
  CSS de la page tourne probablement en continu ; elle n'est pas reproduite.
- L'apparition de la fenêtre n'est pas comparable : Tauri montre une fenêtre vide
  en 50–300 ms puis charge la page ; Karbon UI montre la fenêtre déjà dessinée
  après ~650 ms (polices, images, device D3D). Ce délai est à optimiser.
- La version HTML/CSS consomme ~70 Mo privés de plus que la version API Rust :
  écart à analyser (fond flouté, pack d'icônes, images).

Ces mesures datent d’avant le passage à l’état de référence (pochettes du cache).

### Suivi moteur (2026-11-03)

Corrigés dans le moteur, contournements retirés de l’exemple : `style:prop={…}` sur
un composant, `{#each data::LISTE as x}` par référence, `margin-top: 10px` dans une
règle de base, largeur des paragraphes arrondie au pixel supérieur, `flex flex-col`
dans un conteneur défilant et `overflow-x: auto`, pile en debug. Démarrage et
mémoire : voir [Mesurer le démarrage](performance.md#mesurer-le-démarrage-windows).

Corrigés avec ce travail (commits séparés, tests de régression) :
`align-items` d’une grille ignoré (les cellules s’étiraient) ;
`border-radius: 9999px` dessiné en ellipse par Direct2D (rayon borné axe par axe au
lieu de la normalisation CSS) — cause des coins des pilules (profil, « Ajouter de la
musique », boutons ronds) ; `diagnostics::inspect_preview` pour la comparaison.

Ajoutés au moteur pour la barre repliée (commits séparés, tests de régression) :
transitions CSS de `width` / `height` en pixels (`transition: width 200ms`, état ou
classe basculée par la vue, un layout par image, [css.md](css.md)) ;
`justify-content` d’une grille (colonnes fixes centrées ou réparties) ; infobulles :
un voisin dont seul le clip hérité contient le pointeur (la pastille « Réseau » du
lecteur) ne prend plus l’aide de l’élément réellement survolé.

Ajoutés au moteur pour les menus et les interactions (commits séparés, tests de
régression) : `::-webkit-scrollbar { width }` (épaisseur de la gouttière,
[css.md](css.md)), `cross-offset` des popovers (menus qui débordent de 16 px de leur
bouton), `k-popover combobox` et `onkeydown` des champs texte (suggestions pendant la
saisie, [html-syntax §11](html-syntax.md#11-menus-listes-de-choix-et-infobulles)),
`BenchmarkOptions::pointer` (aperçus survolés). Les apports précédents (SVG en ligne,
`@container`, barres fines, menus `k-popover`, `title` sous un élément transformé,
`direction: rtl`, `align-items: baseline`, pistes `min-content`) sont décrits dans
[html-syntax.md](html-syntax.md) et [css.md](css.md) et adoptés ici ; leurs limites
connues (`width: min-content` sur un élément, `direction: rtl` sans réordonnancement,
SVG sans dégradés / `use` / `text` / masques / tirets, ligne de base approchée) restent
valables.
