# Styles CSS et Rust

Les fichiers `.css`, les recettes `styles!` et les attributs `.ui` construisent
le même arbre natif. Le compilateur traduit le CSS en fonctions Rust : aucun
parseur CSS, DOM, JavaScript ou navigateur ne tourne pendant l’affichage.
Les cinq familles prévues sont maintenant raccordées au moteur. La couverture
ci-dessous reste un sous-ensemble explicite du CSS d’un navigateur.

## Essayer et modifier

Deux exemples complets utilisent le CSS compilé :

- [`rustmusic_html`](rustmusic-html.md) : une page réelle traduite de Svelte +
  Tailwind, avec ses propres feuilles (`examples/rustmusic_html/styles/*.css` :
  jetons, mise en page, menus, `@container`, barres fines, SVG en ligne) ;
- `bootstrap_demo` : Bootstrap 5.3 compilé en mode framework
  ([feuilles de frameworks](css-frameworks.md)).

```powershell
cargo run --offline --release --example rustmusic_html
cargo run --offline --release --example bootstrap_demo
```

Chaque fonctionnalité a son scénario minimal dans `tests/css/` (une feuille par
module, à côté de lui : `selectors.css`, `values.css`, `layout/flex.css`…) : c’est la
référence exacte de ce que le moteur accepte, voir [l’organisation des tests](tests.md).
Les [templates](templates.md) documentent les includes, composants, slots, conditions
et boucles.

## Associer des styles à une vue

```rust,ignore
use karbon_ui::ui::prelude::*;
css_file!(mod css, "views/styles.css");

fn view(&self, _: &ViewContext) -> Element {
    let cx = TemplateContext::default().with_variables(&self.variables);
    ui_file!("views/home.ui", context=cx, styles=css)
}
```

Dans `home.ui` :

```html
<article class="card">
    <h2 class="heading">Bonjour David</h2>
    <button class="button primary" onclick={Msg::Save}>Enregistrer</button>
</article>
```

Dans `styles.css` :

```css
:root { --accent: #47e5b1; }
.card {
    padding: 20px; gap: 16px;
    font-family: "Segoe UI"; font-size: 16px; color: #f4f7ff;
    background: #151f2e; border: 1px solid #2b394f; border-radius: 18px;
}
.heading { font-size: clamp(22px, 3vw, 32px); font-weight: 700; }
.button { height: 44px; padding: 10px 18px; border-radius: 12px; }
.primary { background: var(--accent); color: #071b17; }
.button.primary:hover { background: #b6ffe7; }
.button:focus-visible { border-color: #fff; }
```

Avec `ui!`, la directive est `@styles(css);`. Les includes héritent du module
et du contexte. Un composant qui appelle lui-même `ui!` ou `ui_file!` déclare
son module de styles dans cet appel.

Une classe `.card` exporte `css::CARD` ; `.card-body` exporte `css::CARD_BODY`.
Les collisions d’identifiants Rust sont refusées. Les classes littérales
restent sensibles à la casse : `class="…"` est résolu à la compilation par la macro
`__karbon_classes!` du module (classe inconnue : erreur avec suggestion). Une feuille
qui commence par `@import "tailwindcss";` accepte aussi les utilitaires Tailwind,
`@theme` et `@custom-variant` : voir [Tailwind CSS](tailwind.md).

Une feuille existante d’un framework (Bootstrap, sortie compilée de Tailwind) se
charge en **mode framework** : `css_file!(mod bs, "bootstrap.css", framework)`. Ce
mode n’échoue pas sur la syntaxe non prise en charge (elle est sautée et comptée),
accepte sélecteurs de type, d’attribut et d’id, `!important`, `@layer`, `@supports`,
et résout `var()` dans toutes les propriétés courantes à l’exécution : voir
[Feuilles de frameworks CSS](css-frameworks.md). Les feuilles de l’application
restent strictes.

## Mélanger CSS et Rust

```rust,ignore
styles! { pub CUSTOM { gap: 24; margin_top: 8; } }
let page = ui! {
    <section class={[css::CARD, CUSTOM]} style="padding: 28px">
        <h2 class={css::HEADING}>Une seule interface</h2>
    </section>
};
```

