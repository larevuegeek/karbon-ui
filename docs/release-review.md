# Revue avant publication (2026-11-12)

Relecture de tout le dépôt avant la première publication sur crates.io (0.1.0), menée
depuis Windows : cinq relectures en lecture seule (moteur, backends Windows et macOS,
compilateur), plus un audit mécanique et l'essai de la version minimale de Rust. Le backend
Linux a sa propre revue, [linux-review.md](linux-review.md).

États : **Corrigé** (vérifié par les tests sous Windows), **Corrigé (à valider)** (compile
pour la cible, non exécuté sur le système concerné), **À faire**, **À décider** (choix du
propriétaire). Les constats viennent d'une lecture du code : ceux qui restent « À faire »
n'ont pas été reproduits.

## Publication

| # | État | Constat |
| --- | --- | --- |
| P1 | Corrigé | `rust-version = "1.88"` était faux : le code généré en mode `framework` (Bootstrap) demande Rust 1.89 (durée de vie des temporaires à travers les constructeurs de variantes). Vérifié avec les chaînes 1.88 (échec) et 1.89 (succès) ; version annoncée : 1.89. |
| P2 | Corrigé | Icônes tierces de `examples/rustmusic_html/icons/` (Heroicons, Lucide, Material Symbols, MynaUI, Phosphor, Tabler) sans mention de licence : `icons/LICENSES.md`. |
| P3 | Corrigé | `assets/SOURCES.md` décrivait encore de vraies pochettes et un fichier `covers-sources.json` disparu. |
| P4 | Corrigé | `src/lib.rs` annonçait « seul un backend Windows existe ». |
| P5 | Corrigé | README de `karbon-ui-macros` (page crates.io) en français et interne : remplacé par une page en anglais, la carte du compilateur est devenue [compiler.md](compiler.md). |
| P6 | Corrigé | CI : `actions/checkout@v5`, `actions/upload-artifact@v6` (Node 24), une poussée annule l'exécution précédente de la branche. |
| P7 | Corrigé | README : logo, badges, captures (`docs/screenshots/`, exports des exemples), exemples manquants (`kit_gallery`, `system_dialogs`), nombres de tests. |
| P8 | À faire | Six fichiers dépassent 500 lignes : `styling/resolve/cascade.rs` 577 (extraire l'empreinte des sous-arbres mémorisés, les déclarations dynamiques, les surcharges d'animation), `macos/window/session/lifecycle.rs` 540 (cadence vers `session/pacing.rs`), `ui-macros/css/framework/element.rs` 509, `macos/window/event_loop.rs` 505 (`Pointer`, `pointer()`, `key()` vers `window/input.rs`), `ui-macros/template/generate/tree.rs` 505, `linux/wayland/display.rs` 503. |

