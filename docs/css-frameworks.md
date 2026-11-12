# Feuilles de frameworks CSS (Bootstrap, sortie Tailwind…)

Le compilateur CSS normal est strict : toute syntaxe non prise en charge est une erreur
de compilation (voir [css.md](css.md)). Le **mode framework** accepte au contraire une
feuille existante telle quelle — `bootstrap.css` non modifié, sortie compilée de
Tailwind 4 — pour utiliser ses classes dans les templates `.ui`. Ce qui ne peut pas être
rendu est sauté et compté dans un rapport ; les feuilles de l'application restent
strictes.

## Activer

```rust
css_file!(mod bs, "vendor/bootstrap.css", framework);
// Résumé imprimé pendant la compilation (25 catégories les plus fréquentes) :
css_file!(mod bs, "vendor/bootstrap.css", framework, report);
// Liste complète, avec ligne et colonne de chaque élément sauté :
css_file!(mod bs, "vendor/bootstrap.css", framework, report = "all");

fn view(&self, _: &ViewContext) -> Element {
    ui_file!("page.ui", context = TemplateContext::default(), styles = bs)
}
```

Le module exporte `bs::COMPATIBILITY` (compteurs, type
`karbon_ui::ui::styling::tokens::Compatibility`) et `bs::REPORT` (résumé texte).
Les noms de classes du template doivent exister dans la feuille, y compris dans un
sélecteur sauté : une faute de frappe reste une erreur avec suggestion. Les listes
`class={expr}` restent possibles mais leurs classes ne sont connues qu'à l'exécution
(moins de règles résolues à la compilation).

L'exemple complet est [bootstrap_demo](../examples/bootstrap_demo.rs) :

```powershell
cargo run --release --example bootstrap_demo
cargo run --release --example bootstrap_demo -- --theme dark
cargo run --release --example bootstrap_demo -- --export demo.png --size 1280x1700 --open dropdown,collapse
cargo run --release --example bootstrap_demo -- --export switch.png --switch   # interrupteur coché
cargo run --release --example bootstrap_demo -- --report
```

Une fenêtre n'a pas de défilement de document : la page déclare sa zone défilante.
La démo garde sa barre de titre fixe et fait défiler le reste avec les utilitaires de
Bootstrap :

```html
<body class="d-flex flex-column vh-100">
  <div data-drag-region>…barre de titre…</div>
  <div class="flex-grow-1 overflow-y-auto" style="scrollbar-width: thin">…contenu…</div>
</body>
```

## Comment la cascade est compilée

1. **Lecture tolérante** ([framework/syntax.rs](../crates/ui-macros/src/css/framework/syntax.rs)) :
   grammaire générique de CSS Syntax 3 (règles, at-rules, blocs, `!important`,
   imbrication `&`), commentaires et chaînes compris.
2. **Aplatissement** : chaque règle reçoit ses conditions (`@media`, `@container`), sa
   couche (`@layer`) et son ordre. Chaque alternative d'une liste de sélecteurs devient
   une règle indexée par la clé de son sujet (classe, id, attribut, balise, universel).
