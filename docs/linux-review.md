# Revue du backend Linux (2026-11-06)

Relecture complète de `src/platform/linux/` (116 fichiers, 23 500 lignes) et de
`src/platform/shared/`, menée depuis Windows : six relectures indépendantes par thème
(liaisons C, Vulkan, Wayland, X11 et IBus, Cairo/Pango/shell, AT-SPI), chaque constat
retenu ayant été revérifié dans le code. Les transcriptions à la main sont exactes
(tables des protocoles Wayland message par message, structures XCB et XInput champ par
champ, structures et constantes Vulkan, signatures C) : les défauts sont dans la logique
autour.

Ce fichier suit chaque constat jusqu'à sa clôture. États : **Corrigé** (depuis Windows :
`cargo clippy --target x86_64-unknown-linux-gnu` passe, mais rien n'a tourné sous
Linux — à valider), **À faire**, **Proposé** (remaniement à décider), **Accepté**
(comportement assumé, documenté).

## À valider sous Linux avant tout

État : X11 essayé sous XWayland (Ubuntu 22.04, GNOME 42, `KARBON_BACKEND=x11`,
RustMusic) le 2026-11-06 : tout fonctionne à 100 % ; à 200 %, l'image est floue, ce qui
est le comportement de XWayland sous GNOME ≤ 46 (toutes les applications X11 y sont
dessinées à l'échelle 1 puis agrandies ; `Xft.dpi` reste à 96), pas un défaut du
backend — une session Xorg à 200 % (`Xft.dpi` = 192) doit rendre net.
`cargo test` sur cette machine (VirtualBox, Vulkan par llvmpipe, **Cairo 1.16**) :
285 tests passés, 13 échecs, tous expliqués et traités le jour même :
`decodes_embedded_png_and_rejects_invalid_data` venait du correctif G3 (l'en-tête
d'un format inconnu de l'inspecteur — GIF, WebP, octets quelconques — faisait
échouer le décodage ; corrigé : l'en-tête n'est vérifié que pour PNG et JPEG) ; les
12 autres comparaient Vulkan à Cairo sur des scènes avec du texte, or Cairo 1.16 place
les glyphes au pixel entier et non au quart de pixel (Cairo ≥ 1.17.4) sur lequel le
renderer Vulkan et les seuils sont calés (écarts de 0,2 % à 4,5 % des canaux). Ces
tests se sautent désormais sur un Cairo sans positionnement sous-pixel
(`ffi::cairo::subpixel_glyphs`) ; le rendu à l'écran n'est pas en cause. Après ces deux
corrections (`9d86515`), **la suite complète passe sur cette machine** (point 1 fait :
bibliothèque, crates de `tests/`, exemple, compilateur). Restent les points 2 à 4.

1. ~~`cargo test --workspace --all-targets`~~ — fait le 2026-11-06 sur Ubuntu 22.04 (dont les nouveaux tests de `shell/scroll.rs`
   et `shell/composition.rs`), puis les exports de référence Cairo (inchangés attendus :
   aucune correction ne touche le chemin de dessin Cairo hors écran).
2. RustMusic sous Wayland et sous `KARBON_BACKEND=x11` : saisie IBus (touche morte,
   Ctrl+Maj+U), copier-coller dans les deux sens, molette sur un compositeur sans
   `wl_pointer` 8 (Sway < 1.8, Weston) si disponible, fenêtre réduite puis restaurée.
3. Orca : lecture d'un champ par mots (`GetTextAtOffset`, frontières début et fin de
   mot), position et taille d'un élément, arrêt d'Orca pendant que l'application tourne
   (les événements doivent cesser d'être calculés).
4. Disposition clavier non latine (russe) : Ctrl+C/V/X/A doivent fonctionner.

## Liaisons C (`ffi/`)

| N° | Gravité | État | Où | Constat et correctif |
| --- | --- | --- | --- | --- |
| F1 | important | Corrigé | `ffi/cairo.rs`, `bitmap.rs` | `Context` et `Pattern` d'une surface ne l'empruntaient pas : `into_pixels` ou la fin du `Bitmap` pouvaient précéder un contexte vivant (pointeur pendouillant, tenu par un commentaire). `Context<'a>` et `Pattern<'a>` empruntent la surface (`Surface`) ; dégradés et groupes sont `'static`. |
| F2 | important | Corrigé | `ffi/pango/mod.rs` | `log_attrs` empruntait `&self` alors que `set_text`… prenaient `&self` : une tranche gardée pendant une modification n'était pas refusée. Modificateurs en `&mut self`. |
| F3 | mineur | Corrigé | `ffi/pango/types.rs`, `graphics/text/glyphs.rs` | `GLYPH_NO_INK` ne couvrait pas `PANGO_GLYPH_EMPTY` (0x0FFFFFFF) : glyphes vides envoyés à l'atlas Vulkan. `GlyphInfo::has_ink`. |
| F4 | sécurité (latent) | Corrigé | `ffi/wayland.rs` | La table `UNTYPED` a 8 entrées sans garde ; une transcription à 9 arguments ferait lire libwayland au-delà. `assert!` à la compilation dans `message`. |
| F5 | mineur | Corrigé | `ffi/dbus.rs` | Échec d'`open_container` ignoré puis `close_container` sur un itérateur non initialisé. Drapeau par conteneur. |
| F6 | mineur | Corrigé | `ffi/gio.rs` | Schéma relocalisable (sans chemin) : `g_settings_new_full` interrompt le processus. `g_settings_schema_get_path` vérifié ; `GSettings` tenu par `glib::Object` (plus de `g_object_unref` redéclaré). |
| F7 | mineur | Corrigé | `ffi/dbus.rs` | `DBusMessageIter` figé à 72 octets (LP64). Assertion `size_of::<usize>() == 8`. |
| F8 | lisibilité | Corrigé | `ffi/wayland.rs`, `wayland/display.rs` | `RawMessage` redéclaré pour lire la signature : `Message::signature()`. |
| F9 | lisibilité | Corrigé | `ffi/xcb/` | `xcb.rs` (540 lignes) découpé : `mod.rs` (chargement `dlopen`, tables), `types.rs`, `events.rs`, `constants.rs` ; liaisons et constantes mortes retirées ; `allow(dead_code)` limité aux structures transcrites ; `XInput` (table) renommé `XInputLib` ; alias `xcb_key_press_event_t`, `xcb_motion_notify_event_t`. |
| F10 | lisibilité | Corrigé | `ffi/pango/` | `pango.rs` (505 lignes) découpé en `mod.rs` et `types.rs`. |
| F11 | lisibilité | Corrigé | `ffi/mod.rs`, `docs/architecture.md` | Docs : bibliothèques liées contre chargées, « tailles vérifiées par le compilateur C » → assertions d'après les en-têtes. |
| F12 | lisibilité | Proposé | `ffi/xcb/mod.rs` | `xinput()` rouvre `libxcb-xinput.so.0` pour une donnée (`xcb_input_id`) : étendre `library!` aux entrées `static`. |
| F13 | lisibilité | Proposé | `ffi/dbus.rs` | Deux conventions pour les chaînes C (`dbus::c` supprime les nuls, `ffi::c_string` refuse). Garder celle de `ffi/mod.rs`. |

## Renderer Vulkan (`vulkan/`)

| N° | Gravité | État | Où | Constat et correctif |
| --- | --- | --- | --- | --- |
| V1 | important | Corrigé | `vulkan/present.rs` | `finish` recyclait couches et ressources *avant* de propager l'échec de `vkWaitForFences` (délai, périphérique perdu) : réutilisation de tampons que le GPU lit encore. Sur échec, l'image reste en vol et l'erreur remonte ; la fenêtre passe à Cairo, le renderer attend l'appareil à sa destruction. |
| V2 | sécurité | Corrigé | `vulkan/device.rs`, `memory.rs` | Limites du pilote jamais lues (16384 en dur ; `maxImageDimension2D`/`maxFramebuffer*` valent 4096 sur certains GPU) : image hors limite = comportement indéfini. `Gpu::limits`, `max_image_edge()` vérifié à chaque création d'image. |
| V3 | important | Corrigé (partiel) | `vulkan/textures.rs` | Une allocation Vulkan par texture, et les rastérisations sont clés par position sous-pixel : un défilement fractionnaire en crée une par icône et par image, jusqu'à `maxMemoryAllocationCount` (4096 sur NVIDIA) → `VK_ERROR_TOO_MANY_OBJECTS`, Vulkan coupé pour la fenêtre. Plafond `MAX_TEXTURES` (1024 par sorte). **Proposé** : sous-allouer par blocs, ou quantifier l'offset de la clé au quart de pixel (change légèrement le rendu). |
| V4 | mineur | Corrigé | `vulkan/device.rs` | Appareil fuité si `DeviceFns::load` échoue (l'instance était détruite sans `vkDestroyDevice`). |
| V5 | mineur | Corrigé | `vulkan/pipeline.rs` | Module de sommets fuité si celui des fragments échoue. |
| V6 | mineur | Corrigé | `vulkan/resources.rs`, `staging.rs` | `reset` oubliait les octets en attente du tampon intermédiaire. |
| V7 | mineur | À faire | `vulkan/memory.rs`, `encode/effects.rs`, `wayland/window/paint.rs` | Une seule image ou un seul groupe filtré trop grand fait échouer l'image entière et désactive Vulkan pour la fenêtre, alors que `raster.rs` saute silencieusement une rastérisation trop grande. Harmoniser : sauter l'élément. |
| V8 | mineur | Corrigé | `vulkan/device.rs` | `KARBON_VULKAN_VALIDATION` sans la couche installée désactivait Vulkan en silence. Présence vérifiée, message sur `stderr`. |
| V9 | lisibilité | Corrigé | `shared/gpu.rs`, `shaders/common.glsl`, `ffi/vulkan/pipeline.rs` | Commentaire « étape 1 » et `allow(dead_code)` global périmés (seul `Frame` est propre à Metal) ; chemin `gpu/instance.rs` erroné ; champ `antID` → `constantID`. |
| V10 | lisibilité | Proposé | `vulkan/encode/{shapes,scopes}.rs` | Copies quasi textuelles de `macos/metal/encode/` (30 et 17 lignes de différence, imports et masque) qui ne dépendent que de `shared/` : candidats à `platform/shared/gpu/encode/` si le choix du 2026-11-05 (copie propre à Linux) est revu. `text.rs`, `raster.rs`, `effects.rs`, `mod.rs` divergent vraiment. |
| V11 | lisibilité | Proposé | `vulkan/device.rs`, `submit.rs`, `textures.rs` | `Gpu::device` (~140 lignes : choix du GPU + création + tables), `Renderer::encode`, `Textures::record` (chaîne de mipmaps) à découper. |

## Boucle Wayland (`wayland/`)

| N° | Gravité | État | Où | Constat et correctif |
| --- | --- | --- | --- | --- |
| W1 | sécurité | Corrigé | `wayland/clipboard.rs` | Lecture du presse-papiers sans plafond de taille (un tube local débite des centaines de Mo en 2 s, puis `from_utf8_lossy` double). `READ_LIMIT` 16 Mio. |
| W2 | sécurité | Corrigé | `wayland/clipboard.rs` | Un thread bloquant par `wl_data_source.send` sans borne : un client qui demande sans lire fait croître les threads. `MAX_SENDERS` (8) ; au-delà, le descripteur est fermé. |
| W3 | sécurité | Corrigé | `wayland/events.rs` | Échelles du compositeur bornées par le bas seulement : `cursor_size() * scale` et les tailles de tampon débordaient. `MAX_SCALE` (8, ×120 pour la fractionnaire). |
| W4 | sécurité | Corrigé | `shell/keyboard.rs` | `wl_keyboard.keymap` : projection de `size` octets sans vérifier le fichier (SIGBUS au-delà). `fstat`, taille bornée. |
| W5 | important | Corrigé | `wayland/text_input.rs`, `shell/composition.rs` | Ordre du `done` : les Retour arrière/Suppr étaient rejoués pendant un préedit encore affiché. Ordre du protocole : composition retirée, suppression, texte validé, nouvelle composition. |
| W6 | important | Corrigé | `wayland/clipboard.rs` | Offres de glisser-déposer jamais rendues (fuite de proxies et de types à chaque survol). Rendues au `leave`. |
| W7 | mineur | Corrigé | `wayland/display.rs` | `global_remove` d'une sortie oubliait le proxy. |
| W8 | mineur | Corrigé | `wayland/seat.rs` | Échec de chargement du thème de curseurs retenté à chaque mouvement du pointeur. Mémorisé. |
| W9 | mineur | À faire | `wayland/text_input.rs` | `done.serial` ignoré : comparer au nombre de `commit` envoyés avant de renvoyer `set_surrounding_text` (certaines méthodes bouclent sinon). À reproduire sous Linux. |
| W10 | mineur | Corrigé | `wayland/input.rs` | `axis_discrete` (wl_pointer 5-7) ignoré : molette en pixels sur les compositeurs sans v8. Traité comme `value120 × 120`. |
| W11 | mineur | Corrigé | `wayland/window/{mod,paint}.rs` | État `suspended` : `hidden()` libérait tout, puis la configuration repeignait et réallouait aussitôt. Drapeau `suspended` ; la configuration est validée par un envoi sans tampon. |
| W12 | mineur | Corrigé | `wayland/mod.rs` | `wl_display_flush` en EAGAIN silencieux : requêtes en attente jusqu'au prochain événement. `POLLOUT` demandé quand le tampon d'envoi est plein. |
| W13 | mineur | Corrigé | `wayland/display.rs` | Argument lu avant l'examen de la lettre de type (une signature finissant par `?` lirait un cran de trop). |
| W14 | sécurité | Accepté | `wayland/display.rs` | `wl_array` copié à la taille annoncée : contrat de libwayland (déjà validé côté socket). |
| W15 | lisibilité | Corrigé | `wayland/mod.rs`, `display.rs` | En-tête (« dessinées par Cairo dans des tampons `wl_shm` ») et message d'erreur (« X11 backend is not written yet ») périmés. |
| W16 | lisibilité | Proposé | `wayland/mod.rs` ↔ `x11/mod.rs` | ~150 lignes identiques à la signature près (`record`, `with`, `ids`, `execute`, `action`, `send` hors presse-papiers, corps de `service`, `timeout`) ; lecture d'un tube avec délai en deux exemplaires. Un `shell::Loop` (trait avec `windows()`, `paint`, `clipboard`) éviterait de corriger deux fois chaque bug. À faire sous Linux, où les deux boucles se testent. |
| W17 | lisibilité | Corrigé | `shell/scroll.rs`, `shell/composition.rs` | Cran de molette et « Maj → horizontal » en trois exemplaires ; séquencement de composition (`Commit`/`Preedit` → événements du moteur) en deux. Réunis dans le socle, avec tests. |
| W18 | lisibilité | Proposé | `wayland/display.rs`, `events.rs` | Tuples anonymes `(u32, Ptr, i32)` des sorties lus par `.2` : une `struct Output { name, proxy, scale }`. |

## Boucle X11 et IBus (`x11/`, `ime/`)

| N° | Gravité | État | Où | Constat et correctif |
| --- | --- | --- | --- | --- |
| X1 | important | Corrigé | `x11/events.rs`, `x11/mod.rs`, `x11/clipboard.rs` | Événements gardés pendant une lecture du presse-papiers rejoués *après* ceux lus ensuite (relâchement traité avant l'appui). `VecDeque`, consommée avant la connexion à chaque tour. |
| X2 | important | Corrigé | `x11/events.rs` | Détection de répétition (appui jumeau) cherchée dans la file XCB alors que le jumeau pouvait être différé. Même source unique. **Proposé** : `XkbPerClientFlags(DetectableAutoRepeat)` supprimerait aussi le faux relâchement quand le jumeau n'est pas encore lu. |
| X3 | important | Corrigé | `x11/events.rs`, `ffi/xcb/events.rs` | `FocusIn`/`FocusOut` de mode `NotifyGrab`/`NotifyUngrab` ou de détail `NotifyPointer` traités comme une perte de focus : Alt+Tab validait la composition de force. Ignorés, comme GDK. |
| X4 | important | Corrigé | `x11/clipboard.rs`, `ffi/xcb/` | Propriété de la sélection supposée acquise (ICCCM : une demande plus ancienne est ignorée sans `SelectionClear`) : `read` rendait un texte périmé. `xcb_get_selection_owner` vérifié. |
| X5 | important | À faire | `ime/ibus.rs`, `x11/ime.rs` | `ProcessKeyEvent` synchrone (200 ms) : après le délai, la touche va à l'application *et* le moteur peut la valider (doublée) ; l'inverse (la considérer prise) la perdrait si le moteur la refuse après coup. La vraie correction est l'appel asynchrone de GTK (`dbus_pending_call`, touches suivantes retenues dans l'ordre, rejouées si `false`). Laissé en l'état, à faire sous Linux avec un moteur lent (libpinyin). |
| X6 | mineur | Corrigé | `x11/clipboard.rs` | Troncature au milieu d'un caractère UTF-8 ; `STRING` (Latin-1) recevait de l'UTF-8. |
| X7 | mineur | À faire | `x11/connection.rs` | Disposition clavier figée : écouter `MappingNotify` (et `_XKB_RULES_NAMES`) et recompiler. |
| X8 | — | Accepté | `x11/mod.rs`, `wayland/mod.rs` | `timeout()` ignore les réveils des fenêtres non prêtes (réduites) : même choix dans les deux boucles (une fenêtre cachée n'anime pas ; les tâches de fond réveillent par `eventfd`). |
| X9 | mineur | Corrigé | `x11/window/mod.rs`, `connection.rs` | `_NET_WM_PID` sans `WM_CLIENT_MACHINE` : Mutter ignorait le PID (pas de bouton « Forcer à quitter »). |
| X10 | lisibilité | Corrigé | `x11/ime.rs` | `expect("checked above")` ; `UiEvent` importé pour rien après le partage de la composition. |
| X11 | sécurité | Accepté | `x11/events.rs` | `WM_DELETE_WINDOW` et `_NET_WM_PING` arrivent par `ClientMessage`, que tout client X peut envoyer : comportement standard du protocole. `SelectionRequest` : les écritures vont sur la fenêtre du requérant ; aucune valeur serveur n'alimente une taille non bornée (propriétés plafonnées, XInput borné par `get`). |
| X12 | sécurité | Corrigé | `ime/ibus.rs` | `IBUS_ADDRESS` et le fichier `~/.config/ibus/bus/…` passaient tels quels à `dbus_connection_open_private`, qui accepte `unixexec:` (exécution d'un programme). Même utilisateur, donc pas une frontière, mais seul `unix:` est admis. |
| X13 | lisibilité | Corrigé | `ffi/xcb/` | Voir F9. |
| X14 | mineur | À faire | `x11/clipboard.rs` | Transferts `INCR` (textes plus grands qu'une requête) refusés ; Fcitx ; menu de la fenêtre au clic droit sur la barre (déjà listés dans `architecture.md`). |

## Cairo, Pango, polices, shell (`graphics/`, `fonts.rs`, `shell/`)

| N° | Gravité | État | Où | Constat et correctif |
| --- | --- | --- | --- | --- |
| G1 | important | Corrigé | `fonts.rs` | `load(&[])` remettait `desktop` à `false` après `follow_desktop()` : une application sans police embarquée perdait les polices du bureau (`system-ui` → générique fontconfig au lieu d'Inter sous GNOME). Couvert par `windows_follow_the_desktop_fonts…` (`load_fonts(&[])` après `follow_desktop`). |
| G2 | important | Corrigé | `shell/keyboard.rs`, `ffi/xkb.rs` | Raccourcis Ctrl+C/V/X/A perdus sous une disposition non latine (keysym cyrillique ≠ `c`). Repli sur le symbole latin du premier niveau de la touche (`xkb_keymap_key_get_syms_by_level`), comme Windows et macOS raisonnent sur la touche physique. |
| G3 | sécurité | Corrigé | `imaging.rs` | `decode()` (chemin UI) décodait l'image entière avant la limite de 4096 px : un PNG annonçant 15000 × 15000 faisait allouer ~900 Mio. En-tête vérifié d'abord (`inspect_image`), comme le décodeur de fond. |
| G4 | mineur | Corrigé | `offscreen.rs` | `/proc/self/status` absent (conteneur minimal) faisait échouer tout `process_sample` : repli sur `ru_maxrss` ; `unsafe { zeroed() }` sans commentaire SAFETY. |
| G5 | mineur | Corrigé | `shell/keyboard.rs` | `repeat_info(0, _)` pendant une touche tenue : répétition à 1 Hz au lieu de l'arrêt. |
| G6 | lisibilité | Corrigé | `linux/mod.rs`, `tests/mod.rs`, `shared/gpu.rs`, `platform/mod.rs`, `headless/backend.rs` | Commentaires et docs périmés (étapes « à faire », carte des modules sans `x11/`, `shell/`, `ime/`, `desktop.rs` ; « Linux sans compositeur Wayland → Unsupported » ; message headless sans Linux). |
| G7 | lisibilité | Proposé | `graphics/geometry.rs` ↔ `macos/graphics/geometry.rs` | `rounded`, `round_rect`, `polyline`, `icon`, `svg` identiques à la cible près (`cairo::Context` / `CGMutablePath`). Un trait `PathSink { move_to, line_to, curve_to, close }` dans `shared/geometry.rs` donnerait les mêmes flottants (exports macOS identiques à vérifier sur Mac). |
| G8 | lisibilité | Proposé | `graphics/layers.rs` ↔ `macos/graphics/layers.rs` | `Area`, `scale()`, `area()`, `layer_transform()`, seuils 4 194 304 / 16 777 216 dupliqués ; seule la lecture de la découpe courante dépend de l'OS. |
| G9 | lisibilité | Proposé | `graphics/text/draw.rs` ↔ `macos/graphics/text/draw.rs` | `decoration_rects` dupliqué (branches « custom », barré, surligne) ; seules les métriques du soulignement diffèrent. |
| G10 | lisibilité | Proposé | `fonts.rs` ↔ `macos/fonts.rs` | Table des alias (`Segoe UI`, `Consolas`, `-apple-system`…) dupliquée : un enum `Generic` dans `shared/`. |
| G11 | lisibilité | Proposé | `shared/damage.rs`, `shared/scopes.rs` | Deux `scope_end` ; celui de `scopes.rs` paniquerait en debug (`usize`) si `start` n'ouvrait pas une portée (les deux appelants le garantissent). Un seul, en `isize`. Touche macOS (même résultat). |
| G12 | nit | Accepté | `graphics/mod.rs`, `bitmap.rs` | Double validation 16384. |

## Accessibilité AT-SPI (`accessibility/`)

| N° | Gravité | État | Où | Constat et correctif |
| --- | --- | --- | --- | --- |
| A1 | important | Corrigé | `accessibility/interfaces.rs` | `GetPosition`/`GetSize` renvoyaient une structure `(ii)` au lieu de deux sorties `ii` (libatspi les lit par `=>ii`). |
| A2 | important | Corrigé | `accessibility/interfaces.rs` | Enfants de la racine dans l'ordre d'un `HashMap` : `GetChildAtIndex` et `GetIndexInParent` pouvaient se contredire avec deux fenêtres. Triés par `WindowId`. |
| A3 | sécurité | Accepté | `docs/architecture.md` | Tout processus de la session connecté au bus a11y peut lire l'arbre, appuyer, régler et modifier le texte des champs : contrat d'AT-SPI (GTK/Qt, UIA), socket limité à l'utilisateur, bornes du moteur. Désormais écrit dans l'architecture. |
| A4 | important | Corrigé | `accessibility/properties.rs`, `interfaces.rs` | `Properties.Set` répondait « succès » même quand le moteur refusait (NaN, hors bornes, lecture seule) ; `Get` d'une propriété inconnue donnait `UnknownMethod`. Erreur `InvalidArgs`. |
| A5 | important | Corrigé | `accessibility/mod.rs` | Écoutes jamais purgées si un client meurt sans `EventListenerDeregistered` : instantané rebâti et signaux émis pour personne jusqu'à la fin du processus. `NameOwnerChanged` écouté. |
| A6 | mineur | Corrigé | `ffi/dbus.rs` | Erreur renvoyée à un appel `NO_REPLY_EXPECTED` (`Message::error` ne vérifiait pas le drapeau, contrairement à `reply`). |
| A7 | mineur | Corrigé | `accessibility/text.rs` | `InsertText(position, text, length)` ignorait `length`. |
| A8 | mineur | Corrigé | `accessibility/text.rs` | `BOUNDARY_WORD_END` traité comme `WORD_START` ; `WORD_START` dans une suite de blancs rendait un « mot » d'un seul espace. Deux fonctions, conformes à ATK. **À valider avec Orca.** |
| A9 | mineur | Proposé | `accessibility/mod.rs` | Jusqu'à 3 s de blocage au démarrage (trois appels de 1 s : `GetAddress`, `Embed`, `GetRegisteredEvents`) si le démon a11y est gelé. Différer `GetRegisteredEvents` ou réduire le délai. |
| A10 | sécurité | Accepté | `accessibility/text.rs` | Chaînes reçues matérialisées jusqu'à la limite du bus (128 Mio) avant la borne `max_chars` du moteur : ni panique ni fuite. |
| A11 | lisibilité | Proposé | `accessibility/text.rs`, `windows/accessibility/text/mod.rs`, `ui/components/text_input/events.rs` | Trois définitions du mot (ponts Linux et Windows, Ctrl+flèche du moteur) aux conventions déjà divergentes. Un module `ui::components::text_input::boundaries` (caractère, mot, ligne, paragraphe → bornes en octets) servirait les deux ponts et le clavier, testé une fois hors OS. |
| A12 | lisibilité | Corrigé | `accessibility/interfaces.rs`, `text.rs`, `tests.rs` | `Prop` importé par un détour (`interfaces` ré-exportait `properties::Prop`). |
| A13 | lisibilité | Proposé | `accessibility/interfaces.rs` | Clone complet de l'élément (clusters, lignes) à chaque appel Text pour contourner l'emprunt ; emprunter `trees` séparément de `windows`. |
| A14 | lisibilité | Proposé | `accessibility/events.rs` | `announce` traite quatre sujets en 100 lignes : `announce_window`, `announce_focus`, `announce_text`, `announce_value`. |
| A15 | lisibilité | Corrigé | `docs/architecture.md` | Invariant mono-thread de libdbus (pas de `dbus_threads_init_default`) et surveillance du descripteur par les deux boucles, écrits. |

## Ce qui est bien fait (à préserver)

- Propriété systématique par `NonNull` + `Drop`, un `required()` commun, un commentaire
  `SAFETY` sur chaque appel ; ordre des champs pour l'ordre de libération.
- Entrées non fiables contrôlées avant tout découpage : `pixbuf.rs` (bits, canaux,
  stride, longueur), propriétés X plafonnées, XInput lu par `get()` → `Option` avec tests
  sur octets construits, keymap par `CStr::from_bytes_until_nul`.
- Répartiteur Wayland unique → file thread-locale, aucun emprunt pendant un appel à
  libwayland, descripteurs en `OwnedFd` dès la réception.
- Vulkan : assertions de taille et d'alignement sur chaque structure, `Rc<Gpu>` par
  ressource, barrières niveau par niveau, atlas qui ne peut pas déborder, tests comparés
  à Cairo à trois échelles.
- AT-SPI : `Reader`/`Writer` minimaux, types vérifiés avant lecture, indices bornés,
  événements calculés seulement avec un auditeur.
