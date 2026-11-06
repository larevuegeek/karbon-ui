> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# Mesurer une fenêtre pendant son utilisation

`--profile` active des mesures dans le chemin de peinture Windows habituel.
Le profilage est désactivé par défaut et ne crée ni timer ni boucle de rendu.
Il conserve au maximum 1 024 échantillons par fenêtre, soit 40 Kio de valeurs
chronométriques. Un snapshot ne demande pas de peinture supplémentaire.

## Lancer et lire un rapport

```powershell
cargo run --offline --release --bin rustmusic-demo --target-dir target/presentation-check -- --profile --artwork-dir artifacts/artwork-fixtures-v1
```

Utiliser l’écran normalement : attendre le chargement, défiler, changer de page,
redimensionner puis laisser au repos. F8 affiche les compteurs UI et une ligne
`Presentation : { ... }` contenant un objet JSON. La fermeture normale de la
fenêtre écrit un dernier rapport avec `reason: "close"`. Un arrêt forcé du
processus ne garantit pas ce dernier rapport.

La galerie accepte aussi `cargo run --offline --release -- --profile`.
Dans une application, activer `WindowOptions { profile: true, ..WindowOptions::DEFAULT }`.
Les fenêtres secondaires doivent activer leur propre option.
`--benchmark` et `--profile` sont deux modes distincts ; leur combinaison est refusée.

Pour conserver les lignes dans un fichier avec PowerShell :

```powershell
& './target/presentation-check/release/rustmusic-demo.exe' --profile --artwork-dir artifacts/artwork-fixtures-v1 | Tee-Object -FilePath artifacts/presentation-session.log
```

Les messages habituels partagent la sortie standard. Pour extraire les objets :

```powershell
Get-Content artifacts/presentation-session.log |
    Where-Object { $_.StartsWith('Presentation : ') } |
    ForEach-Object { $_.Substring('Presentation : '.Length) | ConvertFrom-Json }
```

## Ce que mesure `window-submit-direct-dib-v2`

| Champ de `stages` | Périmètre |
| --- | --- |
| `prepare` | Lecture de la géométrie/DPI et préparation UI faite pendant cette peinture. |
| `render_readback` | Direct2D, rendu GPU ou WARP et récupération des pixels CPU. |
| `surface` | Vérification des dimensions et création éventuelle du DIB persistant. |
| `submit` | Appel `UpdateLayeredWindow` jusqu’à son retour. |
| `total` | Début de préparation jusqu’après la soumission. |

Depuis v2, la surface est préparée avant le rendu. Le renderer relit la texture
directement dans les pixels du DIB persistant. Le `Vec<u8>` temporaire et la
copie de ce Vec vers le DIB ont été supprimés du chemin de peinture normal.
À 1 360 × 920 pixels physiques, cela évite une allocation de 5 004 800 octets
et une copie de cette taille par peinture. La relecture GPU → RAM demeure,
ainsi que les allocations internes des bibliothèques natives.

L’ancienne version `window-submit-v1` incluait la copie dans `submit` et la
libération du Vec dans `total`. Comparer les étapes v1/v2 nécessite de tenir
compte de ce changement de périmètre. Le benchmark hors écran conserve une
sortie Vec volontairement : il n’utilise pas ce chemin de fenêtre.

Chaque étape expose médiane, p95 et maximum, calculés au rang supérieur parmi
les **dernières 1 024 peintures réussies**. `frames`, `failed_paints`,
`renderer_recoveries`, `surface_creations` et `size_changes` sont des compteurs
cumulatifs depuis le début du segment. Les dimensions physiques et le facteur
DPI du dernier échantillon sont indiqués. Plusieurs tailles ou thèmes peuvent
donc coexister dans la fenêtre statistique ; séparer les sessions pour les comparer.

Les premières peintures, dont celle effectuée avant l’affichage initial,
sont incluses. Aucune chauffe n’est supprimée. Une peinture de taille nulle
n’ajoute pas d’échantillon. Un échec est compté sans ajouter un temps réussi.
F2 écrit le segment précédent avec `reason: "backend-switch"`, puis remet les
mesures à zéro. Une recréation du renderer écrit `renderer-recovery` et démarre
un nouveau segment ; sa première peinture inclut le coût de récupération.