## Moteur : exécution et composants (`src/ui/runtime`, `components`, `kit`…)

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| E1 | bug | Corrigé, test ajouté | `runtime/overlays.rs` | `layer_viewport` ignorait `chrome.visible` : dans une fenêtre sans barre du moteur (toujours sous macOS, `title_bar: false` ailleurs), la liste d'un `Select` se refermait à la préparation suivante, et menus et aides étaient placés 40 px trop bas. Une seule fonction sert maintenant aux deux endroits. |
| E2 | bug | Corrigé | `components/rich_text.rs` | Fragments d'un lien gardés en coordonnées de la fenêtre : après un défilement, le lien ne se survolait ni ne se cliquait plus, anneau de focus à l'ancienne place. Fragments relatifs à la boîte du lien. |
| E3 | bug | Corrigé | `runtime/events/pointer.rs` | Double clic d'un champ compté avec l'horloge des animations, qui n'avance pas sans animation : deux clics à dix secondes d'écart sélectionnaient le mot. |
| E4 | bug | Corrigé | `runtime/smooth_scroll.rs` | Un seul glissement à la fois : le reste d'un cran passé au conteneur englobant écrasait le glissement du conteneur intérieur, qui n'atteignait jamais son bout. L'autre glissement arrive maintenant à sa cible. |
| E5 | bug | Corrigé | `runtime/interaction.rs` | Appui annulé après une reconstruction pour un bouton hors de l'ordre de tabulation (`tabindex="-1"`) : le clic ne partait pas. |
| E6 | bug probable | À faire | `components/text_input/events.rs` | Les touches lisent `e.line`, rafraîchie par `prepare` seulement : avec `UiRuntime::event` appelé sans `prepare` entre deux entrées (API publique, les backends fournis préparent toujours), un retour arrière peut couper hors d'une frontière de caractère, et `WordRight` boucler. Borner aux frontières de `e.text`, garde de progression, contrat à écrire dans la doc de `UiRuntime`. |
| E7 | bug | Corrigé | `kit/feedback.rs` | `Progress` avec une valeur non numérique (0 / 0) : la mise en page refusait l'arbre et la fenêtre se fermait. |
| E8 | nettoyage | Corrigé | `select`, `slider`, `text_input`, `runtime/accessibility/snapshot.rs`, `windows/accessibility/sync.rs` | Textes par défaut en français dans une API anglaise (« Choisir… », « Champ de saisie », « Zone de défilement », boutons de la barre de titre pour les lecteurs d'écran) : en anglais, comme `DEFAULT_MENU_LABELS`. |
| E9 | nettoyage | Corrigé | `text_input/menu.rs` | Rustdoc public en français. |
| E10 | nettoyage | Corrigé | `slider/widget.rs` | `Slider::snapped` paniquait sur une plage invalide. |
| E11 | mineur | À faire | `runtime/prepare.rs` | L'appui rejoué après la fermeture d'un popover arrive après la peinture : l'état `:active` n'apparaît qu'à l'événement suivant. |

## Moteur : mise en page et styles (`src/ui/layout`, `styling`, `core`)

Cause commune de L2 à L4 : `layout::validate` passe une fois par reconstruction, avant la
cascade ; les valeurs écrites ensuite par une règle d'état (`md:`, `hover:`, `@media`,
`hidden` d'un gabarit) ne sont plus vérifiées.

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| L1 | bug | Corrigé | `layout/virtual_list.rs:172` | `refresh` fait `node.scroll.unwrap()` sur une liste virtuelle jamais placée (dans un conteneur `display: none`, onglet inactif) : panique. |
| L2 | bug | Corrigé | `layout/engine.rs` (`limits`) | `min-width` supérieur à `max-width` par une règle d'état (`min-w-[320px] md:max-w-[240px]`) : `f32::clamp` panique. Le minimum doit gagner. |
| L3 | bug | À faire | `layout/position.rs`, `measure.rs`, `scrolling.rs`, `split.rs` | Enfant `hidden` d'un `Split` ou d'un `Scroll` retiré du flux, puis indexé : panique. |
| L4 | bug probable | À faire | `layout/grid.rs:256` | Étendue de colonne supérieure au nombre courant de colonnes (`auto-fill` dans une fenêtre étroite) : erreur fatale selon la largeur. Borner l'étendue. Ligne 258 : dépassement pour une colonne énorme. |
| L5 | bug probable | À faire | `layout/tracks.rs:387` | `min-content` d'une feuille non textuelle lu avant sa mesure (0) : colonne `min-content` réduite au padding. |
| L6 | bug probable | À faire | `layout/lazy.rs:36` | Conteneur repris et déplacé : un descendant absolu ancré hors du sous-arbre est déplacé aussi. |
| L7 | bug probable | À faire | `layout/expression.rs:94` | Variable qui se référence deux fois (`calc(var(--a) + var(--a))`) : 2^32 nœuds évalués avant l'erreur. Budget de nœuds. |
| L8 | bug rare | À faire | `layout/arrange.rs:116` | Ligne de liste virtuelle hors flux : lignes et indices décalés, insertion hors bornes. |
| L9 | bug probable | À faire | `layout/grid.rs:281` | Placement automatique depuis la ligne 0 pour chaque enfant : comportement `dense`, coût quadratique. |
| L10 | bug probable | À faire | `styling/tokens/shadow.rs` | Listes d'ombres calculées gardées à vie ; après 4 096 listes différentes l'ombre n'est plus appliquée (ombre animée par une variable). |
| L11 | bug probable | Corrigé | `layout/split.rs:56` | `split_extent` (public) : `clamp` peut paniquer par arrondi ou sur une valeur non numérique. |
| L12 | performance | À faire | `core/hash.rs` | `finish` rend l'état brut : des identifiants qui ne diffèrent que par les bits hauts tombent dans le même groupe. `rotate_left(26)` ; commentaire à corriger (les clés viennent aussi de l'application). |
| L13 | bug mineur | À faire | `styling/resolve/cascade.rs:445` | `unit_context` changé sans invalider la mise en page : un sous-arbre mémorisé en `vw`/`vh` garde sa mesure après un redimensionnement. |
| L14 | nettoyage | À faire | divers | Doc de `fingerprint` collée sur `sibling_fingerprint` (`resolve/matching.rs:80`) ; `measure_unmemoized` mal nommée ; `Length::Percent` documenté « 0 à 100 » ; `validation.rs:300` valide la liste avec `gap` au lieu de `vertical_gap()` ; champ public `StyleEnvironment::partial` de type non exporté ; `memo.rs:185` empile des doublons. |

