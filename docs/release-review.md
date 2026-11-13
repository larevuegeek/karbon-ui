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
| P8 | Corrigé | Fichiers de plus de 500 lignes découpés, code déplacé tel quel : `styling/resolve/cascade.rs` (`memoized.rs`, `dynamic.rs`, `motion.rs`), `macos/window/session/lifecycle.rs` (`pacing.rs`), `macos/window/event_loop.rs` (`input.rs`), `linux/wayland/display.rs` (`dispatch.rs`), `ui-macros/template/generate/tree.rs` (`each.rs`), `ui-macros/css/framework/element.rs` (`reveal.rs`). |

## Moteur : exécution et composants (`src/ui/runtime`, `components`, `kit`…)

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| E1 | bug | Corrigé, test ajouté | `runtime/overlays.rs` | `layer_viewport` ignorait `chrome.visible` : dans une fenêtre sans barre du moteur (toujours sous macOS, `title_bar: false` ailleurs), la liste d'un `Select` se refermait à la préparation suivante, et menus et aides étaient placés 40 px trop bas. Une seule fonction sert maintenant aux deux endroits. |
| E2 | bug | Corrigé | `components/rich_text.rs` | Fragments d'un lien gardés en coordonnées de la fenêtre : après un défilement, le lien ne se survolait ni ne se cliquait plus, anneau de focus à l'ancienne place. Fragments relatifs à la boîte du lien. |
| E3 | bug | Corrigé | `runtime/events/pointer.rs` | Double clic d'un champ compté avec l'horloge des animations, qui n'avance pas sans animation : deux clics à dix secondes d'écart sélectionnaient le mot. |
| E4 | bug | Corrigé | `runtime/smooth_scroll.rs` | Un seul glissement à la fois : le reste d'un cran passé au conteneur englobant écrasait le glissement du conteneur intérieur, qui n'atteignait jamais son bout. L'autre glissement arrive maintenant à sa cible. |
| E5 | bug | Corrigé | `runtime/interaction.rs` | Appui annulé après une reconstruction pour un bouton hors de l'ordre de tabulation (`tabindex="-1"`) : le clic ne partait pas. |
| E6 | bug probable | Corrigé, test ajouté | `components/text_input/events.rs` | Arrêts du curseur lus dans la ligne mesurée seulement si elle correspond au texte actuel, sinon d’un caractère ; progression garantie (plus de boucle de `WordRight`) ; `move_to` borné aux frontières (Fin d’un champ multiligne mesuré en retard paniquait aussi). Contrat écrit dans la rustdoc de `UiRuntime` et `event`. |
| E7 | bug | Corrigé | `kit/feedback.rs` | `Progress` avec une valeur non numérique (0 / 0) : la mise en page refusait l'arbre et la fenêtre se fermait. |
| E8 | nettoyage | Corrigé | `select`, `slider`, `text_input`, `runtime/accessibility/snapshot.rs`, `windows/accessibility/sync.rs` | Textes par défaut en français dans une API anglaise (« Choisir… », « Champ de saisie », « Zone de défilement », boutons de la barre de titre pour les lecteurs d'écran) : en anglais, comme `DEFAULT_MENU_LABELS`. |
| E9 | nettoyage | Corrigé | `text_input/menu.rs` | Rustdoc public en français. |
| E10 | nettoyage | Corrigé | `slider/widget.rs` | `Slider::snapped` paniquait sur une plage invalide. |
| E11 | mineur | Corrigé, test ajouté | `runtime/prepare.rs` | Après l’appui rejoué, `prepare_frame` refait un tour (un seul, `replaying`) : `:active` peint dans la même image. |

## Moteur : mise en page et styles (`src/ui/layout`, `styling`, `core`)

Cause commune de L2 à L4 : `layout::validate` passe une fois par reconstruction, avant la
cascade ; les valeurs écrites ensuite par une règle d'état (`md:`, `hover:`, `@media`,
`hidden` d'un gabarit) ne sont plus vérifiées.

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| L1 | bug | Corrigé | `layout/virtual_list.rs:172` | `refresh` fait `node.scroll.unwrap()` sur une liste virtuelle jamais placée (dans un conteneur `display: none`, onglet inactif) : panique. |
| L2 | bug | Corrigé | `layout/engine.rs` (`limits`) | `min-width` supérieur à `max-width` par une règle d'état (`min-w-[320px] md:max-w-[240px]`) : `f32::clamp` panique. Le minimum doit gagner. |
| L3 | bug | Corrigé, test ajouté | `layout/validation.rs`, `measure.rs`, `scrolling.rs`, `split.rs` | Enfant `hidden` d’un `Split` ou d’un `Scroll` permis : l’autre volet occupe toute la place, sans séparateur ; la zone de défilement reste vide. Le séparateur d’un `Split` redimensionnable à volet masqué quitte l’ordre de tabulation et l’arbre d’accessibilité (`layout::resizable_divider`). Une couche masquée (`Layers`) reste refusée. |
| L4 | bug | Corrigé, test ajouté | `layout/grid.rs` | Étendue de colonne supérieure au nombre courant de colonnes (`auto-fill` dans une fenêtre étroite) : erreur fatale selon la largeur. Dans une grille `auto-fill`/`auto-fit`, l'étendue est bornée, une colonne explicite qui n'y tient plus revient au placement automatique, une ligne imposée déjà pleine aussi. Dépassement d'entier pour une colonne énorme corrigé. Une grille au nombre de colonnes fixe refuse toujours une étendue trop large à la validation. |
| L5 | bug probable | Corrigé, test ajouté | `layout/tracks.rs:387` | Feuille non textuelle mesurée à largeur 0 au lieu de relire `intrinsic` avant sa mesure (colonne de 8 px au lieu de 58 à la première mise en page). |
| L6 | bug probable | Corrigé, test ajouté | `layout/lazy.rs:36` | Conteneur repris et déplacé : placement refait quand un descendant `absolute` est ancré hors de lui. |
| L7 | bug probable | Corrigé, test ajouté | `layout/expression.rs:94` | Budget de 4 096 nœuds par évaluation (`calc(var(--a) + var(--a))` : plus de 3 min avant, immédiat après). |
| L8 | bug rare | Corrigé, test ajouté | `layout/arrange.rs:116` | Lignes d’une liste virtuelle jamais retirées du flux ; une ligne `display: none` devient une boîte vide à sa place. |
| L9 | bug probable | Corrigé, test ajouté | `layout/grid.rs:281` | Placement automatique « sparse » de CSS (lignes imposées d’abord, puis un curseur qui ne recule pas) : 16 384 enfants en 4,1 s avant, 66 ms après ; [layout-j2.md](layout-j2.md) à jour. |
| L10 | bug probable | Corrigé, test ajouté | `styling/tokens/shadow.rs` | Listes d’ombres calculées partagées et libérées avec leur dernier utilisateur : `Shadows::values` est un `ShadowList` (`Static` pour les feuilles compilées, `Shared(Arc)` pour les ombres calculées ; égalité par contenu). Plus d’internement à vie ni de limite de 4 096 listes, plus de fuite dans `TemplateContext::variable_shadows`. Changement d’API (accord du propriétaire, 2026-11-13) : `Shadows`, `Properties`, `ComputedStyle` et `ControlPart` ne sont plus `Copy` (`clone()` ou référence). Test : 5 000 listes différentes, toutes appliquées et libérées ; exports identiques, défilement du clone sans saccade. |
| L11 | bug probable | Corrigé | `layout/split.rs:56` | `split_extent` (public) : `clamp` peut paniquer par arrondi ou sur une valeur non numérique. |
| L12 | performance | Corrigé, test ajouté | `core/hash.rs` | `finish` fait `rotate_left(26)` : 4 096 clés `i << 32` dans 1 groupe avant, 1 985 après ; commentaire corrigé. |
| L13 | bug mineur | Corrigé, test ajouté | `styling/resolve/cascade.rs:445` | `unit_context` changé : mise en page demandée quand l’élément évalue des longueurs relatives (sous-arbre mémorisé en `50vh` : 200 px gardés avant, 300 après). |
| L14 | nettoyage | Corrigé, tests ajoutés | divers | Validation de la liste virtuelle avec `vertical_gap()`, doublons de `memo.rs` retirés, `styling::Partial` exporté, doc de `fingerprint` remise en place, `measure_unmemoized` devenu `measure_in_pass`, doc de `Length::Percent` corrigée. |

## Backend Windows et socle (`src/platform/windows`, `task`, `resources`, `window`)

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| W1 | bug | Corrigé | `graphics/typography/rich.rs:257` | `rich_rects` indexe la table UTF-16 du paragraphe coupé avec des octets du paragraphe d'origine (lien dans un texte `nowrap` + ellipse + `rtl`) : hors bornes. Décaler comme Linux (`graphics/text/measure.rs`). |
| W2 | bug | Corrigé | `events/keyboard.rs` | Touches de développement actives dans toute fenêtre (F2 renderer, F3 transparence, F5 thème, F8 statistiques) et Échap qui détruisait la fenêtre quand l'interface ne la consommait pas. Les touches F ne répondent plus qu'avec l'option `profile` ; Échap suit le chemin des autres touches. |
| W3 | bug | Corrigé, test ajouté | `events/procedure.rs` | Après une panique rattrapée, la fenêtre est détruite sans le démontage de `WM_DESTROY` : la fenêtre d'ombre reste à l'écran, l'accessibilité n'est pas déconnectée. Le démontage (`teardown`) s'exécute aussi après une panique, chaque étape isolée. |
| W4 | bug probable | Corrigé (à valider) | `window/session.rs`, `window/tasks.rs` | Rappels de l’application appelés depuis la boucle hors de `catch_unwind` (`task_completed`, `take_window_commands`, `load`). `guard` / `catch_panic` (`events/procedure.rs`) : fenêtre détruite et `Error::Panicked`. Tests Windows ajoutés, non exécutés (écrits sur Mac). |
| W5 | bug probable | Corrigé | `graphics/formats.rs:40` | Formats DirectWrite créés avec la locale `fr-FR` en dur (repli de polices et césure sur un système chinois ou japonais). Lire `GetUserDefaultLocaleName`. |
| W7 | bug probable | Corrigé (à valider) | `present/runtime.rs`, `present/paint.rs` | `DRAWING` emprunté pendant les rappels : peinture imbriquée (application qui pompe des messages dans `update`) reportée (`exclusive`, `paint_deferred`), lectures imbriquées sans panique. Test Windows ajouté, non exécuté. |
| W8 | bug probable | Corrigé (à valider) | `fonts.rs:178` | Cache des familles indexé par un numéro unique par collection au lieu de son adresse. |
| W9 | mineur | Corrigé (à valider) | `accessibility/sync.rs:38` | `PUBLISHED_FOCUS` propre à chaque fenêtre (`accessibility_focus`). |
| W10 | mineur | Corrigé | `events/text.rs` | `WM_UNICHAR` laissait passer les caractères de commande. |
| W11 | nettoyage | Corrigé | `present/drawing.rs`, `window/dwm.rs` | `println!` à chaque ouverture de fenêtre : avec `profile` seulement. |
| W12 | nettoyage | Corrigé | `window/config.rs`, `window/create.rs`, `window/shadow.rs`, `accessibility/provider/common.rs` | Noms d'anciennes étapes : titre par défaut « Native UI - J2 », classes `MonMoteur`, fournisseur UIA `WindowEngine`. |
| W13 | nettoyage | Corrigé | `diagnostics.rs`, `error.rs`, `task/types.rs` | Rustdoc public qui contredit le code (aperçus « sans backend natif hors de Windows », `QueueFull`). |
| W14 | mineur | Corrigé (à valider) | `window/dialog.rs:146`, `window/shadow.rs:92` | Chemin rendu par `OsString::from_wide` (demi-codet isolé gardé) ; `GetModuleHandleW` vérifié. |

## Backend macOS (`src/platform/macos`)

Revue faite depuis Windows ; tout a été repris, exécuté et testé sur un Mac (Apple
Silicon) le 2026-11-13, avec la sonde native `tests/macos`.

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| M1 | bug | Corrigé, test ajouté | `window/session/frame_link.rs`, `pacing.rs` | Attente du lien d’affichage expirée jamais effacée (fenêtre couverte, écran en veille) : la boucle ne dormait plus. Abandonnée à l’échéance (`expire`) ; logique d’attente séparée du lien AppKit (`Beat`) et testée. La sonde native mesure aussi le processeur dans le Dock (16 à 29 ms en 0,9 s). |
| M2 | bug | Corrigé | `window/event_loop.rs`, `window/change.rs` | `Session::change` et l’affichage d’une fenêtre ouverte appelaient AppKit sous l’emprunt des sessions : notifications synchrones (clé, taille) perdues. AppKit est appelé hors emprunt (`change::apply`), l’état relevé ensuite (`Session::changed`). Sonde native : taille demandée par l’application, image refaite à cette taille (non discriminant : la perte d’une notification n’est pas observable depuis l’application). |
| M3 | bug | Corrigé, test ajouté | `metal/renderer.rs`, `metal/resources.rs` | Image traduite sans drawable : une image nouvelle restait en cache sans ses mipmaps (calculés à l’encodage) et un groupe filtré enregistré sans avoir été dessiné ; réduite, l’image aurait montré des niveaux non remplis. Ils sont oubliés (`discard_unrendered`), l’image suivante les refait. |
| M4 | bug | Corrigé, test ajouté | `window/app_delegate.rs` | Délégué de l’application : Quitter (Dock, menu, `terminate:`) termine la boucle et `run_windows` rend `Ok` ; fermeture de session : boucle terminée, applications détruites, puis processus terminé par AppKit (`NSTerminateLater`). Quitter actif sans fenêtre clé. Sonde native : sortie par `terminate:`, garde `atexit` (sans délégué : SIGABRT). |
| M5 | bug | Corrigé, test ajouté | `window/session/paint.rs`, `lifecycle.rs` | Fenêtre affichée mais entièrement couverte : ni peinte ni réveillée, image refaite quand elle redevient visible. Sonde native : fenêtre opaque par-dessus, drawables gardés à un pixel (sans la correction : 1 000) ; vérification sautée quand le serveur de fenêtres ne voit pas la couverture (bureau de l’utilisateur : Space, plein écran). |
| M6 | mémoire | Corrigé, test ajouté | `metal/resources.rs`, `metal/submit.rs` | `Trim` libère aussi les cibles prises par la dernière image et le dernier tampon de commandes terminé sans erreur (une image en erreur reste lue pour le repli). |
| M7 | bug probable | Corrigé | `window/event_loop.rs` | Pas d’activation quand aucune fenêtre ne la demande ; vérifié sur le Mac (sonde : `isActive` faux, vrai si l’activation est forcée). Non exigé par la sonde : macOS peut donner le premier plan à une application lancée depuis un processus qui l’a. Les fenêtres de la sonde sont flottantes : sans premier plan, celles de l’utilisateur les couvraient (M5). |
| M8 | bug | Corrigé | `window/change.rs`, `event_loop.rs` | `dragging` propre à la fenêtre du geste ; `GRIP` terminé à la fermeture de sa fenêtre et effacé à la fin de la boucle. |
| M9 | bug | Corrigé, test ajouté | `window/event_loop.rs` | Réveil consommé par une boucle AppKit imbriquée : l’indicateur restait levé et tous les réveils suivants étaient ignorés. Il repart de zéro à chaque tour. Sonde native : boucle imbriquée qui consomme un réveil, chien de garde (blocage = échec après 20 s). |
| M10 | performance | Corrigé, test ajouté | `metal/submit.rs` | Calque vide que ne lit aucun filtre : texture vide au lieu d’une cible, sans passage (groupe d’opacité vide : 32 Ko de cible en moins). |
| M11 | latent | Corrigé, test ajouté | `metal/resources.rs`, `linux/vulkan/textures.rs` | La rastérisation d’une icône vectorielle retient son `Arc`, dont l’adresse sert de clé (Metal et Vulkan ; test Metal). |
| M12 | nettoyage | Corrigé | `metal/renderer.rs` | Image fermée (`resources.end()`) et tampons recyclés quand la cible ou l’encodage échouent. |
| M13 | nettoyage | Corrigé | divers | Commentaires en double retirés, `// SAFETY :` de `offscreen.rs`, messages de la sortie d’erreur hors diagnostic en anglais. |

## Compilateur (`crates/ui-macros`)

| # | Gravité | État | Fichier | Constat |
| --- | --- | --- | --- | --- |
| C1 | bug | Corrigé, test ajouté | `css/framework/selectors/parse.rs:391` | `a[href^]` (opérateur sans `=`) : tranche hors bornes, la macro panique. |
| C2 | bug | Corrigé, test ajouté | `css/framework/values/tokens.rs:216` | `group_len` rend des octets, utilisés comme nombre de caractères : panique avec un caractère non ASCII dans `rgb()`. |
| C3 | bug | Corrigé | `input/files.rs:72`, `css/framework/model.rs:71` | BOM UTF-8 jamais retiré des `.ui` et des CSS (fichier enregistré « UTF-8 avec BOM ») : erreurs incompréhensibles, variables de `:root` perdues en mode `framework`. |
| C4 | bug | Corrigé, test ajouté | `css/framework/values/normalize.rs:14` | Chaîne entre guillemets recopiée octet par octet : `"Hélvetica"` devient `"HÃ©lvetica"`. |
| C5 | bug probable | Corrigé | `codegen/properties.rs:74` | Expression collée en `({expr} as f32)` : `style:gap={self.base + 4}` devient `self.base + (4 as f32)`. |
| C6 | bug probable | Corrigé, test ajouté | `codegen/properties.rs:94` | Identifiant seul de `style:nom={…}` gardé comme valeur de l’appelant (champ `expression` de `StyleProp`) ; les chaînes littérales restent des mots-clés. |
| C7 | bug probable | Corrigé, test ajouté | `template/generate/components.rs:101` | Prop nommée comme un mot-clé de Rust en identifiant brut (`type` → `r#type`) ; `__ui_*`, `_`, `crate`, `self`, `Self`, `super` refusés. |
| C8 | bug probable | Corrigé | `input/files.rs` | `.ui` en CRLF : `\r` gardés dans `<pre>`. Fins de ligne normalisées à la lecture, avec le BOM (C3). |
| C9 | bug probable | Corrigé, test ajouté | `classes.rs:80` | Feuille en cache revalidée par taille et date du fichier (`framework::load`). |
| C10 | bug probable | Corrigé, test ajouté | `codegen/properties.rs:66`, `template/generate/select.rs` | `overflow` et la valeur d’une `<option>` liés une fois (5 évaluations → 3 dans le test). |
| C11 | bug probable | Corrigé | `tailwind/resolve.rs:100` | `calc(0px + 50%)` replié en `50px`. |
| C12 | mineur | Corrigé, tests ajoutés | `svg/paint.rs:229`, `css/inline.rs:84` | Nombre SVG hors de la plage de `f32` refusé, situé sur l’attribut ; borne de durée `>=` (`u32::MAX as f32` vaut 2^32), aussi dans les listes `animation`. |
| C13 | mineur | Corrigé, tests ajoutés | `template/markup/parser.rs`, `css/parser/reader.rs`, `css/framework/syntax.rs` | Templates : 128 niveaux, `{#if}` et `{#each}` compris ; feuille stricte : 32 blocs imbriqués, erreur située ; feuille de framework : bloc trop profond sauté et signalé. |
| C14 | nettoyage | Corrigé | divers | Code mort vérifié puis retiré ; messages des fichiers inclus selon leur nature (`@import` circulaire signalé) ; `strip_prolog` saute les commentaires de tête comme annoncé (fichier Illustrator). `normalize.rs:6` était déjà corrigé (`4d16e2c`). Reste : le lecteur de fichiers limite une macro à 2 Mio, avant la limite de 8 Mio de `framework::load` (message seul changé). |