Le total n’inclut pas les traitements d’entrée, les tâches de fond, les
préparations déjà réalisées avant WM_PAINT, BeginPaint/EndPaint, ni les
synchronisations de saisie et d’accessibilité après la peinture. Il ne mesure
pas non plus la latence entrée → écran, l’attente de composition ou le balayage
du moniteur. Le retour de soumission est la limite du chronométrage natif.
`UpdateLayeredWindow` met à jour toute la fenêtre, même si seule une bande a
été invalidée ; voir son [contrat Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-updatelayeredwindow).

## CPU et mémoire

`process` contient des instantanés de RAM et le temps CPU du **processus entier**.
Les durées vont de l’activation du segment au snapshot, repos compris. Le CPU
additionne utilisateur et noyau sur tous les threads et peut dépasser 100 %
car il est exprimé par rapport à un seul cœur. Les autres fenêtres du processus
et les workers contribuent à ces valeurs ; ce ne sont pas des mesures CPU/RAM
isolées de la fenêtre portant le rapport.

Le working set décrit la mémoire résidente, les octets privés la mémoire
privée engagée. Le pic concerne le processus depuis son lancement et ne se
réinitialise pas avec F2. La VRAM n’est pas mesurée. `process: null` indique
qu’un relevé natif a échoué ; le profilage ne fait pas échouer l’application.

Les pourcentages de très courtes sessions sont peu représentatifs à cause de
la granularité des compteurs CPU. Faire des phases d’au moins dix secondes.
Pour mesurer une période de repos, prendre deux snapshots et calculer la
différence de leurs `cpu_ms` et `session_wall_ms`. Vérifier aussi que `frames`
reste stable, en tenant compte d’un caret ou d’une info-bulle pouvant demander
une peinture ponctuelle. La création du rapport a elle-même un coût faible
qui peut apparaître dans les relevés suivants.

## Vérification et suite

Quatre tests supplémentaires vérifient l’option désactivée par défaut, la borne
d’échantillonnage, les percentiles, la lecture CPU, l’isolation entre fenêtres,
la réutilisation du DIB, le resize et l’émission à la destruction.
La suite complète compte 204 tests. Les vérifications GPU/WARP comparent aussi
tous les pixels des deux chemins, sur DIB réutilisé, avec alpha, texte, image,
clipping et échelles 1/1,5/2. Une destination de taille incorrecte est refusée
avant écriture. Un autre test change 96 fois d’image, avec des redimensionnements :
le cache natif ne conserve que l’image actuelle puis se vide avec une scène
sans image. Les références faibles confirment la libération des sources
précédentes. Cela ne démontre pas une borne de RAM globale ou de VRAM.

Le [rapport de contrôle v2](../../artifacts/presentation-direct-smoke.json) provient d’un test
release isolé : fenêtre **cachée**, WARP, trois peintures, une modification de
taille, deux surfaces et aucun échec. Il valide les appels natifs et le JSON ;
ses temps ne constituent pas un benchmark de RustMusic ou du bureau visible.
Toutes les fenêtres de ce test sont détruites. Les premières sessions visibles
avec la file sont maintenant consignées dans la [recette](queue-validation.md),
qui distingue versions, tailles, interactions utilisateur et protocoles.

Le lanceur local ouvre la file et deux sessions successives :

```powershell
cargo build --offline --release --bin rustmusic-demo --example profile_session --target-dir target/presentation-direct-check
& './target/presentation-direct-check/release/examples/profile_session.exe'
```

Il ouvre d’abord GPU, puis WARP après fermeture normale de la première fenêtre,
avec le même jeu d’images. Les sorties sont écrites dans
`artifacts/presentation-visible-gpu.log` et `artifacts/presentation-visible-warp.log`.
Ces deux fichiers sont remplacés à chaque lancement. Il ne pilote pas les
interactions : effectuer le même scénario dans les deux fenêtres.
L’ancien [rapport v1](../../artifacts/presentation-smoke.json) est conservé ; les
deux petits contrôles natifs ne permettent pas de chiffrer un gain de performances.

Les premières sessions ont couvert images, déplacement, navigation, repos et
RAM ; le resize compact et la transparence ont été vérifiés sur GPU. Répéter
les séries visibles sans interventions concurrentes et sur une plus longue
durée avant de tirer une conclusion de performance. Le protocole
[hors écran](performance.md) reste disponible pour les mesures répétables du
rendu seul. La présentation sans relecture CPU reste à étudier ensuite.

Code : `src/platform/windows/presentation.rs` pour le collecteur borné,
`drawing.rs` pour les points de mesure, `events.rs` pour la fermeture et F8.
