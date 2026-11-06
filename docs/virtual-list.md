# Listes virtualisées

`Element::virtual_list` représente une collection verticale à hauteur de ligne
fixe. Les données restent dans l’application ; le moteur ne construit que les
éléments nécessaires au viewport, avec une petite marge configurable.
Le code est commun aux plateformes, sans crate externe.

## Utilisation

```rust
use karbon_ui::ui::{
    element::{Element, Length},
    virtual_list::VirtualList,
    widgets,
};

let tracks = Element::virtual_list(
    83,
    VirtualList::new(10_000, 36., |index| {
        widgets::button(
            1000 + index as u64,
            format!("Piste {}", index + 1),
            Msg::OpenTrack(index),
        )
    }).overscan(2),
)
.height(Length::Px(260.))
.gap(2.);
```

La fonction passée à `new` est une **fabrique de lignes**. Elle reçoit un index
compté à partir de zéro et renvoie un `Element`. Les lignes construites plus
tard, au défilement, portent elles-mêmes leurs messages (`Msg::OpenTrack(index)`
ici) : un clic envoie celui de la ligne courante à `Application::update`. Cet élément peut être un widget
personnalisé ou une composition de textes, images et boutons. Les tests de
`src/ui/tests/layout/virtual_list.rs` couvrent ces lignes et leur cycle de vie.

La fabrique doit être déterministe pour la vue courante, sans effet de bord.
Elle peut être appelée à nouveau lorsque l’utilisateur revient sur une ligne.
Pour des données réelles, capturer un `Arc` vers les données métier avec une
closure `move`, puis lire uniquement l’entrée demandée. Ne pas créer un
`Vec<Element>` de toute la collection avant de construire la liste.

Les IDs des éléments doivent rester stables et uniques dans toute l’interface,
y compris dans les enfants de chaque ligne. Le moteur vérifie les éléments
matérialisés ; il ne construit pas les lignes hors champ pour vérifier leurs IDs.
Il possède le tableau `children` de cette liste : ne pas le remplir à la main.

## Disposition et mémoire

Chaque ligne reçoit la largeur du viewport et la hauteur indiquée dans `new`.
La largeur et la hauteur déclarées sur sa racine ne remplacent pas cette case ;
padding, contenu et dispositions internes restent utilisables. Un contenu trop
grand est découpé. `gap` sépare les lignes et participe à l’étendue totale.

Le calcul utilise directement :

```text
pas de ligne = hauteur de ligne + gap
premier index visible = floor(offset / pas de ligne)
étendue = nombre × hauteur + (nombre − 1) × gap
```

Le moteur tient aussi compte du clip d’un éventuel panneau parent : une liste
entièrement cachée ne construit aucune ligne, sauf une éventuelle ligne active
conservée pour le clavier. Le nombre de lignes dépend donc de l’espace visible,
pas des 10 000 données métier.

À chaque changement d’intervalle, les lignes communes sont réutilisées. Celles
qui sortent sont libérées avant la création des nouvelles. Seules les nouvelles
lignes sont mesurées et disposées ; les lignes retenues ont déjà été translatées
par le scroll. Si l’intervalle ne change pas, aucune fabrique ni mesure de texte
n’est appelée. Un resize remet en page les lignes matérialisées.

La ligne active peut rester construite hors champ lorsque la liste possède le
focus. Il y en a au plus une supplémentaire. Une reconstruction applicative
peut détenir temporairement l’ancien et le nouvel arbre, tous deux bornés par
le viewport. Les données métier, les images détenues par l’application et les
caches natifs restent à comptabiliser séparément.

L’état propre à une liste est alloué séparément avec `Box`, pour éviter d’en
augmenter le coût dans chaque élément `Text` ou `Button` ordinaire.

## Souris et clavier

La molette et la barre utilisent le scroll existant. Une liste peut se trouver
dans un panneau défilant ; le mouvement restant en bout de liste remonte au parent.

La liste constitue **un seul arrêt Tab**, pour pouvoir la traverser sans passer
par 10 000 contrôles. Le focus reste sur la liste, avec un index actif logique.

| Action | Résultat |
| --- | --- |
| Tab / Shift+Tab | Entrer dans la liste ou en sortir. |
| Haut / Bas | Changer de ligne active, y compris au-delà des lignes déjà construites. |
| Home / End | Activer la première / dernière position de navigation. |
| Page Up / Page Down | Déplacer la position active d’environ un viewport. |
| Entrée | Déclencher une fois l’action principale de la ligne active. |
| Espace | Montrer l’appui, puis déclencher au relâchement. |
| Clic sur un contrôle d’une ligne | Sélectionner cette ligne et déclencher le contrôle cliqué. |