## Backend Windows et socle (`src/platform/windows`, `task`, `resources`, `window`)

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| W1 | bug | Corrigé | `graphics/typography/rich.rs:257` | `rich_rects` indexe la table UTF-16 du paragraphe coupé avec des octets du paragraphe d'origine (lien dans un texte `nowrap` + ellipse + `rtl`) : hors bornes. Décaler comme Linux (`graphics/text/measure.rs`). |
| W2 | bug | Corrigé | `events/keyboard.rs` | Touches de développement actives dans toute fenêtre (F2 renderer, F3 transparence, F5 thème, F8 statistiques) et Échap qui détruisait la fenêtre quand l'interface ne la consommait pas. Les touches F ne répondent plus qu'avec l'option `profile` ; Échap suit le chemin des autres touches. |
| W3 | bug | À faire | `events/procedure.rs` | Après une panique rattrapée, la fenêtre est détruite sans le démontage de `WM_DESTROY` : la fenêtre d'ombre reste à l'écran, l'accessibilité n'est pas déconnectée. |
| W4 | bug probable | À faire | `window/session.rs`, `window/tasks.rs` | Rappels de l'application appelés depuis la boucle hors de `catch_unwind` (`task_completed`, `take_window_commands`, `load`) : selon le moment, la panique sort de `platform::run` au lieu de rendre `Error::Panicked`. |
| W5 | bug probable | Corrigé | `graphics/formats.rs:40` | Formats DirectWrite créés avec la locale `fr-FR` en dur (repli de polices et césure sur un système chinois ou japonais). Lire `GetUserDefaultLocaleName`. |
| W7 | bug probable | À faire | `present/runtime.rs`, `present/paint.rs` | `DRAWING` emprunté pendant les rappels : une application qui pompe des messages dans `update` (`MessageBoxW`) fait paniquer la peinture imbriquée. `try_borrow_mut` et report. |
| W8 | bug probable | À faire | `fonts.rs:178` | Cache de résolution des familles indexé par l'adresse de la collection, jamais invalidé : deux fenêtres aux polices embarquées différentes peuvent se mélanger. |
| W9 | mineur | À faire | `accessibility/sync.rs:38` | `PUBLISHED_FOCUS` commun à toutes les fenêtres. |
| W10 | mineur | Corrigé | `events/text.rs` | `WM_UNICHAR` laissait passer les caractères de commande. |
| W11 | nettoyage | Corrigé | `present/drawing.rs`, `window/dwm.rs` | `println!` à chaque ouverture de fenêtre : avec `profile` seulement. |
| W12 | nettoyage | Corrigé | `window/config.rs`, `window/create.rs`, `window/shadow.rs`, `accessibility/provider/common.rs` | Noms d'anciennes étapes : titre par défaut « Native UI - J2 », classes `MonMoteur`, fournisseur UIA `WindowEngine`. |
| W13 | nettoyage | Corrigé | `diagnostics.rs`, `error.rs`, `task/types.rs` | Rustdoc public qui contredit le code (aperçus « sans backend natif hors de Windows », `QueueFull`). |
| W14 | mineur | À faire | `window/dialog.rs:146`, `window/shadow.rs:92` | Chemin avec un demi-codet isolé refusé (`OsString::from_wide`) ; `GetModuleHandleW` non vérifié. |

## Backend macOS (`src/platform/macos`)

