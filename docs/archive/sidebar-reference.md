> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# Panneau RustMusic de référence

Une démo autonome reproduit le panneau gauche de la capture à **424 × 609 unités
logiques** : logo, source, import, navigation et bibliothèque. La galerie et
l’autre démo RustMusic gardent leur point d’entrée habituel.

```powershell
cargo run --offline --release --bin sidebar-demo --target-dir target/sidebar-check
```

Pour choisir le renderer, ajouter `-- --renderer=gpu` ou `-- --renderer=software`.
La fenêtre utilise tout son client, sans barre de titre supplémentaire. Fermer
avec **Alt+F4**. Elle respecte le facteur DPI Windows et sa taille minimale
correspond à la référence ; les cartes s’étendent si on l’agrandit.

## Ce qui est reproduit

Les marges, espacements, bordures, arrondis et positions sont composés avec le
layout commun. Les couleurs et les ressources ont été relevées dans le projet
RustMusic local, consulté sans modification.

| Bloc | Position et dimensions à 100 % |
| --- | --- |
| En-tête | 0, 0 ; 424 × 80 |
| Source | 17, 104 ; 383 × 75 |
| Import | 17, 197 ; hauteur 54 |
| Première ligne de navigation | 17, 289 ; hauteur 99 |
| Deuxième ligne de navigation | 17, 398 ; hauteur 99 |
| Bibliothèque | 17, 514 ; 383 × 82 |

Le texte utilise les véritables polices variables Figtree et Inter, embarquées
dans l’exécutable et chargées dans une collection DirectWrite privée. Aucune
installation de police n’est nécessaire. Le nom interne de cette version de
Figtree est `Figtree Light` ; l’axe `wght` choisit la graisse demandée.
Ce chargement des polices WOFF2 exige Windows 10 version 1809 ou ultérieure.

Les Material Symbols sont des contours vectoriels avec leurs évidements,
dessinés par Direct2D. Le script Node fourni convertit les courbes quadratiques
en courbes cubiques natives, sans les découper en petits segments. Node n’est
pas nécessaire pour compiler ni lancer la démo.
Le lissage et les métriques natifs peuvent légèrement différer du navigateur.

## Interactions

- Les six tuiles changent la sélection ; les flèches parcourent un historique
  borné à 128 entrées.
- Le bouton de repli masque ou réaffiche le corps du panneau.
- Source, bibliothèque et « … » ouvrent des menus ancrés.
- Fichier et Dossier ouvrent un dialogue de démonstration. Aucun fichier réel
  n’est importé dans ce test visuel.
- Tab, Entrée/Espace et Échap utilisent le focus et les couches du socle commun.

## Où intervenir

| Fichier | Responsabilité |
| --- | --- |
| `src/bin/sidebar-demo.rs` | Exécutable, fenêtre et export |
| `src/sidebar/mod.rs` | Point d’entrée Application |
| `src/sidebar/view.rs` | Structure du layout et couches temporaires |
| `src/sidebar/state.rs` | État et actions nommées |
| `src/sidebar/styles.rs` | Palette et recettes de mise en page |
| `src/sidebar/components/` | Dessin des composants et mesures des libellés |
| `src/sidebar/icons.rs` | Chargement des contours embarqués |
| `src/ui/rendering/font.rs` | Famille, graisse et déclaration d’une police |
| `src/platform/windows/fonts.rs` | Collection privée DirectWrite |
| `src/platform/windows/graphics.rs` | Conversion des commandes en dessin natif |

`Application::fonts()` déclare les ressources, `TextMeasurer::measure_font()`
mesure avec la même famille et graisse que le rendu, puis `Scene::font_text()`
émet une commande de texte monoligne. Les contrôles de cette démo sont des
widgets Rust personnalisés, avec les actions et sémantiques du moteur.

Les sources et licences des ressources sont dans
`src/sidebar/assets/SOURCES.md`.

## Aperçus et vérifications

```powershell
cargo run --offline --bin sidebar-demo --target-dir target/sidebar-check -- --export artifacts/sidebar-preview.png --renderer=software
cargo run --offline --bin sidebar-demo --target-dir target/sidebar-check -- --export artifacts/sidebar-preview-gpu.png --renderer=gpu
```

L’export appelle le même runtime et le même renderer natifs, sans ouvrir de
fenêtre. Il produit un PNG 424 × 609 à 100 % :
[logiciel](../../artifacts/sidebar-preview.png), [GPU](../../artifacts/sidebar-preview-gpu.png).

215 tests passent après la simplification de l’API, tests explicites compris.
Voir le [guide de syntaxe](../syntax.md) et [l’organisation](../architecture.md).
Les ajouts couvrent les polices
WOFF2 privées, leurs graisses, les évidements vectoriels en GPU/WARP à plusieurs
DPI, la disposition, les couches/focus, l’historique et la sélection rendue.
Une fenêtre native cachée vérifie aussi que la désactivation du chrome laisse
les clics de l’en-tête à l’application. Formatage et Clippy passent sans
avertissement ; l’exécutable release est compilé.

La vérification automatisée dans une fenêtre visible n’a pas abouti : la
demande d’ouverture de Computer Use a expiré. Les aperçus et tests natifs
ci-dessus ont bien été exécutés.