3. **Par élément** : le générateur de templates transmet à la feuille la balise, les
   classes (littérales et `class:x={…}`), les attributs (littéraux et `data-*` /
   `aria-*` calculés) de chaque élément **et de ses ancêtres du même template**. La
   feuille sélectionne les règles candidates, évalue à trois valeurs (vrai, faux, à
   l'exécution) le composé sujet et les relations :
   - type, classe, id, attribut statiques : décidés à la compilation ;
   - descendant / enfant : prouvés par les ancêtres du template (`.navbar-expand-lg
     .navbar-nav`, `.row > *`) ; un parent connu qui ne correspond pas exclut la règle ;
   - frères (`+`, `~`), pseudo-classes d'état, `@media` : prédicats `StyleState`
     évalués pendant la résolution des styles.
4. **Rang** : (importance, couche, spécificité (a, b, c), ordre, rang de déclaration).
   Normales : couches dans l'ordre de première déclaration, déclarations hors couche
   au-dessus. `!important` : ordre des couches inversé, hors couche en dessous.
   `:where()` vaut zéro, `:is()` / `:not()` la plus forte de leurs alternatives.
5. **Élimination** : du rang le plus fort au plus faible, une déclaration dont tous les
   longhands (`padding` → 4 côtés, `border` → largeurs, styles et couleurs…) sont déjà
   posés par une déclaration inconditionnelle de rang supérieur (ou de même condition)
   est retirée : elle ne peut jamais gagner.
6. **Émission** : les déclarations inconditionnelles deviennent des appels de nœud
   (base) ; les conditionnelles des règles d'état `StyleState::Where(…)` (sans
   spécificité propre) ordonnées par rang ; une déclaration de base qui recouvre en
   partie une règle d'exécution de rang inférieur est aussi posée comme règle toujours
   vraie à son rang. Le résultat respecte donc l'ordre CSS même quand la résolution
   applique les états après la base.

Les pseudo-éléments `::before` / `::after` deviennent les boîtes générées habituelles
(contenu `var(--x, "/")` résolu à la compilation). `html`, `:root` et `:host` seuls
fournissent les **variables globales** (repli le plus faible de `var()`), leurs autres
déclarations s'appliquent à `<body>` sous ses propres règles.

### Sélecteurs

