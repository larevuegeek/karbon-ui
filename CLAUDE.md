# karbon-ui : consignes du projet

Bibliothèque d'interface native en Rust destinée à remplacer Tauri (application cible :
RustMusic). Templates `.ui` et CSS compilés par `crates/ui-macros`, moteur indépendant de
l'OS dans `src/ui`, backends dans `src/platform`.

## Règles

- **Dépendances** : aucun crate tiers. Les crates officiels des éditeurs sont admis
  (`windows`, `windows-sys`, `windows-core`, `windows-numerics`, `libc`). Pour le
  backend macOS, la famille `objc2` est acceptée (`objc2`, `objc2-foundation`,
  `objc2-app-kit` et les autres `objc2-*` des frameworks Apple, `block2`), déclarée
  sous `[target.'cfg(target_os = "macos")'.dependencies]`. Sous Linux, `libc` seul ; les
  bibliothèques du bureau (Cairo, Pango, fontconfig…) sont liées par des déclarations
  `extern "C"` écrites à la main. Tout autre crate
  communautaire demande l'accord du propriétaire.
- **Commits** : jamais de `Co-Authored-By` ni de mention d'outil ou d'IA. Ne rien pousser
  ni créer de branche distante sans demande explicite.
- **Publication** : dépôt public `github.com/larevuegeek/karbon-ui`, frère de `karbon`
  (framework web) et `karbonjs`, licence MIT ou Apache-2.0 au choix de l'utilisateur ;
  `artifacts/` et `accueil.png` ne sont jamais suivis (captures de RustMusic avec de
  vraies pochettes). README en anglais, comme les dépôts frères ; `docs/` en français.
- **Langues** : API publique en anglais ; commentaires internes et documentation en
  français. Réponses au propriétaire en français.
- **Comportement** : un remaniement ne change pas les rendus. Les exports de référence
  restent identiques octet pour octet :
  `cargo run --release --example rustmusic_html -- --export x.png --size 1280x900 --scale 1.5 --renderer=software`
  (et `960x700`), voir [docs/next-session.md](docs/next-session.md).
- **Lisibilité** : un dossier par thème, fichiers de 500 lignes au plus sauf exception
  justifiée. Un `mod.rs` ne contient que la documentation du module, les `mod`, les
  `pub use` et la sélection par `cfg` ; toute logique va dans un fichier nommé d'après
  son rôle.
- L'application cible RustMusic n'est pas dans ce dépôt : son code sert de référence,
  en lecture seule.

## Vérifications avant de commiter

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --doc --workspace
cargo doc --no-deps --workspace
```

## Se repérer

- Carte du code et contrat des backends : [docs/architecture.md](docs/architecture.md),
  [src/platform/backend.rs](src/platform/backend.rs).
- Tests et contrat qu'un nouveau backend doit passer : [docs/tests.md](docs/tests.md).
- Compilateur des templates et du CSS : [crates/ui-macros/README.md](crates/ui-macros/README.md).
- Point de reprise : [docs/next-session.md](docs/next-session.md).
- Suivi de la revue du backend Linux (constats, états, remaniements proposés) :
  [docs/linux-review.md](docs/linux-review.md).

## Backend macOS (en cours)

`src/platform/macos/` est sélectionné comme `platform::Native` sous
`cfg(target_os = "macos")`. Fait : services hors écran (texte Core Text, polices
embarquées WOFF2, rendu Core Graphics avec filtres CPU, images et PNG par ImageIO,
compteurs du processus) ; aperçus, mesures et exports de référence passent par lui.
Boucle AppKit (`window/`) : fenêtres, souris, clavier, défilement, curseurs,
presse-papiers, minuteries, tâches de fond et commandes de fenêtre ;
`cargo run --release --example rustmusic_html` ouvre l'UI. IME (`NSTextInputClient`) et
accessibilité NSAccessibility faits. Renderer GPU Metal (`metal/`) : chemin par défaut
des fenêtres (`Auto`/`Gpu` avec un périphérique), Core Graphics en repli (`Software`,
pas de GPU, erreur GPU) et pour le hors écran ; étapes 1-4 faites (toute la scène, caches
bornés, stockage `Managed` testé par `KARBON_METAL_STORAGE=managed`) ; reste la mesure
sur un vrai Mac Intel.
Liaisons Apple : crates `objc2` (accord du 2026-11-04) ; `bitflags` y arrive en
dépendance transitive.

## Backend Linux (en cours)

`src/platform/linux/` est `platform::Native` sous `cfg(target_os = "linux")`. Pile choisie
le 2026-11-05 : Wayland natif puis X11, Pango + HarfBuzz + fontconfig, Cairo puis Vulkan ;
liaisons C écrites à la main (`ffi/`), `libc` seul crate. Fait : services hors écran
(texte Pango, polices embarquées par fontconfig privé, rendu Cairo avec les filtres CPU
communs de `platform/shared/`, images gdk-pixbuf, PNG, compteurs) et boucle Wayland
(`wayland/` : protocoles transcrits dans `wayland/protocol/`, fenêtres `wl_shm`, barre du
moteur sous GNOME, échelle fractionnaire, pointeur, clavier xkbcommon, presse-papiers) ;
`cargo run --release --example rustmusic_html` ouvre l'UI. Redessin partiel (zones de
`platform/shared/damage.rs`), méthode d'entrée (`zwp_text_input_v3`) et accessibilité
AT-SPI (`accessibility/`, libdbus-1) faits. Renderer Vulkan (`vulkan/`) : étape 1 faite
(infrastructure ; shaders GLSL → SPIR-V versionné par `vulkan/shaders/build.sh`,
glslang-tools) ; étape 2 faite (traduction de toutes les scènes, copie propre à Linux
de `macos/metal/encode/` : texte par atlas, images, rastérisations Cairo, filtres comme
le CPU, `backdrop-filter`, masques, texte rempli ; `--renderer=gpu --export` passe par
Vulkan) ; étape 3 faite (fenêtres Wayland présentées par Vulkan par défaut, chaîne
d'échange « mailbox », Cairo en repli et en mode logiciel). X11 en cours (`x11/`, XCB
chargé au lancement, socle commun `shell/` avec Wayland) : fenêtres et image (Vulkan ou
Cairo), pointeur (XInput 2.1), clavier, curseurs, presse-papiers et méthode d'entrée
IBus (D-Bus, `ime/`) faits, `KARBON_BACKEND=x11` pour l'essayer sous XWayland ; reste
Fcitx.
Les backends macOS et Windows se vérifient d'ici par `cargo clippy --target
aarch64-apple-darwin` et `--target x86_64-pc-windows-msvc`.