Home et End changent la ligne active sans déclencher sa commande. La ligne
active est rendue visible lors d’une navigation au clavier. Entrée/Espace
utilisent la première action activable de son arbre, dans l’ordre de composition.
Une ligne purement visuelle ne déclenche aucune commande. Les contrôles
supplémentaires restent cliquables ; la navigation Tab détaillée entre plusieurs
contrôles d’une même ligne n’est pas encore proposée.

Changer de ligne pendant un appui sur Espace annule cet appui. L’action du
nouvel élément ne part donc pas accidentellement au relâchement. Les répétitions
automatiques d’Entrée et d’Espace ne dupliquent pas l’activation.

L’offset et l’index actif sont conservés sous l’ID de la liste pendant une
reconstruction, puis bornés si les données diminuent. La sélection interne est
actuellement un index : après un tri ou un filtrage, le suivi d’un titre métier
par sa clé reste à gérer côté application. Changer l’ID de la liste correspond
à une nouvelle collection et réinitialise son état.

## Observer et vérifier

`element.virtual_metrics()` donne le nombre logique d’éléments, l’intervalle
matérialisé avec sa marge, le nombre de lignes détenues, l’index actif et les
appels à la fabrique depuis la création de cette vue. Les compteurs sont remis
à zéro lors d’une reconstruction. `RuntimeStats::virtual_rows`, affiché avec F8,
donne le nombre courant de lignes matérialisées dans toute l’interface.

```sh
cargo test --offline virtual_list_tests
cargo test --offline --release measure_virtual_list -- --ignored --nocapture
```

Les tests vérifient 10 000 données, la réutilisation, les petits deltas, les sauts
jusqu’au bout, les clips imbriqués, le clavier, le clic, les reconstructions,
la réduction et la suppression des données, les tailles nulles, les IDs invalides
et la libération des widgets. Avec des lignes de 32 unités dans un viewport de
400 unités, ils vérifient 15 lignes au démarrage, au plus 18 pendant des sauts
répétés, et au plus une ligne supplémentaire lorsqu’il faut conserver le focus.

Mesure illustrative du 2 novembre 2026, build release, 500 scrolls de 48 unités,
viewport de 600 × 400, widget de texte instrumenté et faux mesureur de texte :

| Cas | Lignes initiales / pic | Initialisation | Scroll médian | Scroll p95 |
| --- | --- | --- | --- | --- |
| Scroll ordinaire, 1 000 éléments | 1 000 / 1 000 | 394 µs | 19,5 µs | 20,2 µs |
| Liste virtuelle, 1 000 éléments | 15 / 17 | 18 µs | 4,1 µs | 6,1 µs |
| Liste virtuelle, 10 000 éléments | 15 / 17 | 10 µs | 5,5 µs | 7,9 µs |

Ces chiffres concernent uniquement le cœur UI et les widgets de test. Ils
varient avec le bruit de mesure et l’échauffement ; ils excluent DirectWrite,
le dessin GPU/CPU et la présentation de l’image. Ce n’est pas une mesure de RAM
globale ou de FPS, ni une comparaison avec Tauri. La borne sur les objets détenus
est vérifiée indépendamment des chronométrages. La démo réelle a été essayée
sur GPU, sur WARP et avec un fond transparent.

## Limites de cette première version

- Collection verticale et hauteur fixe ; les hauteurs variables et la grille
  virtualisée restent à ajouter.
- Pas de liste virtualisée à l’intérieur d’une ligne virtualisée pour l’instant.
- Pas de champ éditable dans une ligne virtualisée pour l’instant : l’édition
  et la composition doivent être conservées avant de permettre ce recyclage.
- Hauteur minimale d’une unité, overscan de 0 à 64, au plus 2 048 lignes
  matérialisées et étendue totale limitée à 16 777 216 unités pour borner la
  géométrie et les allocations. Les configurations invalides renvoient une erreur.
- Le [fournisseur Windows UI Automation](accessibility.md) expose la liste et les
  lignes matérialisées ; les patterns Selection, Scroll et VirtualizedItem et
  l’accès aux éléments virtuels hors champ restent à créer.
- Les états métier des lignes doivent vivre dans l’application : une ligne sortie
  du viewport peut être détruite. Le recyclage ne conserve pas un widget pour
  chacun des 10 000 indices.

La copie CPU de la texture pour présenter la fenêtre reste inchangée. Les gains
du cœur UI ne suppriment pas ce coût de rendu.