La [feuille par défaut](templates.md#feuille-de-style-par-défaut) passe en
premier, puis les recettes dans l’ordre indiqué, puis `style="…"` et les
attributs `style:propriété={expr}` du template. `class="card compact"` applique deux recettes dans cet ordre.
Les règles conditionnelles sont ensuite résolues selon leur spécificité
et leur ordre de déclaration. Ce modèle n’est pas une feuille de style globale
avec toute la cascade des navigateurs : les classes restent des appels explicites.

Les noms usuels `background_color`, `border_radius`, `font_weight`,
`font_family`, `align_items`, `justify_content`, `margin_top`, `padding_left`
sont utilisables dans `styles!`. Les anciens noms `background`, `radius`,
`weight`, `align`, `justify` restent disponibles. Les widgets spécifiques
peuvent adapter leur dessin avec `Widget::apply_style(&ComputedStyle)`.

## 1. Apparence des contrôles et typographie

La taille, la famille, la graisse, la couleur, l’alignement, la hauteur de ligne
et les décorations se propagent des conteneurs vers leurs descendants.
Text, Button, TextInput, Checkbox, Switch, RadioButton, Slider et Select
utilisent ce contrat commun. Les métriques du champ éditable, sa sélection,
son caret et ses coordonnées accessibles utilisent la même fonte que le dessin.

| Propriétés | Couverture |
| --- | --- |
| `font-family` | Famille ou liste de repli (`"Inter", system-ui, sans-serif`) : la première disponible, chargée par l’application puis système ; génériques `system-ui`/`sans-serif` → Segoe UI, `serif` → Times New Roman, `monospace` → Consolas |
| `font-size` | px, em, rem, vw, vh et expressions |
| `font-weight` | 1 à 999, normal, bold |
| `color`, `accent-color` | Couleur du texte et accent des contrôles de choix/slider |
| `text-align` | left/start, right/end, center, justify selon le contrôle |
| `line-height` | normal, facteur sans unité, px |
| `text-decoration` / `text-decoration-line` | none, underline, overline, line-through, combinaisons |
| `text-decoration-color`, `text-underline-offset`, `text-decoration-thickness`, `text-decoration-style: solid` | couleur, décalage sous la ligne de base et épaisseur des lignes en px (`auto`/`from-font` : celles de la police) ; le moteur dessine alors lui-même le soulignement |
| `::placeholder { color }` | couleur du texte indicatif d’un champ (sinon la couleur atténuée du thème) |
| `font-style` | normal, italic, oblique (rendu italique) |
| `letter-spacing` | normal, px (négatif accepté), em (calculé sur la police de l’élément déclarant puis hérité) |
| `text-transform` | none, uppercase, lowercase, capitalize ; appliqué avant la mesure, portion par portion dans un paragraphe enrichi |
| `white-space` | normal, nowrap, pre, pre-wrap (`text-wrap: nowrap/wrap` accepté) |
| `text-overflow` | clip, ellipsis : ligne coupée terminée par `…` |
| `direction` | ltr, rtl : avec `text-overflow: ellipsis` sur une ligne (`nowrap`), rtl coupe le **début** (`…e HD\FLAC\…`, chemin de la barre d’état) ; ni réordonnancement bidirectionnel ni alignement changés (`text-align` reste celui déclaré) |
| `line-clamp` / `-webkit-line-clamp` | none ou 1 à 1000 lignes ; la dernière ligne gardée se termine par `…` |
| `font-variant-numeric` | normal, tabular-nums, proportional-nums |
| Fond, bordure, padding | Partagés avec Button, TextInput et Select, sans double fond/padding interne |

Les états peuvent changer les couleurs, la typographie et les propriétés
partagées. Les styles Rust détaillés des widgets restent disponibles pour les
sous-parties : piste, pouce, caret, sélection, etc. Il n’existe pas encore de
pseudo-éléments CSS pour styler individuellement ces sous-parties (seuls
`::before` et `::after` sont générés, voir la section 5). Le champ multiligne
(`<textarea>`, `TextInput::multiline`) reprend les styles du champ monoligne (fond,
bordure, rayon, padding, police, couleur, `::placeholder`, `text-align`) ; son texte
se coupe à la largeur intérieure, chaque ligne est mise en page et dessinée seule,
et son contenu défile verticalement (indicateur fin dans la marge droite, sans
`scrollbar-*`). Les mots-clés CSS `inherit/initial/unset` ne sont pas fournis.

Ces propriétés de texte s’héritent toutes, y compris `text-overflow` et
`line-clamp` (non héritées en CSS) : la classe Tailwind `truncate` ou
`line-clamp-2` posée sur un conteneur s’applique donc aussi au texte qu’il
contient. `display: -webkit-box` et `-webkit-box-orient: vertical`, qui
accompagnent `-webkit-line-clamp`, sont acceptés sans effet. Sans retour à la
ligne, la mesure renvoie la largeur naturelle, bornée à la boîte avec `ellipsis`.
Les espaces du texte littéral d’un template sont fusionnés à la compilation :
`pre`/`pre-wrap` conservent ceux des valeurs dynamiques (`{expr}`) ; utiliser
`<pre>` pour un texte littéral. Le champ de saisie ignore la casse, la coupure,
l’ellipse et la limite de lignes. DirectWrite applique l’espacement
(`SetCharacterSpacing`), l’italique, la fonction OpenType `tnum` et le rognage
avec ellipse ; le backend headless reproduit espacement, coupure, ellipse et limite
de lignes avec ses métriques fixes.

## 2. Boîtes, layout et positionnement

| Fonction | Couverture |
| --- | --- |
| Boîte | `box-sizing: border-box/content-box` ; padding et bordure réservés au contenu ; min/max en px, en % de la boîte de contenu du parent ou en expression (`calc(100vh - 330px)`, vw/vh/em) ; `aspect-ratio` (`16 / 9`, nombre, auto) |
| Dimensions | width/height en px, %, auto, em/rem/vw/vh et expressions |
| Espacements | padding/margin en 1 à 4 valeurs, côtés séparés ; gap, row-gap, column-gap ; px, em, rem, vw, vh et expressions (`calc()`, `min()`, `max()`, `clamp()`, `var()`) ; marges négatives et auto ; pas de % (refusé) |
| Affichage | `display` : block, flex, inline-flex (conteneur flex dans le flux), grid, inline-grid, none ; `inline` et `inline-block` (une boîte du moteur, traitée en bloc ; `inline-block` garde sa largeur naturelle dans un bloc) ; `-webkit-box` accepté avec line-clamp |
| Flex | display:flex, row/column et reverse, nowrap/wrap/wrap-reverse, basis/grow/shrink, align-self, `order` (entier) |
| Alignement | start/end/center/stretch ; justify-content start/end/center/space-between |
| Grille | Colonnes et lignes (`grid-template-columns` / `grid-template-rows`) en px, %, auto, fr, `max-content`, `fit-content(px)`, minmax et repeat (dont `repeat(auto-fill | auto-fit, piste)`, répétée selon la largeur) ; grid-column/span, `N / M`, `1 / -1`, lignes et colonnes explicites ; `justify-self` et `align-self` start/center/end/stretch dans la cellule (largeur auto alignée : largeur du contenu), `align-items` et `justify-items` du conteneur comme alignements par défaut ; `justify-content` start/center/end/space-between place les colonnes quand leurs pistes laissent de la place (sans effet avec une piste `fr`) ; lignes implicites d’une grille de hauteur définie étirées à parts égales |
| Position | static/relative/absolute/fixed ; top/right/bottom/left et raccourci `inset` (1 à 4 valeurs) en longueurs ou auto ; z-index entier |
| Débordement | overflow et axes visible/hidden/clip ; contours arrondis pour un clip explicite sur les deux axes ; `overscroll-behavior: contain / none` (le reste d’un défilement à la molette reste dans le conteneur) ; `scroll-margin` et `scroll-padding` (marges gardées quand le focus amène un élément dans la vue) |
| Pointeur | `cursor` (auto, default, pointer, text, not-allowed, wait, progress, help, move, grab, grabbing, crosshair, col/row/ew/ns/nwse/nesw-resize…), hérité et possible dans les états ; `pointer-events: none / auto`, hérité (ni clic, ni survol, ni curseur) |

Les bordures sont incluses dans le calcul du contenu. `content-box` ajoute
padding et bordure à la dimension déclarée. Les marges restent extérieures,
sans fusion entre voisins. Les marges auto absorbent l’espace libre positif.
Le flex distribue l’espace avec gel des éléments arrivés à leurs min/max ;
le shrink est pondéré par la taille de base. Les grilles répartissent les
fractions après les pistes fixes et les gaps.
`flex-basis: auto` (dont `flex: 0 0 auto`) reprend la dimension principale déclarée
(`width` d’une rangée), comme en CSS. Un `display` visible écrit dans `style="…"`
annule le `display: none` des classes.
Les facteurs flex dont la somme est inférieure à 1 ne consomment qu’une partie
de l’espace libre, y compris après gel aux min/max ; voir
[la résolution des longueurs flexibles](https://www.w3.org/TR/css-flexbox-1/#resolve-flexible-lengths).

`relative` conserve sa place dans le flux. `absolute` utilise la boîte de
padding du plus proche ancêtre positionné, ou le viewport. Deux offsets
opposés étirent une dimension auto. `fixed` reste attaché au viewport et
échappe au scroll et au clip de ses ancêtres. Chaque conteneur définit ici
un contexte d’empilement ; dessin et clics emploient le même ordre de z-index
(les feuilles de framework font remonter le `z-index` d’un descendant à travers
ses ancêtres sans `z-index`, comme en CSS : voir [css-frameworks.md](css-frameworks.md)).
Les calques modaux restent au-dessus de leur page.

`display: none` retire l’élément du flux (comme un enfant positionné) : boîte et
clip vides, donc ni dessin ni clic, et son sous-arbre quitte l’ordre de
tabulation et l’arbre d’accessibilité. `order` trie les enfants pour le layout
seulement (flex, grille, bloc) ; dessin, focus et accessibilité gardent l’ordre
du template. Les lignes explicites `grid-template-rows` fixent la hauteur des
premières lignes (px, %, auto, minmax) et partagent la hauteur restante de la
grille entre les `fr` ; les lignes suivantes restent à la hauteur du contenu.

Différences restantes : pas de fusion des marges, align-content, placement
automatique dense, sizing complet min-content /
max-content ou algorithme complet de position statique CSS. Un conteneur auto
conserve les conventions natives de ses enfants extensibles ; il ne reproduit
pas toujours le shrink-to-fit HTML. Le défaut natif de boîte est border-box ;
un conteneur de template sans `overflow` est `visible` comme en HTML (les éléments
construits en Rust gardent un clip rectangulaire). La largeur naturelle d’un bloc
(base d’un élément flex `width: auto`, conteneur `width: auto`) est celle du contenu
de ses enfants, même quand ceux-ci remplissent la largeur offerte : deux `div` dans
une rangée flex prennent leur largeur de contenu, pas des parts égales.

Dans un template, `overflow: auto` ou `overflow: scroll` rend un conteneur
défilant : la boîte (taille, marges, fond, bordure, padding) reste sur la zone
visible et le contenu (enfants, direction, grille, gap, alignements) défile
dans un seul axe, vertical si `overflow-y` défile ; le gap du contenu est celui de la
construction de la vue, en px (un gap en expression n’y est pas reporté). Hors template, ces valeurs
rognent comme `hidden`. Un enfant positionné doit être dans le contenu d’un Scroll/Split/Layers,
pas directement à la place d’un de leurs enfants structurels. Les descendants
fixed sous un ancêtre transformé/animé/filtré et les fixed dans une ligne
virtualisée sont refusés. Les clips par axe restent rectangulaires lorsqu’un
seul axe est visible.

Barres de défilement d’un conteneur défilant :

```css
.contenu { overflow-y: auto; scrollbar-width: thin; scrollbar-color: rgba(255,255,255,.22) transparent; }
.onglets { overflow-x: auto; scrollbar-width: none; }
.liste { overflow-y: auto; scrollbar-width: thin; scrollbar-gutter: stable; }
```

| Propriété | Effet |
| --- | --- |
| `scrollbar-width: auto` | Défaut du moteur : piste de 12 px toujours réservée, pouce de 6 px, couleurs du thème (accent au survol ou au focus) |
| `scrollbar-width: thin` | Gouttière de 10 px réservée **seulement si le contenu déborde** (barres classiques de Chromium), pouce de 4 px aux bouts arrondis, piste transparente ; le pouce s’opacifie (alpha × 1,55) au survol du conteneur, comme `.scrollbar-app:hover` de RustMusic |
| `scrollbar-width: none` | Ni barre ni place réservée ; le contenu défile toujours (molette, clavier) |
| `scrollbar-color: pouce piste` / `auto` | Couleurs constantes du pouce et de la piste (`transparent` : piste non peinte) |
| `scrollbar-gutter: stable` / `auto` | Réserve la gouttière d’une barre fine même sans débordement (`both-edges` refusé) |
| `.x::-webkit-scrollbar { width: 8px }` | Épaisseur de la gouttière (barres `auto` et `thin`), comme WebView2/Chromium qui lit ce pseudo-élément ; `width` et `height` en px (0 à 64), égaux s’ils sont tous deux donnés. Les autres déclarations et `::-webkit-scrollbar-thumb/track` sont refusés : le pouce et la piste viennent de `scrollbar-color` |

Ces propriétés ne sont pas conditionnelles (refusées dans `:hover` ou `@media`). Il n’y
a pas de barre superposée qui disparaît après le défilement : la barre fine reste
visible tant que le contenu déborde, comme dans WebView2 avec `::-webkit-scrollbar`.

## 3. Dessin, transparence et effets

| Fonction | Couverture |
| --- | --- |
| Couleurs | Hex RGB/RGBA courts ou longs, rgb/rgba et hsl/hsla (virgules ou espaces, alpha après `/`), `oklch()`, `oklab()`, `color-mix(in srgb\|oklch\|oklab, …)`, noms usuels, transparent, `currentColor` |
| Fonds | Couleur, linear-gradient (angle, côté ou coin : `to bottom right` suit les proportions de la boîte), radial-gradient, plusieurs couches de dégradés |
| Coins | border-radius 1 à 4 rayons circulaires en px, propriétés par coin ; normalisation des rayons |
| Bordures | border largeur solid couleur / none, width/color/style ; `border-width` à 1–4 valeurs ; côtés séparés `border-top/right/bottom/left` et leurs `-width`, `-color`, `-style` |
| Contour | `outline` largeur solid [couleur] / none, `outline-width`, `outline-color` (couleur du texte par défaut), `outline-offset` (négatif accepté) |
| Ombres | box-shadow multiple, offsets, blur, spread et inset ; couleur explicite ou `currentColor` (couleur de texte de l’élément, évaluée à la résolution des styles) |
| Images | object-fit fill/contain/cover/none/scale-down ; object-position 1 ou 2 mots-clés/% ; clip arrondi |
| Filtres | blur, brightness, contrast, grayscale, sepia, saturate, hue-rotate, invert, opacity, drop-shadow |
| Effets | opacity, translate/X/Y (px ou % de la boîte de l’élément), rotate, scale/X/Y, transform-origin |
| Arrière-plan filtré | `backdrop-filter` : mêmes fonctions que `filter`, appliquées à ce qui est déjà peint sous la boîte arrondie |
| Masque | `mask-image: linear-gradient(…)` / none (alpha du dégradé sur l’élément et son sous-arbre) |
| Texte en dégradé | `background-clip: text` (et `-webkit-`), `-webkit-text-fill-color: transparent` accepté |

Les fonctions de couleur constantes sont évaluées par le compilateur (conversion
OKLab → sRGB de CSS Color 4, canaux hors gamut bornés). `color-mix()` suit la
règle CSS : pourcentages manquants complétés à 100 %, somme inférieure à 100 %
appliquée à l’alpha, mélange prémultiplié, teinte OKLCH par l’arc le plus court.
`currentColor` et les couleurs dont un canal utilise `var()` sont compilés en
`ColorExpression` statique, évaluée pendant la résolution des styles pour
`color`, `background`/`background-color` (une couleur), `border`/`border-color`
et `accent-color`, y compris dans les états (`:hover`). `currentColor` vaut la
couleur de texte calculée de l’élément (noir si aucune `color` n’est déclarée sur
l’élément ou un ancêtre : le thème n’est pas connu des styles). Dans les dégradés,
ombres et filtres, les couleurs restent constantes : `currentColor` et `var()` y
sont résolus à la compilation ou refusés. `lab()`, `lch()`, `hwb()` et
`color()` ne sont pas fournis.

Le premier fond de la liste est au-dessus des suivants. `background-image`
conserve la couleur de fond ; `background` la remplace. Les fonds bitmap CSS,
background-size/repeat/position et les rayons elliptiques ne sont pas fournis.
Utiliser `<img src={…}>` pour une image. Les bordures en pointillés ou doubles ne
sont pas implémentées. Une bordure dont un côté a sa propre largeur ou couleur suit
ses coins arrondis quand les côtés présents partagent largeur et couleur
(`border-bottom-width: 0` d’un onglet, par exemple) ; sinon elle est dessinée en
quatre bandes droites rognées à la forme arrondie extérieure. Les largeurs par côté
entrent dans le modèle de boîte. Comme dans les navigateurs, une largeur de bordure
s’arrondit au pixel physique inférieur, au moins un
([`ViewContext::scale`](../src/application/mod.rs)) : 1 px CSS fait 2/3 de pixel logique
à 150 %, ce qui change aussi le layout. Le contour (`outline`) est
tracé hors de la boîte de bordure, sous le contenu, avec les rayons agrandis du
décalage ; il ne change pas le layout. Les anneaux Tailwind (`ring-*`) restent des
`box-shadow` à étalement.

`aspect-ratio` donne la hauteur d’une boîte de hauteur `auto` depuis sa largeur :
largeur déclarée, sinon la place offerte par le parent (cellule de grille,
colonne), dans les bornes min/max. `max-content` est la largeur naturelle du
contenu sans partage de l’espace libre ; `fit-content(L)` la borne à L.
`min-content` (piste de grille, aussi dans `minmax()`) est la plus petite largeur
sans débordement : le plus long mot d’un texte (chaque mot mesuré seul avec la
typographie calculée), la largeur naturelle d’une autre feuille, la somme des enfants
d’une rangée flex sans retour à la ligne, sinon le plus large ; une largeur en px est
gardée. `width: min-content` sur un élément n’est pas fourni (erreur).

`align-items: baseline` (et `align-self`) aligne, dans une rangée flex, les premières
lignes de base : celle d’un texte se déduit de sa taille et de sa hauteur de ligne
calculées (`(ligne − taille) / 2 + 0,8 × taille`), celle d’un conteneur de son premier
enfant dans le flux ; sans texte, le bas de la boîte. Approximation : la police réelle
n’est pas interrogée, une police différente par élément peut décaler d’un pixel ou deux.
Dans une colonne ou une grille, `baseline` vaut `start`.

Les dégradés acceptent les stops en pourcentage et les positions intermédiaires
déduites. Les angles linéaires suivent CSS : 0deg vers le haut, 90deg vers la
droite. Les directions simples `to top/right/bottom/left` fonctionnent aussi.
Les radiaux acceptent circle/ellipse, closest/farthest-side/corner, des rayons
explicites (`circle 40px`, `30px 20px`, `80% 170%` en pourcentages de la boîte) et
`at` avec mots-clés ou pourcentages. Pas de répétition.
Référence : [CSS Images](https://www.w3.org/TR/css-images-3/).

Les ombres de boîte sont des masques BGRA prémultipliés, calculés à la demande
avec trois passes de flou séparables puis conservés tant que taille/style ne
changent pas. Translation et opacité réemploient l’image. Bornes : 16 ombres,
4 millions de pixels par masque et 16 Mio de pixels cumulés par composant.
Le flou est une approximation gaussienne, pas une promesse de parité pixel
avec chaque navigateur.

Les filtres utilisent les effets Direct2D 1.1 sur GPU ou WARP : listes de commandes
et graphes conservés, invalidés lorsque leur contenu change. Une chaîne peut
s’appliquer à tout un sous-arbre. Bornes : 16 filtres par chaîne, 32 groupes par
scène, 8 niveaux imbriqués, 16 millions de pixels physiques par groupe ; cache
Direct2D configuré à 64 Mio. Ces bornes ne constituent pas une limite globale
de RAM du processus. Pas de filtre URL ni de mix-blend-mode.

`backdrop-filter` émet une commande `Backdrop` juste avant le fond de l’élément :
Direct2D vide la file de dessin, copie la zone de la cible déjà composée (agrandie
de trois écarts-types de flou), lui applique la chaîne d’effets natifs et la
repeint à travers la forme arrondie de l’élément (pinceau image). À l’intérieur
d’un `filter` (liste de commandes), la commande est ignorée. Le rastériseur
headless applique un flou en boîte (trois passes) aux pixels déjà dessinés.
`mask-image` ouvre un calque Direct2D dont le pinceau d’opacité est le dégradé
posé sur la boîte de bordure (le headless l’ignore). `background-clip: text`
retire le fond et peint les glyphes du sous-arbre avec ce fond (couleur, premier
dégradé ou couche) ; la couleur du texte est alors ignorée, y compris celle des
portions colorées d’un paragraphe enrichi qui n’ont pas leur propre couleur (le
headless utilise la couleur moyenne).

Les matrices de couleur suivent [Filter Effects 1](https://www.w3.org/TR/filter-effects-1/).
Le [mode alpha prémultiplié Direct2D](https://learn.microsoft.com/en-us/windows/win32/direct2d/color-matrix)
assure une composition correcte après transformation des couleurs. Les rendus
GPU et WARP peuvent différer sur les filtres et les bords anticrénelés.

Les transformations conservent l’ordre translate, rotate, scale, une fois par
groupe. Skew, matrices, 3D et ordre inverse sont refusés. Les restrictions
spatiales des champs, sliders, selects, couches et scrolls restent celles du
[moteur d’animation](animation.md).

## 4. Unités, expressions et variables

Les dimensions, offsets, flex-basis, font-size et espacements (padding, margin,
gap, row-gap, column-gap) acceptent em/rem, vw/vh, `calc()`, `min()`, `max()` et
`clamp()`. `em` suit la taille de police courante (la taille héritée pour
font-size, celle de l’élément pour les espacements) ; rem suit celle de la racine ;
vw/vh suivent le viewport en unités logiques. Le layout réévalue ces expressions lors du
redimensionnement. Les espacements en expression sont évalués à la résolution des
styles puis appliqués comme des px déclarés (padding et gaps négatifs ramenés à 0) ;
le `%` y est refusé (pas de boîte de référence). Les feuilles de framework gardent
leur conversion des `rem` en px à la compilation. Les additions combinent des longueurs compatibles ; les
multiplications/divisions impliquent un scalaire sans unité. Cycles, résultats
non finis et divisions invalides produisent une erreur.

```css
.responsive { width: calc(100% - 2rem); height: max(100px, 20vh); padding: 1em calc(1rem + 2vw); }
.title { font-size: clamp(22px, 3vw, 38px); }
@media screen and (max-width: 800px) {
    .header { flex-direction: column; gap: 12px; }
}
```

Les media queries acceptent screen/all, only/not, des alternatives séparées par
virgule, `and`, min/max-width/height, width/height, orientation et
`prefers-reduced-motion: reduce | no-preference` (réglage du runtime,
`UiRuntime::set_reduced_motion`) ; leur imbrication
fonctionne. Les seuils sont en px. Elles peuvent changer la typographie,
l’apparence partagée et le layout conditionnel : dimensions, marges, padding,
gaps, direction, alignements, paramètres flex, `display` (none, block, flex,
grid), pistes `grid-template-columns` / `grid-template-rows` et `order`. Un
`display` visible dans une règle conditionnelle annule le `display: none` de la
base (motif Tailwind `hidden md:flex`). Les mêmes changements sont possibles dans
les états (`:hover`, combinateurs).

Les min/max de dimensions, rayons, ombres et longueurs des filtres
restent en px ; line-height accepte px ou facteur. Les media queries de préférences
et les changements conditionnels de position/overflow, fonds dégradés ou animations
ne sont pas fournis. La syntaxe de plages CSS4 est acceptée (`(width >= 700px)`,
`(400px < width <= 800px)`, `<`/`>` stricts à 0,01 px près) ; les seuils acceptent
px, `rem` et `em` (16 px).

### Requêtes de conteneur

```css
.panneau { container-type: inline-size; container-name: accueil; }
/* ou : container: accueil / inline-size; */
.mixes { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); }
@container (min-width: 48rem) {               /* @3xl de Tailwind */
    .mixes { grid-template-columns: repeat(4, minmax(0, 1fr)); }
}
.hero {
    flex-direction: column;
    @container accueil (width >= 64rem) { flex-direction: row; }   /* @5xl */
}
```

- `container-type: inline-size | size | normal`, `container-name` (un ou plusieurs
  noms, `none`) et le raccourci `container: noms [/ type]` déclarent un conteneur ; ils
  ne sont pas conditionnels (refusés dans `:hover` ou `@media`).
- `@container [nom] [not] (condition) [and (condition)…]` : `min-width`, `max-width`,
  `width`, `inline-size`, et avec `container-type: size` `min-height`, `max-height`,
  `height`, `block-size`, en syntaxe classique ou de plage. Pas de `or` ni de requêtes
  de style (`style(…)`), refusés à la compilation.
- La requête interroge la **boîte de contenu** du plus proche ancêtre conteneur dont le
  nom correspond (sans nom : le plus proche conteneur), jamais l’élément lui-même ;
  sans conteneur la condition est fausse (`not` la rend vraie).
- Mêmes capacités que `@media` : `display` (dont `none`), pistes de grille,
  `flex-direction`, `order`, gaps, dimensions, marges, padding, typographie, couleurs…
  `@container` et `@media` s’imbriquent l’un dans l’autre, et peuvent être écrits
  **dans une règle** (`.a { @container (…) { … } }`, règle des mêmes sélecteurs placée
  après elle).
- Réévaluation : après chaque layout, le runtime compare la taille de chaque conteneur à
  celle vue par la dernière résolution des styles ; si elle a changé, il relance la
  résolution puis le layout (au plus quatre passes), **sans reconstruire la vue**. Le
  premier rendu d’une vue avec conteneurs fait donc une passe de plus. Une ligne
  virtualisée résolue seule ne voit pas les conteneurs de ses ancêtres.

Correspondance Tailwind 4 (`@container` sur le parent, `@3xl:grid-cols-4` sur
l’enfant) : `@3xs` 16rem, `@2xs` 18rem, `@xs` 20rem, `@sm` 24rem, `@md` 28rem, `@lg`
32rem, `@xl` 36rem, `@2xl` 42rem, `@3xl` 48rem, `@4xl` 56rem, `@5xl` 64rem, `@6xl`
72rem, `@7xl` 80rem ; `@max-3xl:` devient `(width < 48rem)`.

Les variables `:root` fournissent les valeurs et fallbacks compilés, avec imports
locaux et détection des cycles. L’application peut surcharger les variables typées :

```rust,ignore
self.variables.set_color("--accent", Color::hex(0xc6a0ff));
self.variables.set_length("--panel-width", percent(50.));
// Lors de view(), transmettre le même contexte aux templates/includes :
let cx = TemplateContext::default().with_variables(&self.variables);
```

`CssVariables` accepte Color, Length, Number, Family et Insets. Le partage utilise
Arc avec copie à l’écriture : une ancienne vue conserve ses valeurs. Le callback
retourne true pour reconstruire la vue avec les nouvelles variables. Les classes
prennent le contexte sans analyser de chaîne CSS au runtime. Les alias, les
longueurs dans calc et les valeurs simples de couleurs, fontes, dimensions,
espacements et nombres sont raccordés. Un type incompatible utilise le fallback.
Un nouveau scope remplace la table ; cloner la table parente pour l’étendre.

### Propriétés personnalisées héritées

Un élément peut déclarer ses propres propriétés personnalisées, héritées par tout
son sous-arbre :

```css
.mix { --h: 210; --w: 40px; }
.mix:hover { --h: 30; }
.swatch {
    background: oklch(70% 0.1 var(--h));
    width: calc(var(--w) * 2);
}
```

```html
<div class="mix" style:--h={self.hue}>
    <div class="swatch"></div>
</div>
```

- Sources : déclaration `--nom: valeur` dans une classe ou un état (`:hover`…),
  `style="--w: 12px"` littéral, ou `style:--h={expr}` dont la valeur est un
  nombre (`f32`, `i32`…), une `Color`, une `Length`, des `Insets` ou une famille
  `&'static str`. Les valeurs déclarées hors de `:root` sont littérales : nombre,
  longueur (px, %, em, calc…), couleur constante ou famille entre guillemets ;
  un `var()` y est refusé.
- Lecture à l’exécution : couleurs (`color`, fonds unis, bordures, `accent-color`,
  canaux de `oklch()`/`hsl()`/`rgb()`/`color-mix()`), dimensions `width`,
  `height`, `flex-basis`, `top/right/bottom/left` (aussi dans `calc()`) et
  `font-size`. Les autres propriétés (padding, marges, gap, rayons, ombres,
  filtres, dégradés) lisent seulement les variables de l’application au moment de
  la construction de la vue, puis le `:root`.
- Priorité, de la plus forte à la plus faible : déclaration de l’élément le plus
  proche (template `style:--x`, puis états actifs, puis classes), propriétés des
  ancêtres, variables typées de l’application (`CssVariables` du contexte), valeur
  de `:root` compilée comme repli (ou repli explicite de `var(--x, repli)`). Une
  variable lue sans `:root` ni repli est une erreur de compilation.
- La portée est recalculée à chaque résolution des styles ; un changement de
  valeur (survol, nouvelle vue) relance le layout. Un sous-arbre résolu seul
  (ligne virtualisée matérialisée) ne voit pas les propriétés de ses ancêtres.

Les unités relatives ne s’appliquent pas encore à toutes les propriétés. Pas de
hot reload ; modifier le CSS demande une compilation.

## 5. Sélecteurs et animations à étapes

Sélecteurs : `.a`, listes `.a, .b`, composés `.a.b`, plusieurs pseudo-classes,
`:not(...)` simple, `:first-child`, `:last-child`, `:only-child`, `:empty`,
`:nth-child(an+b)` / odd/even, `:hover`, `:active`, `:focus`, `:focus-visible`,
`:focus-within`, `:enabled`, `:disabled`, `:checked`, `:indeterminate`, `:read-only`,
`:read-write`, `:placeholder-shown`. Le placeholder suit le véritable éditeur,
même si l’application ne reconstruit pas la vue à chaque frappe.

Combinateurs : descendant (`.card .title`), enfant (`.list > .row`), frère
adjacent (`.item + .item`) et frères suivants (`.marker ~ .after`), jusqu’à 8
composés. Chaque composé peut porter une balise, des classes, des attributs et des
pseudo-classes : `.group:hover .reveal` (motif `group-hover` de Tailwind),
`.group:focus-within .ring`, `.list .row:first-child`, `.liste > p.note`,
`.carte > .ligne[data-rang="2"]`, `[aria-expanded="true"] .chevron` ; balises et
attributs sont ceux écrits dans le template (littéraux ou `data-x={…}`,
`aria-x={…}`), enregistrés sur les éléments des templates qui utilisent une telle
règle. Le dernier composé (le sujet) commence par une classe ou une balise suivie
d’une classe : la règle est attachée à cette classe ; le compilateur produit un prédicat statique
(`StyleState::Ancestor`, `Parent`, `PreviousSibling`, `AnyPreviousSibling`)
évalué pendant la résolution des styles : les ancêtres forment une chaîne de cadres
sur la pile (classes prêtées sans copie), les frères précédents sont évalués à la
demande. La résolution reste la passe compilée existante, relancée quand le
pointeur, le focus ou l’appui changent ; un changement de survol d’un ancêtre
restyle donc ses descendants sans invalidation supplémentaire. La spécificité
d’une relation est celle de son composé.

### Pseudo-éléments `::before` et `::after`

```css
.tab.active::after {
    content: ""; position: absolute; left: 50%; bottom: -6px;
    width: 4px; height: 4px; border-radius: 2px; background: currentColor;
}
.required::before { content: "* "; color: #e05050; }
.card:hover::after { background: #fff; }
```

`content` accepte une chaîne (vide comprise), `none` ou `normal` ; sans
`content`, rien n’est généré. La boîte générée est un vrai enfant de l’élément
hôte (premier pour `::before`, dernier pour `::after`), identifiée par
`id.child("::before")`, avec un texte (`Text`) quand la chaîne n’est pas vide.
Elle accepte toutes les propriétés compilées (position, taille, fond,
typographie…). Les états et media de l’hôte (`.card:hover::after`,
`.groupe:hover .x::before`) deviennent des règles `Parent(…)` de la boîte ; la
syntaxe à un deux-points (`:before`) est acceptée. Limites : pas de `content`
conditionnel, d’`attr()`/`counter()` ni d’animation ; pas de boîte générée dans
une feuille (texte, contrôle, image) ; la boîte compte dans `:first-child` /
`:nth-child` de ses frères.

Les règles d’état générales (combinateurs, `:focus-within`, media…) peuvent aussi
déclarer `opacity` et `transform` : l’effet remplace les parties déclarées de la
base et suit la `transition` de l’élément, comme `:hover` simple.

Hors [mode framework](css-frameworks.md) : pas de sélecteurs de type/id/attribut, `:has`, sélecteur universel, autres
pseudo-éléments (`::placeholder`, `::marker`…) ou cascade globale. Les index de lignes virtualisées désignent les enfants matérialisés, pas
l’ensemble des données ; un sous-arbre virtualisé résolu seul ne voit pas ses
ancêtres.

```css
@keyframes arrive {
    0% { opacity: 0; transform: translateY(24px) rotate(-12deg); }
    55% { opacity: .8; transform: translateY(-6px) rotate(4deg); }
    100% { opacity: 1; transform: translateY(0px) rotate(0deg); }
}
.animated { animation: arrive 1300ms ease-out both; }
```

Le compilateur produit des étapes statiques (`Keyframe` du moteur), une piste par
propriété, comme un navigateur :

- **Propriétés animables** : `opacity`, `transform`, `color`, `background-color`
  (ou `background` réduit à une couleur), `border-color` (une seule couleur). Les
  couleurs sont des constantes (variables de `:root` substituées) ; `currentColor`,
  les dégradés, les tailles et les autres propriétés sont refusés avec la position
  de la déclaration (`feuille.css:2:9: Keyframe property 'width' is not animatable`).
- **Étapes partielles** : une étape ne déclare que certaines propriétés. Chaque
  propriété s'interpole entre ses propres étapes voisines qui la déclarent ; absente
  de `0%` ou de `100%` (ou de `from`/`to`), elle y prend la valeur de l'élément.
  `0%` et `100%` ne sont plus obligatoires ; `0%, 100% { … }`, `from`/`to`, l'ordre
  quelconque et les doublons sont acceptés (à un même offset, la dernière déclaration
  d'une propriété gagne). Au plus 128 étapes du moteur.
- **Courbe par étape** : l'`animation-timing-function` d'une étape s'applique, pour
  chaque propriété qu'elle déclare, jusqu'à l'étape suivante qui déclare cette
  propriété (une étape qui ne déclare que `transform` ne coupe pas le segment
  d'opacité). Sans courbe d'étape, celle de l'animation.
- **Plusieurs animations** : `animation: a 1s, b 2s infinite` et les listes des
  longhands (`animation-name: a, b; animation-duration: 1s`), les listes plus courtes
  se répétant comme en CSS ; `none` dans la liste n'anime rien à sa place. Au plus
  16 animations par élément. Quand deux animations touchent la même propriété, la
  dernière de la liste qui a un effet à cet instant gagne (une animation dans son
  délai sans remplissage arrière, ou terminée sans remplissage avant, n'en a pas).
- **Minutage** : `animation-duration` positive ; `animation-delay` positif ou
  **négatif** (l'animation démarre déjà avancée) ; `animation-iteration-count`
  entier ≥ 1 ou `infinite` ; `animation-direction` `normal`, `reverse`, `alternate`,
  `alternate-reverse` (l'état final et le remplissage arrière suivent le sens de la
  dernière, ou de la première, itération) ; `animation-fill-mode`
  `none`/`forwards`/`backwards`/`both` ; `animation-play-state` `running`/`paused`.
  Une classe basculée par la vue qui ne change que `animation-play-state` met en
  pause ou reprend l'animation sans la redémarrer.
- Les longhands sont vérifiés à la déclaration (position de l'erreur), un
  `@keyframes` inconnu est signalé sur la déclaration qui le nomme.

Modèle du moteur : opacité et transformation animées se composent avec l'état de base
de l'élément (opacités multipliées, translations ajoutées) au lieu de le remplacer ;
la « valeur de l'élément » d'une étape extrême absente est donc l'effet neutre
(`opacity: 1`, `transform: none`), ce qui donne le rendu d'un navigateur pour un
élément sans opacité ni transformation propres. Les couleurs, elles, remplacent la
valeur calculée (une étape extrême absente reprend la couleur de l'élément, après
ses transitions). La valeur reprise aux extrémités est celle de l'élément, pas le
résultat d'une animation précédente de la liste.

Coût : une animation qui ne touche que l'opacité et la transformation s'applique à la
peinture, sans résolution des styles ni layout par image. Une animation de couleur
visible relance la résolution des styles à chaque image, comme une transition de
couleur (hors du viewport, ni images ni restyle). Mouvements réduits : état final
immédiat de chaque animation (celui de son remplissage avant).

Limites : pas de nombre d'itérations fractionnaire ou nul, pas de `steps()`, pas
d'`animation-composition`, `animation-timeline`, `animation-range` ; pas d'animation
de taille, d'ombre, de dégradé ou de `currentColor` ; pas d'animation dans une règle
d'état ou une media query, ni sur un pseudo-élément ; les couleurs d'une animation de
sortie (`data-exit`) ne sont pas jouées par le fantôme. Les clés de replay Rust et
les mouvements réduits restent disponibles.

```css
@keyframes pop {
    0% { opacity: 0; }
    40% { transform: scale(1.1); animation-timing-function: ease-out; }
    100% { opacity: 1; }
}
@keyframes flash { 50% { background-color: #fff3; } }
.badge { animation: pop 400ms ease-in both, flash 1.2s -300ms infinite; }
```

`transition` accepte `opacity`, `transform`, `color`, `background-color`
(`background`), `border-color`, `width`, `height` et `all`, avec une
durée/courbe commune (linear, ease…, `cubic-bezier()`). Les couleurs de texte,
de fond uni et de bordure sont alors interpolées en sRGB à chaque image
lorsqu’un état, une variable ou une media query les change ; seules les
propriétés listées sont animées (`transition: background-color 150ms` n’anime
plus un effet de survol). Les longhands
`transition-property/-duration/-timing-function`, les dégradés et ombres
restent immédiats. Les mouvements réduits appliquent la valeur finale
directement.

`width` et `height` (et `all`) s’interpolent quand l’ancienne et la nouvelle
valeur sont des longueurs en pixels, qu’elles viennent d’une règle d’état
(`:hover`, media, combinateur) ou d’une classe basculée par la vue
(`class:replie={…}`, qui reconstruit l’arbre : la piste suit l’identité de
l’élément). Chaque image de la transition relance la résolution des styles
et le layout de l’arbre, sans reconstruire la vue : les voisins (un `flex: 1`
à côté) et les conteneurs de requêtes (`@container`) suivent la taille
intermédiaire. Pourcentages, `auto`, `calc()`/`var()` et les autres
dimensions (`min-*`, `max-*`, `padding`, `margin`…) changent sans
interpolation. Une première apparition n’est pas animée.

```css
.sidebar { width: 288px; transition: width 200ms ease; }
.sidebar.replie { width: 76px; }
```

### Révélation de hauteur (`auto` ↔ 0)

La hauteur `auto` ne s’interpole pas par `transition: height`. Pour un panneau
qu’une classe ou un état fait passer de `display: none` à affiché (et retour),
l’attribut de template `data-reveal` anime sa hauteur entre 0 et sa hauteur
naturelle, comme `interpolate-size: allow-keywords` ou le `.collapsing` de
Bootstrap :

```html
<div class="details" class:ferme={!self.ouvert} data-reveal="350ms ease">…</div>
```

```css
.ferme { display: none; }
```

- valeur : `durée [courbe] [délai]` (`data-reveal` seul vaut `350ms ease`),
  `data-reveal={transition}` (expression `Transition` Rust), ou, avec une
  [feuille de framework](css-frameworks.md), `data-reveal=".classe"` qui reprend
  la transition de hauteur déclarée par cette classe ;
- pendant l’animation l’élément reste affiché (même déjà `display: none` à la
  fermeture), son contenu est disposé à sa hauteur naturelle et rogné à la
  boîte animée (`overflow: hidden`) ; les voisins suivent, avec un layout par
  image ; une animation interrompue repart de la hauteur courante ;
- la première apparition n’est pas animée ; les mouvements réduits
  (`--reduced-motion`, exports) appliquent l’état final immédiatement ;
- en Rust : `Element::reveal(Transition::new().duration(350).easing(Easing::Ease))`.

`z-index` est aussi accepté dans une règle d’état (`.carte:focus { z-index: 3 }`).

## Organisation et vérification

- `crates/ui-macros/src/css/` : parsing, imports, sélecteurs, expressions,
  media queries, keyframes et génération de fonctions typées.
- `src/ui/styling/` : propriétés communes, héritage, états, variables et mesures de texte.
- `src/ui/layout/` : mesure, flex, pistes de grille et positionnement réel.
- `src/ui/rendering/` : scènes, fonds, filtres et cache d’ombres.
- `src/platform/windows/graphics/` : Direct2D 1.1, DirectWrite, effets,
  géométries et caches natifs, partagés par GPU et WARP.
- `tests/css/` : scénarios compilés par thème (sélecteurs et états, valeurs, texte,
  requêtes, peinture, animations, mise en page), déterministes et indépendants de la
  plateforme ; `tests/frameworks/` pour Tailwind et Bootstrap.

Hors mode framework, une propriété ou une combinaison non prise en charge est refusée avec un
diagnostic de compilation ; les validations natives limitent aussi les valeurs
et allocations. Les tests de pixels couvrent alpha, clips, coins, textes,
dégradés et filtres, à trois échelles DPI sur WARP et GPU explicite.

Le cache de filtres ne supprime pas la relecture des exports PNG et du chemin
layered de repli ; les fenêtres sont présentées par DirectComposition sans relecture.
Il n’existe toujours pas de mesure comparative permettant de promettre un gain face à
Tauri/WebView au-delà de la [mesure de RustMusic](rustmusic-html.md). Les backends
macOS et Linux restent à écrire.