| Accepté | Détail |
| --- | --- |
| Type, universel `*`, classe, id, attribut | `[a]`, `=`, `~=`, `\|=`, `^=`, `$=`, `*=`, drapeau `i` ; noms échappés (`.md\:flex`, `.\31 0`) |
| Combinateurs | descendant, `>`, `+`, `~`, jusqu'à 16 composés |
| `:is()`, `:where()`, `:not()` | listes complexes (`:where(.space-y-4 > :not(:last-child))`) |
| Pseudo-classes | `:hover`, `:active`, `:focus`, `:focus-visible`, `:focus-within`, `:disabled`, `:enabled`, `:checked`, `:indeterminate`, `:read-only`, `:read-write`, `:placeholder-shown`, `:first/last/only-child`, `:nth-child`, `:nth-last-child`, `:first/last/only-of-type`, `:nth-of-type`, `:nth-last-of-type`, `:empty`, `:root` |
| Jamais vraies | `:link`, `:visited`, `:target`, `:valid`, `:invalid`, `:required`… (pas de liens ni de validation de formulaire natifs) |
| Pseudo-éléments | `::before`, `::after` (et `:before`) ; `::-webkit-slider-thumb` et `::-webkit-slider-runnable-track` (pièces du curseur, voir [contrôles](#contrôles-dessinés-par-la-feuille)), suivis éventuellement d'une pseudo-classe appliquée à l'hôte (`::-webkit-slider-thumb:active`) |
| Sautés | `:has()`, `::placeholder`, `::selection`, autres `::-webkit-*`, `::-moz-range-*`, `::file-selector-button`, préfixes `:-moz-*`, imbrication non résolue |

### At-rules

| At-rule | Traitement |
| --- | --- |
| `@media` | largeurs/hauteurs à l'exécution ; à la compilation : `print` faux, `prefers-reduced-motion: reduce` faux (le moteur applique déjà les valeurs finales en mouvement réduit), `prefers-color-scheme: dark` faux (les thèmes passent par `data-bs-theme`), `hover: hover` et `pointer: fine` vrais, résolution faux |
| `@supports` | évalué à la compilation : une déclaration est « supportée » si le moteur l'accepte (compilée ou évaluée à l'exécution), `selector()` si le lecteur l'accepte, `not` / `and` / `or` |
| `@container` | requêtes de conteneur du moteur |
| `@layer` | instructions (`@layer a, b;`) et blocs, couches anonymes ; ordre de première apparition |
| `@property` | `initial-value` devient une variable globale ; `inherits: false` est enregistré (voir écarts) |
| `@keyframes` | compilées comme dans le mode strict (étapes partielles, couleurs) ; une étape `0%`/`100%` manquante reprend la valeur neutre (`transform: none`, `opacity: 1`), ou l'opacité propre de l'élément quand l'animation la remplace |
| `@font-face`, `@import`, `@page`… | sautées et comptées (les polices sont chargées par l'application) |

### Valeurs

- `rem` vaut 16 px (taille racine des navigateurs) ; les propriétés logiques
  (`padding-inline`, `margin-block-start`…) deviennent les côtés physiques d'un texte
  horizontal de gauche à droite ; `border-radius` de 50 % et plus arrondit toute la
  boîte (cercle, pilule) ; `position: sticky` colle dans la zone défilante verticale la plus proche
  ([CSS](css.md)).
- `animation` (raccourci et longues) passe par le compilateur commun quand sa valeur
  est connue à la compilation, variables de l'élément substituées
  (`.spinner-border`, `.spinner-grow`). Une étape `0%`/`100%` absente, ou qui omet une
  propriété animée ailleurs, reprend la valeur de l'élément (opacité déclarée, sinon
  1 ; `transform: none`). Une animation infinie qui anime l'opacité la remplace,
  comme en CSS (le moteur compose sinon l'animation avec la base).
- `border-radius: inherit` et `border-*-radius: inherit` reprennent les rayons calculés
  du parent (le dernier élément d'une `.list-group-flush` suit l'arrondi intérieur de
  sa carte) ; les autres propriétés non héritées refusent `inherit`.
- `inherit` / `unset` sur une propriété de texte héritée sont acceptés sans effet ;
  préfixes `-moz-`, `-ms-`, `-o-` ignorés comme dans Chromium ; propriétés sans effet
  natif (`-webkit-text-size-adjust`, `color-scheme`, `will-change`…) acceptées.
- `display: inline-block | inline | inline-flex` (et les balises en ligne sans
  `display`) marquent une boîte **en ligne** : dans un bloc, elle garde sa largeur
  naturelle au lieu d'être étirée. Il n'y a pas de formatage en ligne (deux boutons
  voisins dans un bloc restent l'un sous l'autre : utiliser `d-flex`).
- `transition` est évaluée pendant la résolution des styles, dans l'ordre de la
  cascade, y compris composée et lue par `var()` (`.accordion-button { transition:
  var(--bs-accordion-transition) }`, `.btn`, `.form-control`). Le moteur garde un
  minutage par élément : celui du premier élément de la liste qu'il sait animer —
  couleurs (texte, fond, bordure), `opacity` / `transform`, `width` / `height` en
  pixels ; les autres (`box-shadow`, `border-radius`, `background-position`) changent
  immédiatement. Les longhands `transition-*` restent sautés. Les couleurs
  s'interpolent en alpha prémultiplié comme dans les navigateurs : le fond blanc d'un
  onglet `.nav-link` qui devient transparent s'efface sans passer par le gris. Les
  pistes sont attachées à l'identité de l'élément, stable d'une reconstruction à
  l'autre : seuls les éléments dont une couleur change sont animés.
- `z-index` d'une feuille de framework traverse, comme en CSS, les ancêtres sans
  `z-index` : `.accordion-button:focus { z-index: 3 }` peint l'élément d'accordéon qui
  le contient au-dessus du suivant (son anneau de focus n'est plus recouvert), le lien
  actif d'une `.pagination` (`z-index: 3`) passe devant son voisin. Accepté aussi dans
  les règles d'état. Les feuilles strictes gardent un `z-index` limité aux frères.
- `border-top-left-radius`… (un seul coin), dans une règle d'état ou avec `var()`, se
  pose sur les rayons en vigueur (`.btn-group` : coins intérieurs carrés).
- Bordures par côté : un côté d'au moins 2 px voisin d'un côté transparent les
  rejoint **en onglet** comme dans les navigateurs (trapèzes d'une boîte à angles
  droits, secteurs d'une boîte arrondie, côtés voisins de même couleur d'un seul
  tenant). Le caret de `.dropdown-toggle::after` (`border-top: .3em solid`, côtés
  transparents) devient un triangle, `.spinner-border` (`border-right-color:
  transparent`) un arc anticrénelé qui tourne avec ses `@keyframes`. Les bordures
  uniformes et les autres bordures par côté (couleurs opaques différentes) gardent
  leurs bandes droites, dont les coins se recouvrent.
- Calage sur les pixels physiques : comme un navigateur, le moteur peint la boîte
  d'un élément d'une feuille de framework (fond, bordure, ombres, images de fond,
  contour, rognage) avec ses bords arrondis au pixel physique le plus proche ; la largeur d'une
  bordure l'est déjà (1 px à 150 % = un pixel physique). Sans ce calage, une boîte
  placée à un demi-pixel physique (fréquent à 150 %, hauteurs de texte en tiers de
  pixel) étalait sa bordure du haut et du bas sur deux rangées grises alors que les
  côtés restaient nets (`.btn-outline-dark` irrégulier). Le layout, le texte et les
  bornes (`bounds()`) restent fractionnaires ; les feuilles strictes ne sont pas
  calées.

### Images de fond SVG

`url("data:image/svg+xml,…")` (encodée en pourcentages, ou `;base64,`) est décodée à la
compilation et son SVG compilé par le compilateur des `<svg>` de templates en un
`SvgImage` statique ([images.rs](../crates/ui-macros/src/css/framework/images.rs)) :
l'URL devient un jeton `Token::Image`, y compris dans une propriété personnalisée
(`--bs-form-select-bg-img`, `--bs-btn-close-bg`, `--bs-accordion-btn-icon`,
`--bs-form-check-bg-image`, `--bs-form-switch-bg`…). Rien n'est analysé à l'exécution :
une URL déjà vue pendant la compilation réutilise son code, et la géométrie Direct2D de
chaque forme est mise en cache par le moteur de rendu.

`background-image` (deux couches), `background-position` (mots-clés, décalages
`right .75rem center`, pourcentages, `calc()`), `background-size` (`auto`, `contain`,
`cover`, une ou deux longueurs), `background-repeat` (`no-repeat`, sinon pavage) et le
raccourci `background` d'une couche (`transparent var(--bs-btn-close-bg) center/1em auto
no-repeat`) sont évalués pendant la résolution des styles, donc aussi à travers `var()`
et selon les états (`:checked`, `:focus`, `.collapsed`…). L'image est placée dans la
boîte de remplissage, rognée à la boîte de bordure, peinte avec `currentColor` ; un SVG
sans taille intrinsèque remplit la zone comme `contain` quand la taille est `auto`.
`background-image: none` retire les couches. Les images de fichier (`url(icon.png)`)
restent refusées.

### Contrôles dessinés par la feuille

`appearance: none` sur une case à cocher, un interrupteur, un curseur ou une liste
déroulante retire le dessin natif du contrôle : sa boîte CSS (fond, bordure, rayons,
images de fond, ombres de focus) le dessine, avec les états `:checked`, `:focus`,
`:disabled`, `:active`. Le comportement, le clavier et l'accessibilité restent ceux du
contrôle natif. Sans `appearance: none`, le contrôle garde son dessin natif.

Comme tout contrôle de template, la case est contrôlée par l'application :
`checked={self.ok}` et `onchange={Msg::Ok}` (le message reçoit la nouvelle valeur, que
`update` enregistre). Le balisage Bootstrap `<input id="x" …>` suivi de
`<label class="form-check-label" for="x">` fonctionne tel quel : le libellé reste un
élément à part (stylé par `.form-check-label`) et un clic dessus bascule la case,
comme dans un navigateur.

- Case et interrupteur Bootstrap : fond bleu et coche SVG blanche une fois cochée,
  rond SVG à gauche puis à droite (`background-position`) pour l'interrupteur.
- Curseur : `::-webkit-slider-thumb` (couleur, largeur, hauteur, rayon, bordure,
  ombres réduites à leur décalage et leur étalement) et
  `::-webkit-slider-runnable-track` (couleur, hauteur, rayon, bordure) ; la piste
  occupe toute la largeur, le pouce reste entier dedans.
- Liste déroulante : le chevron natif disparaît au profit de `--bs-form-select-bg-img`.

### Boîtes générées dans un bouton

Un `<button>` au texte seul est un contrôle natif sans enfants. Quand la feuille de
framework lui génère une boîte `::before` / `::after` avec `content`
(`.dropdown-toggle`, `.accordion-button`), le template choisit à la compilation (par
une constante de la feuille) une zone cliquable de rôle bouton qui porte le texte puis
la boîte générée, en rangée centrée (sous les déclarations de la feuille). Un bouton
`disabled` ou sans `onclick` garde la forme native. Les règles d'état transmettent aussi
leurs `transform` évaluées par `var()` à l'animation : le chevron de l'accordéon ouvert
tourne de 180°, interpolé par `transition: var(--bs-accordion-btn-icon-transition)`
(`transform .2s ease-in-out`).

Un `<button aria-label="Close"></button>` vide garde son nom accessible ; son libellé
de repli (le nom) n'est pas dessiné quand la feuille peint une image de fond
(`.btn-close`).

## Variables à l'exécution

Une variable dont toutes les déclarations de la feuille ont la même valeur et qui est
définie globalement est une **constante**, substituée à la compilation. Une variable
que l'élément déclare lui-même sans règle conditionnelle plus forte (`.btn` et
`--bs-btn-padding-x`, `.btn-primary` et `--bs-btn-bg`) est aussi résolue à la
compilation : la plupart des déclarations des composants Bootstrap deviennent
statiques.

Les autres valeurs (`color: var(--bs-body-color)` qui dépend de `data-bs-theme`,
`rgba(var(--bs-primary-rgb), var(--bs-text-opacity))`, `calc()` avec variables,
`color-mix()`, ombres Tailwind composées de `--tw-*`, `em`) sont découpées en
**jetons** à la compilation ([tokens/](../src/ui/styling/tokens/mod.rs)) :

1. les propriétés personnalisées de l'élément (classes, états actifs, `style:--x`)
   sont calculées dans sa portée — leurs `var()` d'abord parmi ses propres
   déclarations, puis chez les ancêtres, les variables de l'application et enfin les
   variables globales de la feuille ; une valeur invalide garde la valeur héritée ;
2. chaque déclaration dynamique substitue ses `var()` (replis compris) puis évalue la
   grammaire de sa propriété : couleurs (`rgb`/`rgba`/`hsl`/`color-mix`, `currentColor`),
   longueurs avec `calc()`/`min()`/`max()`/`clamp()` et unités `px`, `rem`, `em`, `vw`,
   `vh`…, `padding`/`margin`/`border-*`/`border-radius`/`gap`/dimensions, ombres
   multiples, `opacity`, `transform` et `translate`/`scale`/`rotate`, typographie ;
3. une valeur invalide au moment du calcul laisse la propriété inchangée.

La portée des variables est une chaîne (un élément qui déclare ne recopie pas celles
de ses ancêtres ; la chaîne est aplatie au-delà de 16 niveaux) et les noms compilés ne
sont pas alloués. Les ombres calculées sont internées (au plus 4096 listes distinctes).

### Coût mesuré

| Mesure (release, Windows) | Valeur |
| --- | --- |
| Une déclaration dynamique (substitution + évaluation, portée de 3 niveaux) | ≈ 180 ns |
| Page `bootstrap_demo` : déclarations dynamiques par résolution complète | 612 (dont les `transition`) |
| Page `bootstrap_demo` : résolution complète des styles | ≈ 0,70 ms (sombre) à 0,84 ms (clair) |

Les déclarations dynamiques représentent donc ≈ 0,11 ms par résolution, soit 13 à 16 %
du temps de résolution de la page ; la résolution complète reste relancée par les
changements de survol, focus et appui comme pour les feuilles strictes.

Ces chiffres viennent de tests de mesure ignorés, retirés de la suite avec les autres
mesures de temps : `cost_of_one_evaluation` (retiré au commit `06731ec`) et
`resolve_cost_of_the_bootstrap_demo` (jusqu’au commit `a37ea7e`). Le compteur
`styling::tokens::evaluation_count()` (caché de la documentation) est conservé pour
refaire la mesure : 200 résolutions de la page de démonstration, puis le nombre de
déclarations évaluées par passe.

## Scores de compatibilité

Mesurés par
`cargo test -p karbon-ui-macros compatibility -- --nocapture --include-ignored`
(le second test lit la sortie Tailwind locale de RustMusic, non versionnée ; chemin
modifiable par `KARBON_TAILWIND_CSS`).

| Feuille | Règles appliquées | entièrement | Sélecteurs | Déclarations | dont à l'exécution | At-rules |
| --- | --- | --- | --- | --- | --- | --- |
| Bootstrap 5.3.3 `bootstrap.css` | 95,4 % (2397/2513) | 92,0 % | 98,5 % (2884/2927) | 95,9 % (5263/5488) | 898 | 100 % (115) |
| RustMusic, sortie Tailwind 4.3 | 95,4 % (2716/2848) | 91,5 % | 98,3 % (2861/2910) | 93,4 % (3791/4061) | 926 | 97,2 % (583/600) |

43 règles de Bootstrap et 443 de la sortie Tailwind sont écartées par une condition
fausse à la compilation (`@supports (color: color-mix(in lab, …))`, `@media print`…) :
elles ne comptent ni comme acceptées ni comme refusées.

Principales catégories sautées — Bootstrap : `text-decoration-color` avec `var()`
(56), `align-content` (36), `visibility` (20), `position` composée,
`z-index` avec `var()`, pseudo-éléments
`::file-selector-button` et `::-moz-range-thumb`. Tailwind : `content` avec `var()`
(26), `filter`/`backdrop-filter` avec `var()`, longhands `transition-*`, `animation`,
`@font-face`, `::-webkit-scrollbar-thumb`, `contain-intrinsic-size`,
`content-visibility`, `text-shadow`.

Ces scores mesurent la feuille hors contexte. Dans un élément, une déclaration
acceptée peut encore être refusée si elle est conditionnelle et que le moteur n'a pas
de résolveur d'état pour elle (`position` dans `:hover`, par exemple) : elle est alors
ignorée pour cet élément.

## Comportements Bootstrap (`data-bs-*`)

Le template est réécrit avant la génération ([bootstrap/](../crates/ui-macros/src/bootstrap/mod.rs)) ;
les déclencheurs émettent des `BootstrapAction` que le runtime applique lui-même
(table d'état par fil d'interface, [ui::bootstrap](../src/ui/bootstrap.rs)) avant de
reconstruire la vue : l'application n'a rien à câbler.

| Bootstrap | Rendu natif |
| --- | --- |
| `data-bs-toggle="collapse"` + `data-bs-target` | la cible reçoit `show` (et le déclencheur `collapsed`, `aria-expanded`) ; `data-bs-parent` forme un accordéon ; `show` écrit dans le template = ouvert par défaut. Ouverture et fermeture animées comme `.collapsing` : la hauteur passe de 0 à la hauteur naturelle (et retour), contenu rogné, au minutage de la règle `.collapsing` de la feuille (`height .35s ease`) ; dans un accordéon, un volet se referme pendant que l'autre s'ouvre |
| `data-bs-toggle="tab"` / `"pill"` / `"list"` | volets d'un même `.nav` : `active` + `show` sur le volet choisi et son onglet |
| `data-bs-toggle="dropdown"` | le `.dropdown-menu` suivant s'ouvre dans un `<k-popover menu>` ancré au déclencheur (fermeture au clic extérieur et Échap, éléments `role="menuitem"` qui ferment le menu) ; le thème `data-bs-theme` de l'ancêtre est recopié |
| `data-bs-toggle="modal"` | la `.modal` devient un `<dialog>` natif (focus piégé, Échap, voile) portant les variables `--bs-modal-*` ; placé en haut de la fenêtre (`placement="top"`, ses marges `1.75rem auto` donnent l'écart), centré avec `.modal-dialog-centered` ; le focus va au dialogue lui-même (`autofocus`), comme avec le script de Bootstrap ; `data-bs-dismiss="modal"` ferme |
| `data-bs-toggle="offcanvas"` | panneau `.offcanvas.show` fixe et voile `.offcanvas-backdrop` qui ferme ; `data-bs-dismiss="offcanvas"` |
| `data-bs-dismiss="alert"` | l'alerte englobante disparaît |
| `data-bs-toggle="tooltip"` / `"popover"` | `data-bs-title` (ou `data-bs-content`) devient l'info-bulle native (`title`) |
| `.form-check` | la case flottante (`float: left`) est rendue par une rangée flex : retrait du parent annulé, case au bord, libellé après 8 px (.5em), comme le flottement à 1.5em (2.5em pour un interrupteur) |

`karbon_ui::ui::bootstrap::set_shown("id", true)` préouvre une cible (exports),
`reset()` revient aux valeurs du template.

Non couvert : popover Bootstrap riche (seul son texte devient une info-bulle),
`scrollspy`, `carousel`, `toast`, positionnements Popper (`data-bs-placement`,
`dropup`, `dropend`), transitions `fade` (apparition et disparition des modales,
offcanvas, alertes), `.collapse-horizontal` (largeur non animée), modale `data-bs-backdrop="static"`
(Échap ferme quand même), formulaires `.was-validated`, `<a href>` (utiliser
`<button>`), boutons `.btn-check` (radio/case stylée en bouton).

## Fidélité de la démonstration

`examples/bootstrap_demo/tools/fidelity.ps1` produit la page HTML équivalente
(`reference.py` : même balisage, attributs Rust retirés), la capture avec Microsoft
Edge headless (profil temporaire sous `target/`), l'export natif, puis compare
(`compare_png.py`, bibliothèque standard). Résultats dans `artifacts/bootstrap/`,
produit localement et non suivi par git (côte à côte : Edge à gauche, natif à
droite).

Pixels dont chaque canal diffère d'au plus 16 (et 48) sur 255, à 150 %, avant
(`705ada2`) et après les contrôles dessinés par la feuille, les images SVG, les
bordures en onglet et la modale en haut :

| Capture | ≤ 16 avant | ≤ 16 | ≤ 48 avant | ≤ 48 | Écart moyen avant | Écart moyen |
| --- | --- | --- | --- | --- | --- | --- |
| `light` (1280 × 1720) | 94,8 % | 95,1 % | 96,7 % | 96,9 % | 6,15 | 5,78 |
| `dark` (1280 × 1720) | 94,8 % | 95,1 % | 96,9 % | 97,0 % | 5,23 | 5,00 |
| `light-dropdown-collapse-tab` (1280 × 1100, menu, collapse, onglet Profile) | 92,3 % | 92,7 % | 95,1 % | 95,4 % | 9,09 | 8,52 |
| `light-offcanvas` (1280 × 900) | 96,0 % | 96,2 % | 97,7 % | 98,0 % | 2,97 | 2,70 |
| `light-modal` (1280 × 900) | 78,9 % | 93,9 % | 81,8 % | 96,7 % | 24,87 | 4,38 |
| `dark-modal` (1280 × 900) | 81,0 % | 94,8 % | 95,2 % | 97,0 % | 9,37 | 3,70 |

Avec le calage des boîtes sur les pixels physiques (≤ 16 / ≤ 48 / écart moyen) :
`light` 95,0 % / 96,8 % / 5,94, `dark` 95,1 % / 97,0 % / 5,15, `light-switch`
(interrupteur coché, `-Switch`) 95,0 % / 96,8 % / 5,94. Les bords se placent au pixel
près comme dans Edge ; le gain est compensé par la dérive verticale du texte, qui
décale les boîtes de part et d'autre d'un pixel par rapport à la référence.

Les écarts restants viennent surtout du texte : la ligne de base est placée à 0,8 em
sous la demi-interligne (le navigateur utilise l'ascendante de la police, environ 1,8 px
plus bas pour 16 px) et les glyphes sont un peu plus étroits ; et d'une dérive verticale
de 6 px sous le curseur `.form-range` (voir écarts).

```powershell
cargo build --release --example bootstrap_demo
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x1720 -Name light
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x1720 -Theme dark -Name dark
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x1100 -Open dropdown,collapse -Tab profile-pane -Name light-dropdown-collapse-tab
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x900 -Open offcanvas -Name light-offcanvas
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x900 -Open modal -Name light-modal
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x900 -Theme dark -Open modal -Name dark-modal
# Interrupteur coché (--switch côté démo et référence) :
pwsh examples/bootstrap_demo/tools/fidelity.ps1 -Out artifacts/bootstrap -Size 1280x1720 -Switch -Name light-switch
# Boîtes placées par le moteur (classes et bornes logiques), à comparer au navigateur :
cargo run --release --example bootstrap_demo -- --dump boxes.txt --size 1280x1720
```

Fenêtre interactive : la page dessine sa propre barre de titre aux couleurs du thème
Bootstrap (claire ou sombre, zone de glissement, réduire / agrandir / fermer) à la
place de la barre générique du moteur ; `--title-bar` l'affiche dans un export.

Ce qui est fidèle : couleurs, thèmes clair et sombre, boutons de toutes variantes et
tailles, groupes (coins intérieurs, bordures superposées), badges, alertes et bouton de
fermeture, cartes, liste, grille `row`/`col-*` avec gouttières, barre de navigation
responsive, pagination, barres de progression, spinners, formulaires (champ, liste et
son chevron, case, interrupteur, curseur), onglets, accordéon et ses chevrons, menu
déroulant et son caret, modale, offcanvas.

Écarts connus :

- ligne de base du texte à 0,8 em (le navigateur suit les métriques de la police) ;
- pas de formatage en ligne ni de « strut » : un `inline-block` seul dans un bloc
  (`.form-range`) n'ajoute pas la descente de la ligne sous lui (6 px de moins que le
  navigateur), pas de `float`, de `visibility`, de `align-content`, de tableaux
  (`<table>` reste refusé par les templates), de `<a>` stylé en boîte ;
- le menu déroulant est placé par le popover (il s'ouvre au-dessus quand la place
  manque dessous, la référence statique déborde) ;
- les couches remontées (dialogues, popovers) ne voient pas les variables de la page :
  elles reçoivent le `data-bs-theme` de l'ancêtre et les variables globales.

## Écarts du moteur

- Propriétés enregistrées `inherits: false` (`--tw-*`) : héritées malgré tout ;
  les sorties Tailwind qui empilent `ring` et `shadow` sur un parent et un enfant
  peuvent cumuler les ombres.
- Pas de `::placeholder`, `:has()`, `@font-face`, `@scope`, `@starting-style`.
- Une déclaration conditionnelle n'est appliquée que si le moteur sait la résoudre à
  l'exécution (couleurs, typographie, dimensions, marges, padding, bordures, rayons,
  images de fond SVG, ombres, opacité, transformations, `display`, `z-index`,
  `transition`…) ; `position`, `overflow`, dégradés et animations
  conditionnels sont ignorés pour l'élément.
- Les ombres (`box-shadow`) ne s'interpolent pas : l'anneau de focus d'un bouton
  apparaît sans le fondu de `.15s` du navigateur ; `transition` d'une propriété non
  animée par le moteur est sans effet.
- `rem` suppose une racine de 16 px ; `vh`/`vw` suivent la fenêtre.
