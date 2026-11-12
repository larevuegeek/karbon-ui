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

Comme en CSS, la zone visible est la **boîte de padding** : la barre se loge entre
la bordure et le padding, et le padding défile avec le contenu (`pb-16` laisse une
marge après la dernière ligne au lieu de rogner le contenu 64 unités avant le bord).

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
Le scroll incrémente `RuntimeStats::scrolls`, visible avec F8 (option `profile`), sans augmenter
`views` ni `layouts`. Un changement de taille demande encore un layout.

## Performances et mémoire : limites actuelles

Les éléments hors champ ne produisent aucune commande de dessin, y compris les lignes
en `overflow: visible` (le cas des gabarits) dont la découpe est celle de toute la
liste : un sous-arbre dont l'encre ne coupe pas sa découpe n'est pas peint
(`runtime/painting/cull.rs`). Ce mécanisme est du découpage avec élimination du dessin
invisible, pas de la virtualisation.
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

## Contenu sauté (`content-visibility`)

`content-visibility: auto` (module `layout/containment.rs`) reprend le comportement de
Chromium pour les longues listes : le contenu d'un élément éloigné de la vue n'est ni
mesuré, ni placé, ni peint, ni touché, ni animé. L'élément garde sa boîte : taille
déclarée, sinon `contain-intrinsic-size`, ou avec `auto` la dernière taille de son
contenu. La proximité se constate après chaque layout et chaque défilement, sur la vue
de chaque conteneur défilant élargie de la moitié de sa taille (et la fenêtre, de
même) ; un élément qui contient le focus reste disposé.

- Un élément qui **s'approche** est disposé sur place quand sa taille ne dépend pas de
  son contenu (mesuré avec lui, il garde la taille de son dernier layout) : son seul
  contenu est placé dans sa boîte, sans layout de l'arbre ni résolution des styles.
  Sinon, ses ancêtres oublient leur placement retenu et un layout refait leur chemin.
- Un élément qui **s'éloigne** cesse simplement d'être peint et touché ; son contenu
  garde son dernier placement, repris tel quel s'il revient.
- L'élément rogne toujours son contenu à sa boîte de padding arrondie (confinement de
  la peinture), même proche de la vue.

Sur la liste des morceaux de RustMusic (`row-track` de l'original), seules une
trentaine de lignes sur 200 sont disposées ; la mise en page d'un chargement de page
passe de 20 à 36 ms à 6 à 15 ms.

## Événement, ancrage et cible initiale

- **`onscroll`** (`Element::on_scroll`) : chaque déplacement réel du conteneur
  (molette, barre, clavier, révélation) envoie un message construit depuis la
  nouvelle géométrie [`ScrollMetrics`] (`offset`, `max_offset()`,
  `viewport_size()`), après le déplacement ; une butée n'émet rien. Usage type,
  la page suivante d'une longue liste :
  `onscroll={|s: ScrollMetrics| Msg::Scrolled(s.offset, s.max_offset())}`.
- **Ancrage** (`overflow-anchor: auto` des navigateurs) : avant une reconstruction
  ou une remise en page, chaque conteneur vertical défilé retient le premier élément
  visible (le plus profond qui chevauche le haut de la vue) ; après, si cet élément
  existe encore (même identifiant), le décalage est corrigé de son déplacement. Une
  liste chargée par pages peut donc ajouter ou retirer des lignes au-dessus de la vue
  sans que la ligne regardée ne saute. En haut (décalage nul), pas d'ancrage : du
  contenu ajouté au-dessus se montre, comme dans Chromium.
- **`scroll-initial-target: nearest`** (CSS Scroll Snap 2, `style:scroll_initial_target={bool}`
  dans un template) : un conteneur qui apparaît (nouvel identifiant) part de cet
  élément (sa `scroll-margin` et le `scroll-padding` du conteneur comptent) ; les
  reconstructions suivantes gardent le décalage de l'utilisateur. Pour revenir en
  haut ou sur une cible après un nouveau chargement, changer la clé du conteneur
  (`{#each [génération] as g (g)}`).
- **Molette à crans** (`UiEvent::Wheel`) : comme les navigateurs, le conteneur glisse
  vers sa cible en 150 ms (sortie cubique) au lieu de sauter ; un cran pendant le
  glissement allonge la cible en gardant la vitesse acquise (courbe raccordée, comme
  `UpdateTarget` de Chromium) : une rotation continue donne un mouvement continu, sans
  élan relancé à chaque cran. Le raccord se fait à l'instant du cran, depuis la position
  de la courbe en cours et non depuis le dernier décalage affiché : un cran reçu entre
  deux images ne fait pas perdre le chemin parcouru depuis la précédente (avant le
  2026-11-11, l'image suivante ne bougeait presque pas, un rafraîchissement manqué par
  cran). Chaque image échantillonne la courbe à son instant (une
  image un peu en avance sur le réveil prévu avance quand même), et un ancrage du
  défilement pendant le glissement (lignes retirées au-dessus par une liste chargée par
  pages) déplace aussi sa cible. Le défilement précis d'un pavé tactile
  (`UiEvent::Scroll`) et les mouvements réduits restent immédiats. Sous Windows, un cran
  fait 100 unités (trois lignes de 100/3, comme Chromium) ; les backends envoient
  `Wheel` pour un cran entier (`WM_MOUSEWHEEL` multiple de 120, `value120` de Wayland,
  boutons 4 à 7 de X11, deltas non précis d'AppKit).
- **Survol pendant un défilement** : sous un pointeur immobile, le survol suit le
  contenu au plus toutes les 100 ms (comme les faux mouvements de souris de
  Chromium), et aussitôt que le pointeur bouge : les lignes qui passent sous le
  pointeur ne s'allument pas l'une après l'autre, et la molette ne relance pas la
  cascade des styles à chaque cran.