Rien de cette section n'a été exécuté : la revue a été faite depuis Windows.

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| M1 | bug | Corrigé (à valider) | `window/session/frame_link.rs:80`, `lifecycle.rs:328` | L'attente du lien d'affichage expirée n'est jamais effacée : fenêtre couverte pendant une animation, `timeout()` reste dans le passé, la boucle tourne à 100 % du processeur. Filtrer `timeout(now)` ou effacer `awaiting` à l'expiration. |
| M2 | bug | À faire | `window/event_loop.rs:320,352`, `window/change.rs` | `Session::change` et `show` appellent AppKit sous l'emprunt des sessions : les notifications synchrones (clé, taille) sont perdues (`try_borrow_mut`). Fenêtre B sans focus après `Focus`, image non rafraîchie après `Size`. |
| M3 | bug probable | À faire | `metal/renderer.rs:195` | Image sans drawable : les caches (groupes filtrés, mipmaps) gardent des entrées jamais rendues. |
| M4 | bug probable | À faire | `window/event_loop.rs:466` | Pas de `NSApplicationDelegate` : Quitter depuis le Dock ou la fermeture de session termine le processus sans que `run_windows` rende la main ; Quitter grisé sans fenêtre clé. |
| M5 | bug probable | À faire | `window/session/paint.rs:26` | Une fenêtre entièrement couverte continue de peindre, ce qui annule la libération des ressources. |
| M6 | mémoire | À faire | `metal/resources.rs:167`, `metal/submit.rs:47` | `Trim::Hidden` garde les cibles de la dernière image (`taken`, `ring.last`). |
| M7 | bug probable | Corrigé (à valider) | `window/event_loop.rs:473` | `activateIgnoringOtherApps(true)` même quand toutes les fenêtres ont `activate: false`. |
| M8 | bug | Corrigé (à valider) pour `dragging` ; À faire pour `GRIP` | `window/change.rs:203` | `dragging` vrai pour toutes les fenêtres pendant le geste d'une seule ; `GRIP` jamais effacé si la fenêtre se ferme pendant le geste. |
| M9 | à vérifier | À faire | `window/event_loop.rs:47` | `WAKE_PENDING` reste vrai si une boucle AppKit imbriquée consomme l'événement de réveil. |
| M10 | performance | À faire | `metal/submit.rs:125` | Cible allouée pour chaque calque, même vide. |
| M11 | latent | À faire | `metal/encode/raster.rs:57` | Clé du cache par adresse d'un `Arc<VectorIcon>` non retenu (même schéma dans `linux/vulkan/encode/raster.rs`). |
| M12 | nettoyage | À faire | `metal/renderer.rs:141` | Erreur d'`encode` sans `resources.end()`. |
| M13 | nettoyage | À faire | divers | Commentaires en double (`window/view.rs:55`, `graphics/cache.rs:41`, `window/events.rs:80`), `offscreen.rs` : trois `unsafe` sans `// SAFETY :`, messages d'erreur en français sur la sortie d'erreur hors `--profile`. |

## Compilateur (`crates/ui-macros`)

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| C1 | bug | Corrigé, test ajouté | `css/framework/selectors/parse.rs:391` | `a[href^]` (opérateur sans `=`) : tranche hors bornes, la macro panique. |
| C2 | bug | Corrigé, test ajouté | `css/framework/values/tokens.rs:216` | `group_len` rend des octets, utilisés comme nombre de caractères : panique avec un caractère non ASCII dans `rgb()`. |
| C3 | bug | Corrigé | `input/files.rs:72`, `css/framework/model.rs:71` | BOM UTF-8 jamais retiré des `.ui` et des CSS (fichier enregistré « UTF-8 avec BOM ») : erreurs incompréhensibles, variables de `:root` perdues en mode `framework`. |
| C4 | bug | Corrigé, test ajouté | `css/framework/values/normalize.rs:14` | Chaîne entre guillemets recopiée octet par octet : `"Hélvetica"` devient `"HÃ©lvetica"`. |
| C5 | bug probable | Corrigé | `codegen/properties.rs:74` | Expression collée en `({expr} as f32)` : `style:gap={self.base + 4}` devient `self.base + (4 as f32)`. |
| C6 | bug probable | À faire | `codegen/properties.rs:94` | Identifiant seul lu comme mot-clé CSS : `style:align={alignment}` refusé. |
| C7 | bug probable | À faire | `template/generate/components.rs:101` | Nom d'attribut d'un composant collé tel quel : `<Button type="…">` (mot-clé Rust) inutilisable ; noms `__ui_*` non refusés. |
| C8 | bug probable | Corrigé | `input/files.rs` | `.ui` en CRLF : `\r` gardés dans `<pre>`. Fins de ligne normalisées à la lecture, avec le BOM (C3). |
| C9 | bug probable | À faire | `classes.rs:80` | Feuille `framework` gardée en cache par chemin : diagnostics périmés sous rust-analyzer après une modification. |
| C10 | bug probable | À faire | `codegen/properties.rs:66`, `template/generate/select.rs` | Expression de l'appelant évaluée deux fois (`overflow`, `option`). |
| C11 | bug probable | Corrigé | `tailwind/resolve.rs:100` | `calc(0px + 50%)` replié en `50px`. |
| C12 | mineur | À faire | `svg/paint.rs:229`, `css/inline.rs:84` | Valeur finie en `f64` devenue infinie en `f32` (`inff32` dans le code généré) ; borne de durée décalée de un. |
| C13 | mineur | À faire | `template/markup/parser.rs`, `css/parser/reader.rs`, `css/framework/syntax.rs` | Récursion non bornée pour `{#if}`, `@media` et accolades imbriqués. |
| C14 | nettoyage | À faire | divers | Code mort (`styles.rs:37`, `template/generate/element.rs:275`, `components.rs:139`, `css/selector.rs:322`, `tailwind/utilities/dispatch.rs:289`, `statics.rs:272`, `css/class/rules.rs:239`) ; messages « Template include… » pour un CSS ou un SVG (`input/files.rs`) ; commentaires périmés (`normalize.rs:6`, `svg/document.rs:132`, `markup/location.rs:7`). |
