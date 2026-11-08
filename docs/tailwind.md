# Tailwind CSS dans les templates

Les classes utilitaires de [Tailwind CSS v4](https://tailwindcss.com/docs) s’écrivent
directement dans les `class="…"` des fichiers `.ui` et des macros `ui!` :

```html
<div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-white/5 hover:bg-white/10
            @3xl:grid text-[13px] dark:text-neutral-300">
```

Le compilateur de templates les traduit à la construction en styles natifs, comme les
classes d’une feuille [`css_file!`](css.md) : pas de Node.js, pas de feuille générée,
pas d’analyse au lancement. Aucune dépendance n’est ajoutée (le thème par défaut de
Tailwind 4.3.3 est repris dans `crates/ui-macros/src/tailwind/theme.css`, licence MIT).

## Activer Tailwind

Comme avec Tailwind v4, la feuille de l’application importe `tailwindcss` :

```css
/* styles/app.css */
@import "tailwindcss";
@import "theme.css";          /* imports locaux habituels */

@theme {
    --font-sans: "Inter", system-ui, sans-serif;
    --color-accent-500: var(--acc-500, #22c55e);
}
@custom-variant dark (&:where(.dark, .dark *));

.carte { padding: 20px; }     /* classes propres, toujours permises */
```

```rust,ignore
css_file!(pub mod css, "styles/app.css");
ui_file!("ui/page.ui", context = cx, styles = css)     // ou ui! { @styles(css); … }
```

Les templates qui utilisent ce module de styles acceptent alors les utilitaires. Sans
`@import "tailwindcss";`, seules les classes de la feuille sont acceptées ; une classe
inconnue qui serait un utilitaire valide le signale (« add @import "tailwindcss"; »).
`@source` est accepté et sans effet : le compilateur lit les classes là où elles sont
écrites. `@import "tailwindcss/…"` partiel, `@plugin`, `@config`, `@utility` et
`@apply` ne sont pas fournis.

## Résolution des classes

Chaque liste `class="…"` d’un élément (et chaque `class:nom={condition}`) est résolue
à la compilation par la macro `__karbon_classes!` que `css_file!` génère dans le module
de styles :

1. un nom déclaré par la feuille (`.carte`) désigne la classe de la feuille, même s’il
   existe un utilitaire homonyme ;
2. sinon, si la feuille importe Tailwind, c’est un utilitaire ;
3. sinon c’est une erreur de compilation, au bon endroit du template, avec une
   suggestion : `Unknown class 'itms-center': Unknown utility 'itms-center'; did you mean
   'items-center'?`, `Unknown color 'whit'; did you mean 'white'?`,
   `Unknown variant 'hovr:'; did you mean 'hover:'?`, ou la raison d’un refus
   (`Variant 'has-checked:' is not supported: :has() is not implemented by the engine`).

Ordre d’application, comme dans un navigateur où la feuille de l’application (hors
`@layer`) l’emporte sur la couche `utilities` :

- les utilitaires de la liste, triés dans l’ordre de la feuille que Tailwind
  générerait (variantes, puis ordre des propriétés : `px-2` suit `p-4` quel que soit
  l’ordre de l’attribut) ;
- les classes de la feuille, dans l’ordre de l’attribut ;
- les utilitaires `!important` (`!font-bold` ou `font-bold!`).

Les règles d’état de la feuille gardent un ordre supérieur à celui des utilitaires, à
spécificité égale. `!important` n’a pas d’équivalent exact : il place l’utilitaire
après la feuille, sans dépasser une règle d’état plus spécifique.

Chaque élément porte aussi le nom de ses classes (`Element::classes()`), ce qui sert aux
marqueurs `group`, `peer`, `dark` et aux dumps.

## Calcul par élément

Comme un navigateur calcule le style d’un élément, la liste est compilée en un tout :
les utilitaires produisent les déclarations de Tailwind, variables `--tw-*` comprises,
regroupées par condition (base, `hover`, `md`…) ; les variables sont résolues pour
chaque condition (celles de la base restent visibles des variantes), puis les
propriétés composées sont traduites pour le moteur :

| Tailwind | Moteur |
| --- | --- |
| `translate`, `scale`, `rotate` (`--tw-translate-x`…) | un `transform` : `translate-x-2 hover:scale-95` donne `translate(8px, 0) scale(0.95)` au survol ; `-translate-x-1/2` donne `translate(-50%, 0)`, pourcentage de la boîte de l’élément |
| `--tw-shadow`, `--tw-ring-shadow`, `--tw-inset-shadow` | une liste `box-shadow` sans les couches transparentes nulles ; `currentcolor` d’un anneau prend la couleur de texte constante de l’élément, sinon (`focus-visible:ring-2`, `shadow-[0_0_6px_currentColor]`) celle calculée à la résolution des styles |
| `--tw-blur`, `--tw-brightness`… | `filter` / `backdrop-filter` |
| `--tw-gradient-*` | `linear-gradient(to right, from 0%, via 50%, to 100%)` ; `to bottom right` (`bg-linear-to-br`) suit les proportions de la boîte peinte |
| `transition-property`, `--tw-duration`, `--tw-ease`, `transition-delay` | raccourci `transition` (propriétés animables : `color`, `background-color`, `border-color`, `opacity`, `transform`, `width`, `height`, `all` ; `box-shadow`, `filter`… sont écartées) |
| `--tw-leading` | `text-sm leading-tight` garde la hauteur de ligne de `leading-*` |
| `--tw-numeric-*` | `font-variant-numeric` |
| `--tw-content` | `content` des `before:`/`after:` |

Les longueurs sont en px (`1rem` = 16 px, comme la feuille racine d’un navigateur) :
le moteur n’accepte que des px pour les marges, paddings, rayons et ombres ; largeurs,
hauteurs et leurs limites (`max-w-[40%]`, `min-w-full`, `max-h-[calc(100vh-330px)]`,
`max-h-[72vh]`) acceptent aussi pourcentages et expressions. Les rapports de `calc()`
gardent neuf décimales (`text-xs` : 16 px de ligne, pas 15,9996). Les valeurs
du thème sont insérées dans les déclarations (`@theme inline`) ; une valeur du thème qui
lit une variable de l’application (`var(--acc-500, #22c55e)`) reste lue à l’exécution.

## Thème

Le thème par défaut est celui de Tailwind 4.3.3 : palette en `oklch()`, `--spacing:
0.25rem`, tailles de texte et leurs hauteurs de ligne, graisses, `--tracking-*`,
`--leading-*`, rayons, ombres, flous, points de rupture `sm` à `2xl`, conteneurs `@3xs`
à `@7xl`, courbes `--ease-*`, animations `spin`, `ping`, `pulse`, `bounce`.

`@theme { … }` surcharge ou étend les variables (`--color-marque-500: #336699` rend
`bg-marque-500` disponible) ; `--color-*: initial` vide un espace de noms, `--*: initial`
tout le thème. Les `@keyframes` déclarés dans `@theme` ou dans la feuille servent à
`animate-nom` (avec `--animate-nom`) et à `animate-[nom_1s_linear]`. Les options
`inline`, `static`, `default` et `reference` de `@theme` sont acceptées.

## Variantes

| Variante | Condition |
| --- | --- |
| `hover`, `focus`, `focus-visible`, `focus-within`, `active`, `disabled`, `enabled`, `checked`, `indeterminate`, `read-only`, `placeholder-shown`, `first`, `last`, `only`, `odd`, `even`, `empty`, `nth-3`, `nth-[2n+1]` | pseudo-classe de l’élément |
| `not-hover`, `not-[.x]`… | `:not()` |
| `group-hover`, `group-focus-within`, `group-disabled/nom`, `group-[.ouvert]`… | état d’un ancêtre `group` (`group/nom`) |
| `peer-checked`, `peer-focus-visible`… | état d’un frère précédent `peer` |
| `sm` … `2xl`, `max-md`, `min-[900px]`, `max-[760px]`, `portrait`, `landscape` | `@media` sur la fenêtre |
| `@md`, `@3xl`, `@max-6xl`, `@[500px]`, `@max-[760px]`, `@lg/nom` | `@container` (déclarer `@container` ou `@container/nom` sur un ancêtre) |
| `dark` | stratégie par classe : `&:where(.dark, .dark *)`, l’élément ou un ancêtre porte `class="dark"` (ou `class:dark={…}`) ; `@custom-variant dark (…)` la remplace (`(&)` : toujours) |
| `data-[state=open]`, `data-active`, `aria-expanded`, `aria-[sort=ascending]`, `group-data-[…]`, `peer-aria-…`, `not-data-…` | attributs `data-*` et `aria-*` écrits dans le template, littéraux ou calculés (`aria-expanded={…}` : `"true"`/`"false"`) |
| `in-data-[x]`, `in-focus` | état d’un ancêtre quelconque (`:where(*:…) &`) |
| `motion-safe`, `motion-reduce` | `@media (prefers-reduced-motion: …)`, suivi du réglage du runtime (`set_reduced_motion`) |
| `before`, `after` | pseudo-éléments générés (`content: ""` implicite) |
| `placeholder` | couleur du texte indicatif d’un champ (`::placeholder { color }`) |
| `*`, `[&>div:last-child]`, `[&>.x]` | règle posée sur les enfants qui vérifient le composé (comme `space-y-*`) |
| `[&:hover]`, `[.liste_&]`, `[.ligne+&]`, `[button+&]`, `[a[href]+&]`, `[@media(width>=600px)]` | variantes arbitraires dont `&` est le sujet ; les composés des autres éléments peuvent nommer une balise et des attributs |
| `@custom-variant nom (&.c)`, `(.c &)`, `(&:where(.c, .c *))`, `(&:is(…))`, `(@media …)` | variantes de la feuille |

`dark`, `group` et `peer` ne comptent pas dans la spécificité (`:where()` :
`StyleState::Where`), comme dans Tailwind : `hover:` l’emporte sur `dark:` et
`dark:hover:` sur les deux. Les variantes s’empilent (`dark:md:hover:…`).

Balises et attributs : le compilateur de templates les enregistre sur les éléments
(`Element` → `StyleState::Tag`, `StyleState::Attribute`, la correspondance des feuilles
de [frameworks](css-frameworks.md)) seulement quand une liste de classes du template ou
une règle de la feuille les lit ; les autres éléments n’allouent rien. Les règles de la
feuille peuvent aussi nommer balises et attributs hors du sujet (`.liste > p.note`,
`.carte > .ligne[data-rang="2"]`), le sujet gardant une classe.

Valeurs : `[…]` arbitraires (`_` pour une espace, opérateurs de `calc()` espacés comme
Tailwind, indication de type `[length:…]`, `[color:…]`), raccourci `(--var)` pour
`var(--var)`, propriétés arbitraires `[prop:valeur]`, valeurs négatives (`-mt-2`,
`-translate-y-1`), modificateurs d’opacité (`bg-white/5`, `bg-[#123]/50`,
`text-(--x)/[0.4]`) et de hauteur de ligne (`text-sm/6`, `text-sm/tight`), fractions
(`w-1/2`, `basis-1/3`). Une variable `(--x)` doit avoir une valeur dans le `:root` de
la feuille (règle commune du [CSS compilé](css.md#propriétés-personnalisées-héritées)).

## Utilitaires

| Famille | Utilitaires |
| --- | --- |
| Layout | `block`, `inline`, `inline-block`, `flex`, `inline-flex`, `grid`, `inline-grid`, `hidden`, `static`/`relative`/`absolute`/`fixed`, `inset-*`, `inset-x/y-*`, `top/right/bottom/left/start/end-*`, `z-*`, `order-*`, `isolate`, `overflow-*`, `overflow-x/y-*`, `overscroll-*`, `box-border/content`, `@container`, `@container/nom`, `sr-only`, `scroll-m*-*`, `scroll-p*-*` |
| Flex et grille | `flex-row/col(-reverse)`, `flex-wrap/nowrap`, `flex-1/auto/initial/none/[…]`, `grow`, `shrink`, `basis-*`, `items-*`, `self-*`, `justify-start/end/center/between`, `justify-self-*`, `place-items-*`, `place-self-*`, `gap(-x/-y)-*`, `grid-cols-*` (dont `[repeat(auto-fill,minmax(150px,1fr))]` et `auto-fit`), `grid-rows-*`, `col-span-*`, `col-span-full`, `col-start-*`, `row-start-*`, `grid-flow-row` |
| Espacements et tailles | `p*`, `m*` (négatifs, `auto`), `space-x/y-*`, `w-*`, `h-*`, `size-*` (pas de 0,25, fractions, `full`, `screen`, `px`, `auto`, conteneurs `max-w-md`), `min/max-w/h-*` (px, `%`, `full`, `vh`/`vw`, `calc()`), `aspect-*` |
| Typographie | `text-xs` … `text-9xl` (avec hauteur de ligne), `text-[13px]`, `font-sans/mono/…`, `font-thin` … `font-black`, `font-[600]`, `leading-*`, `tracking-*`, `uppercase`/`lowercase`/`capitalize`/`normal-case`, `italic`, `underline`/`line-through`/`no-underline`, `decoration-*` (couleur, épaisseur, `decoration-solid`), `underline-offset-*`, `truncate`, `text-ellipsis`, `line-clamp-*`, `whitespace-*`, `text-wrap`/`nowrap` (`text-balance`/`pretty` → `wrap`), `text-left/center/right/justify`, `tabular-nums`/`proportional-nums`/`normal-nums` |
| Couleurs | `text-*`, `bg-*`, `border(-x/y/t/r/b/l)-*`, `ring-*`, `outline-*`, `accent-*`, `shadow-*` (couleur), `from-/via-/to-*` (et positions `from-10%`) |
| Fonds | `bg-linear-to-*` (coins compris), `bg-linear-45`, `bg-gradient-to-*`, `bg-radial`, `bg-[linear-gradient(…)]`, `bg-none`, `bg-clip-text` |
| Bordures | `border`, `border-2`, `border-[1.5px]`, côtés `border-t/r/b/l/x/y` (largeur et couleur ; une couleur ne colore que les côtés qui ont une largeur), `border-solid/none`, `divide-x/y`, `divide-*` (couleur), `rounded*` (coins `t/r/b/l/tl/tr/br/bl`), `outline`, `outline-none/hidden`, `outline-offset-*`, `ring*`, `ring-offset-*`, `inset-ring-*` |
| Effets | `shadow*`, `inset-shadow*`, `opacity-*`, `blur*`, `brightness-*`, `contrast-*`, `grayscale`, `invert`, `sepia`, `saturate-*`, `hue-rotate-*`, `drop-shadow*`, `backdrop-*`, `filter-none` |
| Transformations | `translate(-x/-y)-*` (px, fractions et `full` en % de l’élément), `scale(-x/-y)-*`, `rotate-*`, `origin-*` |
| Transitions et animations | `transition`, `transition-all/colors/opacity/shadow/transform/none`, `transition-[width]`, `transition-[height]`, `duration-*`, `ease-*`, `delay-*`, `animate-spin/ping/pulse/bounce/none`, `animate-[…]` |
| Interaction | `cursor-*`, `pointer-events-none/auto`, `select-none/auto` (sans effet : le texte statique n’est pas sélectionnable) |
| Images | `object-cover/contain/fill/none/scale-down`, `object-center/top/…` |
| Sans effet | `antialiased`, `subpixel-antialiased`, `appearance-none`, `transform`, `transform-gpu/cpu`, `will-change-*` (indications de rendu) ; marqueurs `group`, `peer`, `dark` |

Un utilitaire reconnu dont le moteur ne sait pas rendre la valeur est refusé avec le
message du compilateur CSS (`border-dashed` : « Only solid borders are implemented »).

## Fonctions ajoutées au moteur

- `cursor` (et `cursor-*`) : forme du pointeur héritée par le sous-arbre, résolue avec
  les états (`disabled:cursor-not-allowed`) ; le runtime montre celle de l’élément le
  plus profond sous le pointeur ou de son plus proche ancêtre qui en déclare une (un
  champ de saisie vaut `text`). Formes : flèche, main, texte, interdit, attente,
  progression, aide, déplacement (`move`, `grab`), croix, redimensionnements.
- `pointer-events: none | auto` : sous-arbre ignoré du hit-test, du survol (`:hover`) et
  du curseur, sauf descendant qui rétablit `auto`.
- `justify-items` sur une grille (défaut du `justify-self` des enfants) ; un élément
  aligné (`start`/`center`/`end`) de largeur auto prend la largeur de son contenu ; les
  lignes implicites d’une grille de hauteur définie se partagent la hauteur libre
  (`align-content: normal`) : `grid place-items-center` centre dans les deux axes.
- Règles d’enfants (`Node::child_rule`) : bloc posé sur les enfants qui vérifient une
  condition (`:not(:last-child)`), sous leurs propres déclarations, pour `space-x/y-*`
  et `divide-x/y` (variantes comprises : `md:space-x-1`). Approximation : la règle
  l’emporte sur une marge de base de l’enfant (seules ses règles d’état passent devant).
- Règles conditionnelles (`:hover`, `@media`, `@container`) qui placent dans la grille :
  `grid-row`, `grid-column`, `span`, `justify-self` (`@3xl:col-start-1`).
- `StyleState::Where` : condition sans spécificité (`:where()`).
- Animations à étapes : `cubic-bezier()` comme courbe (`animate-ping`, `animate-pulse`).
- `style:left={…}` (et `top`, `right`, `bottom`) dans les templates.

Ajoutés pour couvrir RustMusic (tests : `tests/frameworks/tailwind/`) :

- Limites relatives (`LayoutRules::relative_limits`) : `min/max-width/height` en
  pourcentage ou en expression (`calc()`, `vh`, `vw`, `em`), résolues au layout contre
  la boîte de contenu du parent ; un pourcentage d’une taille indéfinie ne limite rien ;
  dans les règles d’état aussi (`md:max-w-[50%]`).
- Translations en pourcentage de la boîte de l’élément (`Effect::translate_percent`,
  champs `x_percent`/`y_percent`), dans les règles d’état et les `@keyframes`.
- Ombres en `currentColor` évaluées à la résolution des styles (déclarations dynamiques
  des feuilles de frameworks, `DynamicProperty::BoxShadow`) : couleur de texte finale de
  l’élément, état compris.
- Dégradés vers un coin (`LinearGradient::to_corner`, `angle_in`) : la ligne du dégradé
  est perpendiculaire à la diagonale des deux autres coins, comme en CSS.
- Pistes `repeat(auto-fill | auto-fit, piste)` (`LayoutRules::grid_auto_repeat`) :
  autant de colonnes que la largeur en contient (taille maximale de la piste si elle
  est définie, sinon minimale, écarts compris) ; `auto-fit` n’en garde pas plus que
  d’enfants. `grid-column: N / M` et `1 / -1` (`Style::SPAN_FULL`, `col-span-full`).
- Transitions de `width`/`height` depuis `transition-[width]`.
- Couleur, décalage et épaisseur des soulignements (`DecorationStyle`) : DirectWrite ne
  dessine plus la ligne, le moteur la pose sous la ligne de base de chaque ligne du texte.
- Balises et attributs des éléments lus par les variantes et les sélecteurs de la
  feuille (`StyleState::Tag`, `StyleState::Attribute`, partagés avec les feuilles de
  frameworks).
- `::placeholder { color }` (`Properties::placeholder_color`) pour les champs de saisie.
- `prefers-reduced-motion` dans `@media` (`MediaQuery::reduced_motion`,
  `StyleEnvironment::reduced_motion`) : réévalué quand le réglage change.
- `display: inline | inline-block` : un élément du moteur est une boîte, traitée en bloc
  comme dans un conteneur flex ou grille (`inline-block` garde sa largeur naturelle dans
  un bloc).
- `scroll-margin`/`scroll-padding` : zone gardée visible quand le focus amène un élément
  dans la vue ; `overscroll-behavior: contain | none` : le reste d’un défilement à la
  molette ne passe pas aux conteneurs englobants.

## Couverture sur RustMusic

L’outil de couverture lit les `class="…"` des fichiers `.svelte` de RustMusic (parties
`{…}` ignorées), écarte les classes propres d’`app.css` et des blocs `<style>` des
composants et les jetons que Tailwind n’a pas générés (feuille de `build/`), puis
compile chaque jeton comme le ferait un template :

```powershell
cargo test -p karbon-ui-macros --lib tailwind::tools::coverage -- --ignored --nocapture
# variables : RUSTMUSIC (racine du projet), TAILWIND_COVERAGE_OUT (rapport Markdown)
```

Relevé sur RustMusic (Tailwind 4.3.3) : 2 239 jetons distincts ; 112 classes propres (`app.css`, blocs `<style>`) et 8 jetons que Tailwind n’a pas générés sont écartés. **2 014 utilitaires sur 2 119 sont acceptés (95,0 %)**, soit 99,1 % des occurrences (24 338 / 24 569) ; avant les ajouts de ce relevé : 1 941 (91,6 %), 98,3 % des occurrences. Compiler les jetons un par un, validation du moteur comprise, prend 0,2 s (tests en mode debug). La liste complète des refus est à la fin du rapport (`target/tailwind-coverage.md`).

Familles les plus employées :

| Famille | Acceptés avant | Acceptés | Total | % |
| --- | ---: | ---: | ---: | ---: |
| `bg` | 264 | 274 | 288 | 95 |
| `text` | 243 | 249 | 259 | 96 |
| `border` | 104 | 112 | 116 | 97 |
| `w` | 111 | 111 | 112 | 99 |
| `shadow` | 97 | 98 | 99 | 99 |
| `h` | 86 | 86 | 89 | 97 |
| `grid-cols` | 62 | 70 | 70 | 100 |
| `opacity` | 35 | 35 | 39 | 90 |
| `ring` | 26 | 34 | 34 | 100 |
| `max-w` | 28 | 31 | 32 | 97 |
| `top` | 32 | 32 | 32 | 100 |
| `mt` | 29 | 29 | 29 | 100 |
| `px` | 29 | 29 | 29 | 100 |
| `rounded` | 28 | 28 | 29 | 97 |
| `from` | 27 | 27 | 27 | 100 |
| `gap` | 27 | 27 | 27 | 100 |
| `py` | 25 | 25 | 25 | 100 |
| `tracking` | 25 | 25 | 25 | 100 |
| `right` | 24 | 24 | 24 | 100 |
| `bottom` | 22 | 22 | 22 | 100 |
| `left` | 21 | 21 | 21 | 100 |
| `mb` | 16 | 16 | 21 | 76 |
| `p` | 21 | 21 | 21 | 100 |
| `pl` | 18 | 18 | 21 | 86 |
| `hidden` | 18 | 18 | 20 | 90 |
| `pb` | 20 | 20 | 20 | 100 |
| `leading` | 18 | 18 | 18 | 100 |
| `pt` | 18 | 18 | 18 | 100 |
| `to` | 18 | 18 | 18 | 100 |
| `max-h` | 9 | 15 | 17 | 88 |

Variantes :

| Variante | Acceptés avant | Acceptés | Total | % |
| --- | ---: | ---: | ---: | ---: |
| `dark:` | 183 | 193 | 198 | 97 |
| `hover:` | 156 | 157 | 157 | 100 |
| `group-hover:` | 61 | 61 | 64 | 95 |
| `md:` | 26 | 26 | 28 | 93 |
| `[arbitrary]:` | 1 | 4 | 26 | 15 |
| `@3xl:` | 19 | 20 | 20 | 100 |
| `disabled:` | 18 | 18 | 18 | 100 |
| `data:` | 0 | 16 | 16 | 100 |
| `max:` | 14 | 15 | 16 | 94 |
| `max-sm:` | 15 | 15 | 15 | 100 |
| `before:` | 8 | 10 | 12 | 83 |
| `@max:` | 10 | 10 | 11 | 91 |
| `max-lg:` | 11 | 11 | 11 | 100 |
| `enabled:` | 9 | 9 | 9 | 100 |
| `focus-visible:` | 8 | 9 | 9 | 100 |
| `max-md:` | 7 | 8 | 9 | 89 |
| `@5xl:` | 7 | 8 | 8 | 100 |
| `after:` | 7 | 7 | 8 | 88 |
| `focus:` | 7 | 8 | 8 | 100 |
| `sm:` | 7 | 8 | 8 | 100 |
| `focus-within:` | 7 | 7 | 7 | 100 |
| `@4xl:` | 6 | 6 | 6 | 100 |
| `@7xl:` | 6 | 6 | 6 | 100 |
| `placeholder:` | 0 | 6 | 6 | 100 |
| `@max-6xl:` | 5 | 5 | 5 | 100 |
| `lg:` | 5 | 5 | 5 | 100 |

## Oracle

`crates/ui-macros/src/tailwind/tests/oracle/oracle.css` est un extrait de la feuille que Tailwind
4.3.3 a générée pour RustMusic (thème, valeurs initiales `@property`, règles de 130
utilitaires de toutes les familles, variantes, modificateurs d’opacité, valeurs
arbitraires, couleurs d’accent du `@theme` de RustMusic). Le test
`tailwind::tests::oracle::utilities_match_the_declarations_generated_by_tailwind` compare les
déclarations produites à celles de cet extrait, après la même normalisation
(variables, `rem`, propriétés logiques, propriétés composées) et traduction par le
compilateur CSS du moteur ; les nombres doivent concorder à 1 % près. L’outil ignoré
`tailwind::tools::oracle` (`cargo test -p karbon-ui-macros --lib tailwind::tools::oracle -- --ignored --nocapture`) fait de même pour
tous les jetons acceptés avec la feuille complète : 2 025 sur 2 043 sont identiques ; les
18 autres sont des formes minifiées équivalentes que la normalisation de l’oracle ne lit
pas (`flex: 1 1 220px` écrit `flex: 220px`, `transform-origin: 0`,
`calc(calc(1/2*100%)*-1)` de `-translate-x-1/2`, dégradé à arrêts doubles), des
déclarations refusées des deux côtés (`text-shadow`, `decoration-current/45`, variable
`--gc` d’un style en ligne) ou les approximations listées plus bas (`text-balance`,
`h-dvh`). Ce test a corrigé `border-t` (style par côté), `origin-*` (pourcentages),
`grid-flow-col` et `resize-*`.

## Écarts

Refusés à la compilation (avec la raison), faute de fonction du moteur ; entre
parenthèses, les jetons distincts de RustMusic concernés (105 au total) :

- règles posées sur les descendants (`[&_a]`, `[&_li]`, `[&_p:last-child]`,
  `[&_blockquote]`) et sur un autre sujet que `&` (`[&:hover+a]:before:`,
  `[&:hover+.ligne-morceau]`) : une règle s’attache à l’élément qui porte la classe ou,
  avec `[&>…]` et `*:`, à ses enfants ; les descendants n’ont pas encore ce mécanisme
  (≈ 23, contenu Markdown des réglages et des paroles) ;
- variables connues seulement à l’exécution dans une couleur constante ou un canal
  calculé : `--ls` (teinte du morceau dans `oklch(… calc(k * var(--ls)) …)`), `--acc-on`,
  `--gc`, `--h`, `--rangs` posés en style en ligne (≈ 22) ; l’exemple garde des
  valeurs précalculées dans son `:root` ;
- `:has()` (`has-checked:`, `has-[:focus-visible]:`), `marker:`, `print`,
  `contrast-*`, `ltr`/`rtl` (3) ;
- `position`, `overflow` et `z-index: auto` conditionnels (`md:relative`,
  `max-md:overflow-y-auto`, `md:z-auto`), `sticky`, `display: contents` (5) ;
- transformation d’un conteneur qui porte une zone défilante (refusée à l’exécution,
  pas à la compilation : `-translate-x-full` sur la barre latérale) ;
- `visibility` (`invisible`), `fill-*`/`stroke-*` (le SVG prend ses couleurs dans ses
  attributs), `text-shadow`, `clip-path`, `vertical-align` (`align-*`), `overflow-wrap`
  et `word-break` (`wrap-*`, `break-*`), `hyphens-*`, `list-*`, `snap-*`, `touch-*`,
  `resize-*`, `select-text`, `whitespace-pre-line`, `cursor-zoom-in/out`,
  `decoration-dotted`, `border-dashed`, `repeating-linear-gradient`, `rounded-[inherit]`,
  `dvh`, `ch`, `grid-flow-col`, `grid-rows-none` conditionnel (≈ 33) ;
- `animate-[nom]` dont les `@keyframes` sont dans le `<style>` d’un composant Svelte et
  non dans la feuille.

Ajoutés depuis ce relevé : `w-max` (`width: max-content`), `h-[110%]` et les tailles
au-delà de 100 %, les `@keyframes` qui animent `width`/`height`
(`animate-[eq-bar-1_0.8s_ease-in-out_infinite]`), `transition` et les réglages
d’animation dans un état (`motion-reduce:transition-none`, `group-hover:animate-none`,
`animation-play-state`, `animation-direction`).

Approximations assumées : `!important` (ordre seulement), `text-balance`/`text-pretty`
(retour à la ligne simple), `h-dvh`/`svh`/`lvh` (= `vh`), dégradés interpolés en sRGB
(pas `in oklab`), un `before:`/`after:` conditionnel crée sa boîte même hors condition
(vide), variables `--tw-*` des variantes combinées entre elles non croisées
(`hover:` et `md:` actifs ensemble voient chacun la base), transitions limitées aux
propriétés animables du moteur, `display: inline` traité en bloc (comme un élément
flex), limites en pourcentage résolues contre la place offerte par le parent, position
et épaisseur par défaut d’un soulignement dessiné par le moteur (`0,12em`, `1/14 em`)
dès qu’il a une couleur, un décalage ou une épaisseur propres, `auto-fit` qui garde au
plus une piste par enfant.

## Coût de compilation

Chaque liste de classes d’un template est compilée une fois par template (les listes
identiques sont partagées), dans la macro `__karbon_classes!` ; la feuille décrite par
`css_file!` est lue une seule fois par compilation de la crate. Les utilitaires
produisent une fonction de style par liste, comme une classe de feuille.

Mesure sur `rustmusic_html` (recompilation de l’exemple après modification de `mod.rs`,
dépendances déjà compilées, trois mesures) : en release 10,4 / 10,0 / 9,5 s avant,
9,6 / 9,1 / 8,9 s après (deux composants en Tailwind, `@import "tailwindcss"` dans la
feuille, résolution des listes de tous les templates par `__karbon_classes!`) ; en debug 1,9 / 1,8 s
avant et après. L’écart est dans le bruit de mesure. L’outil de couverture compile les
2 119 jetons de RustMusic un par un en 0,2 s (debug).

## Démonstration

Tous les templates de [l’exemple RustMusic](rustmusic-html.md) (`ui/**`) reprennent les
classes Tailwind de leurs composants Svelte, dans l’ordre de la source ; les feuilles
écrites à la main sont passées de 1 993 à 320 lignes (dont 115 de jetons de thème en
`:root`) : il reste les classes propres d’`app.css` de RustMusic (`.sidebar`,
`.smart-scroll`, `.lecteur`, `.hero-reprise`, `.rg-field`…) et quelques écarts
documentés. `styles/app.css` importe `tailwindcss` avec le `@theme` de RustMusic, les
gris du contraste élevé des captures et `@custom-variant dark (&)` (captures en thème
sombre). Le détail, les écarts et les mesures sont dans
[rustmusic-html.md](rustmusic-html.md#traduction-tailwind--css).

Fidélité face aux captures Tauri : identique avant et après la conversion (accueil aux
cinq tailles : 91 à 151 éléments appariés, 99 à 100 % à moins de 2 px, médiane 0,0, écart
maximal 2,0 à 2,6 px ; menus : mêmes chiffres, sortie audio 97 % → 100 %).

## Organisation

- `crates/ui-macros/src/classes.rs` : résolution des listes de classes (feuille,
  utilitaires, couches) ;
- `crates/ui-macros/src/tailwind/` : thème (`theme.rs`, `theme.css`), découpage
  (`candidate.rs`), utilitaires par famille (`utilities/`), variantes (`variants/`),
  liste d’un élément (`set.rs`), tri (`order.rs`), variables et propriétés composées
  (`resolve.rs`), tests et oracle (`tests/`), outils de mesure ignorés (`tools/` :
  couverture et relevé de l’oracle) ;
- `tests/frameworks/tailwind/` (feuille `tailwind.css` à côté) : mise en page
  (`placement.rs` : espacements, flex, grilles, `space-*`, limites en pourcentage,
  translations en %, `auto-fill`, `col-span-full`, `inline`), couleurs et variantes
  (`variants.rs` : thème, opacité, états, `group-*`, `dark:`, `data-*`/`aria-*`/balises,
  sélecteurs de la feuille, `placeholder:`, `motion-*`, priorités), effets
  (`effects.rs` : pseudo-éléments, anneaux en `currentColor`, décorations, dégradés vers
  un coin) et interaction (`interaction.rs` : curseurs, `pointer-events`,
  `overscroll`, `scroll-margin`) ;
- `tests/templates/compilation.rs` : diagnostics des classes inconnues.
