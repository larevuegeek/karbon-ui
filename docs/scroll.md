# Défilement

Le scroll appartient au cœur UI commun. Le backend Windows transforme la
molette en événements ; il ne connaît pas le contenu de la liste.
Aucune crate externe n’a été ajoutée.

## Composer un viewport

```rust
use karbon_ui::ui::{element::{Axis, Element, Length}, widgets};

let content = Element::column(2, vec![
    widgets::text(3, "Première ligne"),
    widgets::text(4, "Deuxième ligne"),
]).gap(8.);

let panel = Element::scroll(1, Axis::Vertical, content)
    .height(Length::Px(200.))
    .padding(12.);
```

Le viewport possède exactement un enfant. Une colonne de hauteur `Auto`
conserve sa hauteur naturelle et peut dépasser les 200 unités disponibles.
`Fill` sur le contenu signifie remplir le viewport : utiliser `Auto` ou une
dimension explicite sur l’axe que l’on souhaite faire défiler.

`Axis::Horizontal` utilise le même contrat avec la largeur. Pour une rangée
de largeur naturelle, lui donner `.width(Length::Auto)` et des largeurs `Auto`
ou fixes à ses enfants, plutôt que `Fill`. Chaque viewport gère
un axe ; deux viewports d’axes différents peuvent être imbriqués.

Une gouttière de 12 unités est réservée pour éviter un changement de largeur
et de retour à la ligne lorsque la barre apparaît. La barre n’est peinte que
si le contenu dépasse le viewport. Sa poignée mesure au minimum 24 unités,
dans la limite de la piste. Couleurs et survol suivent le thème. Le style se
règle par `Element::layout_rules().scrollbar` ([`Scrollbar`] : `ScrollbarWidth::Auto`,
`Thin` ou `None`, couleurs du pouce et de la piste, gouttière stable) ou en CSS
(`scrollbar-width`, `scrollbar-color`, `scrollbar-gutter`, voir [css.md](css.md)) :
une barre fine ne réserve sa gouttière de 10 unités que si le contenu déborde,
`none` n’affiche rien et ne réserve rien. `ScrollMetrics::thickness` donne
l’épaisseur retenue.

`Element::scroll_metrics()` expose le viewport, l’étendue du contenu, l’offset,
la piste et la poignée après disposition. Les distances sont en unités
logiques, converties en pixels par le backend.

## Interactions

| Action | Comportement |
| --- | --- |
| Molette | Déplacer le viewport sous le pointeur, sans devoir lui donner le focus. |
| Shift+molette ou molette horizontale | Défilement horizontal. |
| Molette dans des viewports imbriqués | L’enfant consomme ce qu’il peut ; le reliquat va au parent de même axe. |
| Glisser la poignée | Déplacer le contenu avec capture de la souris. |
| Cliquer sur la piste | Avancer ou reculer de 90 % du viewport. |
| Échap pendant un glissement | Restaurer l’offset de départ et libérer la capture. |
| Tab / Shift+Tab | Parcourir les contrôles et les zones de scroll. Un contrôle hors champ est rendu visible. |
| Flèches sur une zone de scroll | Déplacer de 40 unités sur son axe. |
| Page Up / Page Down | Déplacer de 90 % du viewport. |
| Home / End | Atteindre le début / la fin. |

Les touches de scroll peuvent aussi remonter depuis un bouton focalisé.
Entrée et Espace conservent leur fonction d’activation ; les touches d’un
séparateur redimensionnable gardent leur priorité.

Windows donne les coordonnées écran de la molette, avec des valeurs signées.
Le backend les convertit en coordonnées client puis en unités logiques.
Il conserve les fractions de cran et lit le réglage système de lignes,
caractères ou pages. Références natives :
[WM_MOUSEWHEEL](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousewheel),
[WM_MOUSEHWHEEL](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousehwheel) et
[SystemParametersInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow).
Les gestes tactiles et l’inertie ne sont pas encore implémentés.

## Ce qui se passe dans le moteur

1. `layout::arrange` mesure le contenu, calcule le viewport, borne l’offset et
   place l’enfant avec le décalage correspondant.
2. Un événement de scroll appelle `scroll::set_offset`. Il translate les
   rectangles déjà calculés, les séparateurs et les éventuels viewports enfants.
3. Les clips sont recalculés depuis les rectangles : les éléments qui entrent
   dans le champ deviennent visibles et ceux qui en sortent cessent d’être
   dessinés et cliquables.
4. La scène est repeinte. Les mesures de texte et le layout sont réutilisés.

L’offset reste dans le runtime sous l’identifiant stable du viewport. Il est
restauré après une reconstruction, puis borné aux nouvelles limites si la
fenêtre ou le contenu change. Supprimer le viewport retire son état conservé.
Le scroll incrémente `RuntimeStats::scrolls`, visible avec F8, sans augmenter
`views` ni `layouts`. Un changement de taille demande encore un layout.

## Performances et mémoire : limites actuelles

Les éléments hors champ ne produisent aucune commande de dessin. Ce mécanisme
est du découpage avec élimination du dessin invisible, pas de la virtualisation.
Dans un scroll ordinaire, tous les widgets restent construits et détenus par l’arbre. Leur
translation et le parcours des enfants coûtent encore un travail proportionnel
au nombre d’éléments. Les changements de taille remesurent le contenu.

Le test de 1 000 lignes vérifie au plus 14 widgets peints dans son viewport,
moins de 100 commandes de scène, et aucune nouvelle mesure, reconstruction ou
disposition lors de 100 scrolls. Il vérifie aussi la libération des anciens
widgets après reconstruction, réduction de la liste et destruction du runtime.
Ce test ne mesure pas la RAM globale du processus ni les performances d’une
application réelle. Une reconstruction peut détenir temporairement les deux arbres.

La présentation Windows conserve la relecture CPU de la texture GPU, le DIB
et une présentation de l’image entière. Le scroll ne supprime pas ce coût.
La [liste virtualisée](virtual-list.md) construit seulement les lignes utiles
et réutilise ce même scroll. Les tests couvrent une collection de 10 000 entrées. Une présentation GPU sans la copie CPU reste un travail distinct
avant toute promesse de gain global face à une UI web.
