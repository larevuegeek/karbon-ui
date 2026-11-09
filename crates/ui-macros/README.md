# karbon-ui-macros

Macros procédurales de `karbon-ui` (`ui!`, `ui_file!`, `styles!`, `css_file!` et la macro
cachée `__karbon_classes!`). Tout est compilé pendant la construction, avec `proc_macro`
et `std` seulement. Les macros publiques sont les enveloppes de `src/lib.rs` de
`karbon-ui` : elles passent en tête le chemin de la crate, que le code généré utilise
sous le nom `__ui_root`.

## Carte

| Dossier / fichier | Rôle |
| --- | --- |
| `lib.rs` | Points d'entrée des macros, rien d'autre |
| `error.rs` | `Error` (message et span) devenu `compile_error!` |
| `input/` | Arguments des macros (`Cursor`) et fichiers lus relativement à l'appelant (`Source`, `Reader`) |
| `codegen/` | Assemblage du code produit (`Output`) et traduction commune des propriétés de style (`StyleProp`, `property`) |
| `styles.rs` | `styles!` : recettes de styles nommées |
| `template/` | `ui!` et `ui_file!` : directives, puis `markup/` (balisage → nœuds), `html.rs` (tables HTML) et `generate/` (nœuds → constructeurs, un fichier par sorte d'élément) |
| `bootstrap/` | Comportements `data-bs-*` réécrits dans l'arbre du template avant la génération |
| `svg/` | `<svg>` en ligne compilé en `SvgImage` statique (chemins, formes, transformations) |
| `css/` | `css_file!` : lecteur strict (`parser/`), sélecteurs, requêtes, animations (`animation/` : étapes, listes `animation`), valeurs par famille (`values/`), classe compilée (`class/`), feuille stricte (`sheet/`), point d'entrée (`module.rs`), `style="…"` (`inline.rs`) |
| `css/framework/` | Feuilles existantes (Bootstrap, sortie Tailwind) : lecteur tolérant, parcours des at-rules, sélecteurs complets, cascade par élément, valeurs évaluées à l'exécution |
| `classes.rs` | `__karbon_classes!` : listes de classes d'un template résolues contre la feuille, puis contre Tailwind |
| `tailwind/` | Utilitaires Tailwind v4 : découpage (`candidate.rs`), thème, variantes, utilitaires par famille (`utilities/`), tri et résolution par élément (`set.rs`, `order.rs`, `resolve.rs`) |

## Chaînes de compilation

**Template → Rust.** `template::compile_inline` / `compile_file` lisent les directives
(`@context`, `@scope`, `@slots`, `@styles`), puis `markup` découpe le balisage en pièces
situées et construit les nœuds (`Element`, `{#if}`, `{#each}`, `{@include}`).
`bootstrap::rewrite` réécrit les comportements `data-bs-*`, puis `generate` produit le
texte Rust de chaque nœud (constructeur, classes, styles, attributs, événements).
La liste des enfants d'un conteneur est réservée d'avance (`Generator::capacity` : borne
haute par branche, et pour une boucle `{#each}` la taille de l'itérable) : un élément
fait ~500 octets, une liste qui grandissait par doublements recopiait chaque enfant.
`codegen::Output::finish` relit ce texte en jetons, réinsère les expressions de
l'appelant avec leurs spans et remplace `__ui_root` par le chemin de la crate. Les
classes littérales passent par la macro `__karbon_classes!` du module de styles, que
`classes.rs` résout.

**CSS → Rust.** `css::module::compile` lit les arguments de `css_file!` et le fichier
(avec ses `@import`). En mode strict, `css::parser` lit la feuille, chaque règle est
versée dans sa classe (`class::rules::add_rule`, valeurs traduites par `values/`) et
chaque classe devient une fonction de style (`class::render`). En mode `framework`,
`css/framework` aplatit la feuille, compile les sélecteurs et garde la cascade, évaluée
élément par élément à la compilation des templates (`framework::element`). Le module
généré contient aussi `__karbon_classes!`, qui décrit la feuille (classes, variables,
directives Tailwind) pour `classes.rs`.

## Tests et outils

- Tests unitaires, à côté du code ou dans un dossier `tests/` par thème : `css/tests/`,
  `css/framework/tests.rs`, `tailwind/tests/` (dont l'oracle, comparé à
  `tailwind/tests/oracle/oracle.css`), tests en fin de fichier (`svg/path.rs`…) :
  `cargo test -p karbon-ui-macros --lib`.
- Outils de mesure, tests ignorés qui régénèrent les chiffres de la documentation :
  `tailwind/tools/` (couverture RustMusic, relevé complet de l'oracle,
  [docs/tailwind.md](../../docs/tailwind.md)) et `css/framework/tools.rs` (scores de
  compatibilité, [docs/css-frameworks.md](../../docs/css-frameworks.md)).
