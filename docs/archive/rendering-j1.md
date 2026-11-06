> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# Premier démonstrateur graphique J1

État au 2 novembre 2026. Le démonstrateur pose le contrat de dessin commun et
prouve un rendu Windows matériel et logiciel, y compris sur fond transparent.
Il prépare les composants du futur socle UI ; il ne remplace pas encore les
écrans de RustMusic.

## Fonctions disponibles

- Une scène indépendante des OS, décrite par une liste de commandes.
- Couleurs RGBA, surfaces et contours arrondis, traits, chemins ouverts et polygones remplis.
- Découpage rectangulaire imbriqué avec contrôle de l’équilibrage des zones.
- Texte DirectWrite avec tailles, graisse normale ou forte et police Segoe UI avec les replis natifs.
- Image PNG embarquée, décodée par GDI+ puis transférée comme bitmap prémultiplié.
- Thèmes clair et sombre, changeables à l’exécution.
- Direct2D sur D3D11 matériel, ou sur WARP pour le rendu logiciel.
- Fond de fenêtre totalement transparent entre les éléments ; surfaces locales opaques ou translucides selon la scène.
- Options natives de maintien au-dessus, passage intégral des clics et affichage sans activation.
- Fenêtre redimensionnable, gestion du DPI par moniteur et conversion des interactions vers les unités logiques.

Le paramètre de transparence ne diminue pas l’opacité globale des enfants.
La scène garde des fonds locaux sous ses textes pour rester lisible sur un
bureau variable. L’application pourra choisir librement les surfaces dessinées.

## Chemin des pixels

```text
Scene et DrawCommand
        ↓
Direct2D et DirectWrite
        ↓
Texture D3D11 BGRA à alpha prémultiplié
        ↓
Copie vers une texture de lecture puis mémoire CPU
        ↓
DIB persistant et UpdateLayeredWindow
        ↓
Composition avec les fenêtres du bureau
```

Le périphérique de rendu change selon le mode ; les commandes restent les
mêmes. `Auto` tente le matériel puis le logiciel à la création. En cas d’erreur
pendant le rendu, une recréation est tentée ; le mode Auto choisit alors WARP.
Une erreur persistante est propagée et ferme proprement le démonstrateur.
Un changement explicite par F2 conserve l’ancien renderer si la création du
nouveau échoue.

Les ressources COM sont libérées par des gardes Rust et restent sur leur
thread d’origine. Le mapping de la texture est refermé même en cas d’erreur.
La surface DIB est réutilisée tant que les dimensions ne changent pas. Les
formats de texte sont mis en cache ; les bitmaps utilisés par la scène sont
conservés jusqu’à leur retrait ou à la recréation de la cible.

Le décodage de l’unique image embarquée a lieu au démarrage. Le chargement
asynchrone, les caches généraux bornés et la priorité aux images visibles
restent à construire pour les collections de RustMusic.

## Vérifications effectuées

| Vérification | Résultat |
| --- | --- |
| Dessin natif avec WARP | Alpha nul, mélange prémultiplié, clip, image, polygone et texte vérifiés par lecture des pixels. |
| Dessin natif avec GPU matériel | Mêmes vérifications, exécutées explicitement sur cette machine. |
| Échelles 100 %, 150 % et 200 % | Vérifiées sur les surfaces des tests natifs avec recréation des cibles. |
| Décodage PNG | Dimensions et pixels vérifiés ; données invalides rejetées. |
| Structure de la scène | Clips déséquilibrés rejetés avant BeginDraw ; renderer réutilisable après une erreur de validation. |
| Fenêtre à l’écran | Texte, image, formes, mélange alpha et découpage examinés. |
| F2 | Passage GPU vers WARP et retour GPU vérifiés à l’écran. |
| F3 | Fond transparent observé en CPU et GPU ; retour au fond opaque et restauration de l’ombre. |
| F5 | Changement des thèmes clair et sombre vérifié. |
| Comportement de fenêtre | Déplacement, redimensionnement, agrandissement, restauration, réduction et fermeture vérifiés. |

Le passage réel entre plusieurs moniteurs de DPI différents n’a pas été
exercé ici. Les trois facteurs d’échelle ci-dessus concernent les tests de
rendu hors fenêtre ; la recette multi-écran du cahier des charges reste à faire.
La prise en charge de la perte physique d’un périphérique et les modes
passif/traversable n’ont pas fait l’objet d’une recette d’interaction complète.

## Première mesure technique

Mesure locale sur Windows x64 build 26300, Rust 1.99.1, compilation release.
Scène par défaut de 900 × 640 unités logiques à 150 %, soit 1350 × 960 pixels.
Après cinq images d’échauffement, trente images sont mesurées par backend.
Les ressources sont en cache ; la mesure inclut dessin et relecture CPU,
mais exclut UpdateLayeredWindow, la composition du bureau et le démarrage.

| Backend | Médiane | 95e percentile |
| --- | --- | --- |
| GPU Direct3D 11 | 1,55 ms | 1,84 ms |
| CPU WARP | 1,76 ms | 2,12 ms |

Ces valeurs décrivent une exécution de cette petite scène, pas une cadence
mesurée de l’application complète. La machine n’est pas encore une configuration
de référence figée. Elles ne démontrent pas un gain sur Tauri ou sur une UI
RustMusic alimentée par une grande bibliothèque.

Pour refaire la mesure :

```sh
cargo test --offline --release measure_gallery -- --ignored --nocapture
```

## Limites et suite

La présentation avec relecture simplifie la validation de l’alpha dans les deux
modes. Elle ajoute des transferts et doit être mesurée avant d’envisager des
animations complexes ou une grande surface à haute fréquence. Une présentation
par composition GPU sans relecture est une évolution possible.

Les chemins sont actuellement des polylignes et des polygones ; les courbes de
Bézier, les dégradés et les masques arbitraires restent à ajouter. Le texte est
affiché mais pas éditable. La famille de police est encore fixée au backend.
Le moteur de layout, les widgets, l’accessibilité des contrôles et l’IME
appartiennent aux prochains jalons.

Pour les clics, Windows utilise l’alpha nul de la fenêtre layered pour laisser
passer les zones vides ; `--click-through` rend toute la fenêtre traversable.
La déclaration de régions interactives indépendante du dessin reste à fournir
avec le routage d’événements du socle. Les règles natives utilisées sont décrites
dans [les fonctions des fenêtres Windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features).

L’architecture native s’appuie sur les contrats du SDK Windows et sur les
[principes d’interopérabilité Direct2D et D3D11](https://learn.microsoft.com/en-us/windows/win32/direct2d/devices-and-device-contexts).
Les autres OS ne sont pas encore implémentés.
