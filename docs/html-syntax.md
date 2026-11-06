# Syntaxe HTML des templates (spécification)

Statut : **implémentée** (macros `ui!` / `ui_file!`, voir le
[guide](templates.md)). Elle remplace la syntaxe à balises Rust (`<Column>`,
`<Text>"…"</Text>`, `animate={…}`), retirée. Les écarts et précisions sont
listés en fin de document (§9).

Principe : un fichier `.ui` s'écrit comme du HTML. Il est compilé en appels
natifs à la compilation ; rien n'est interprété au runtime. Tout ce qui n'est
pas pris en charge est une **erreur de compilation** explicite, jamais un
rendu silencieusement faux.

```html
<main class="page">
  <section class="banner" data-exit="fade-out">
    <h1>Une page native, aucun navigateur.</h1>
    <p class="muted">Modifie ce fichier puis relance <b>cargo run</b>.</p>
  </section>

  <label for="name">Ton nom</label>
  <input id="name" type="text" value={self.name} oninput={Msg::Name} placeholder="Ex. David">
  <input type="checkbox" checked={self.notif} onchange={Msg::Notif}> Notifications
  <input type="range" min="0" max="100" value={self.volume} oninput={Msg::Volume}>

  <ul data-stagger="40ms">
    {#each self.tracks as track (track.id)}
      <li onclick={Msg::Play(track.id)}>{track.title}</li>
    {/each}
  </ul>

  {#if self.dialog}
    <dialog onclose={Msg::Close}><Card title="Réglages">…</Card></dialog>
  {/if}
</main>
```

## 1. Texte et expressions

- Le texte s'écrit directement, sans guillemets. Les blancs sont regroupés
  comme en HTML (suite d'espaces/retours = un espace, bords des blocs ignorés).
- `{expr}` en position de texte insère `expr` affiché (`Display`).
- `{expr}` en valeur d'attribut est une expression Rust typée ;
  `attr="texte"` est une chaîne littérale, convertie au type attendu
  (`min="0"` → nombre) avec erreur de compilation si la conversion échoue.
- Entités usuelles : `&amp; &lt; &gt; &quot; &nbsp; &#…;`.
- `{@element expr}` insère un `Element` ou un `Widget` Rust existant
  (porte de sortie vers l'API Rust : virtualisation, widgets maison…).

## 2. Blocs de contrôle (style Svelte)

```html
{#if cond} … {:else if autre} … {:else} … {/if}
{#each iterable as item (clé)} … {/each}
{#each iterable as (index, item) (clé)} … {/each}
{@include "form.ui"}
```

La clé de `{#each}` est obligatoire : elle donne une identité stable aux
éléments quand la liste est réordonnée. Seule la branche active est construite.

## 3. Balises HTML prises en charge

Les balises en minuscules sont du HTML et correspondent aux composants natifs.
Une balise inconnue ou non prise en charge est une erreur de compilation.

| Balises | Composant natif / comportement |
| --- | --- |
| `div`, `section`, `main`, `header`, `footer`, `nav`, `article`, `aside`, `form` | Conteneur bloc (empilement vertical par défaut) ; rôle sémantique transmis à l'accessibilité |
| `span` | Contenu en ligne |
| `p`, `h1`…`h6`, `label`, `small`, `blockquote`, `pre`, `code` | Paragraphe / titre : texte avec styles par défaut |
| `b`, `strong`, `i`, `em`, `u`, `s`, `a`, `code`, `span`, `br` dans un texte | Portions de texte enrichi en ligne |
| `ul`, `ol`, `li` | Liste (conteneur vertical, rôle liste/élément) |
| `button` | Bouton ; `type`, `disabled`. Avec des éléments enfants (icône + texte, `div`, composants) : conteneur cliquable de rôle Button (`onclick` obligatoire, sans `type` ni `disabled`), stylé par ses classes |
| `input type="text" / "search" / "password"` | Champ de saisie |
| `textarea` | Champ multiligne ; `value={…}`, `rows`, mêmes attributs que le champ texte sauf `onchange` |
| `input type="checkbox"` | Case à cocher ; `switch` (attribut booléen) → interrupteur |
| `input type="radio"` | Bouton radio ; `name` regroupe |
| `input type="range"` | Slider ; `min`, `max`, `step`, `orient="vertical"` |
| `select` + `option value=…` | Liste déroulante ; `value` des options = identifiant `u64` |
| `img` | Image ; `src={Arc<Image>}`, `alt` (nom accessible), `object-fit` en CSS |
| `dialog` | Modale (focus piégé, Échap) ; `placement="top"`, `autofocus`, `backdrop="rgba(…)"` |
| `hr` | Séparateur |

Éléments propres au moteur, sans équivalent HTML : nom avec tiret, à la
manière des *custom elements*.

| Balise | Composant |
| --- | --- |
| `k-split` | Deux panneaux redimensionnables (`axis`, `fraction`, `min-first`, `min-second`) |
| `k-popover` | Popover / menu / liste ancrés (`anchor="id"`, `onclose`, `menu`, `listbox`, `placement`, `offset`, `match-width`, §11) |
| `k-virtual-list` | Liste virtualisée (`items`, `row-height`, gabarit de ligne en enfant) |

`<svg>` en ligne est compilé (formes, chemins, `currentColor`), voir §10.

Les balises avec majuscule (`<Card title="…">`) sont des composants Rust
(`impl Component`), comme en Svelte ou JSX. `<slot />`, `<slot name="x" />`
et l'attribut `slot="x"` suivent les *web components*. `class`, `style="…"` et
`style:x={…}` s'appliquent à l'élément racine rendu par le composant ;
`style={…}` reste une prop.

Non pris en charge pour l'instant (erreur explicite) : `table` et dérivés,
`video`, `audio`, `iframe`, `canvas`, `script`,
`details/summary`. `<style>` dans un `.ui` est prévu dans une seconde étape ;
d'ici là, les styles viennent de `css_file!` / `styles!`.

## 4. Attributs communs

| Attribut | Effet |
| --- | --- |
| `id` | Identité explicite (unique dans l'arbre) et cible de `label for` / `anchor` |
| `class` | Classes CSS compilées : classes de la feuille, puis [utilitaires Tailwind](tailwind.md) si elle importe `tailwindcss` ; une classe inconnue est une erreur de compilation avec suggestion ; `class={expr}` accepte une recette Rust |
| `style="…"` | Déclarations CSS compilées, avec la même couverture que `css_file!` |
| `title` | Infobulle (délai 500 ms, voir §11) |
| `hidden` | Non construit |
| `disabled` | Contrôle inactif |
| `aria-label` | Nom accessible |
| `aria-hidden` | `"true"`, `"false"` ou `{bool}` : sous-arbre retiré de l'arbre d'accessibilité (dessin, clics et focus inchangés) |
| `data-drag-region` | sans valeur, `"true"`, `"false"` ou `{bool}` : la zone déplace la fenêtre (barre de titre personnalisée, comme `data-tauri-drag-region`) ; un appui hors des boutons, champs et curseurs lance le déplacement natif, un double clic agrandit ou restaure. Inactif quand un menu, popover ou dialogue est ouvert |
| `role`, `aria-expanded`, `aria-haspopup` | Rôle d’accessibilité et déclencheur de menu (§11) |
| `tabindex="-1"` | Exclu de la navigation clavier |
| `draggable`, `ondrop` | Glisser-déposer interne |

## 5. Événements

| Événement | Où | Valeur passée au message |
| --- | --- | --- |
| `onclick={msg}` | Tout élément, `button`, `li`… | aucune (message cloné) |
| `oninput={Msg::X}` | `input` texte | `String` à chaque frappe |
| `onchange={msg}` | `input` texte | aucune, à la validation (Entrée) |
| `oninput={Msg::X}`, `onkeydown={Msg::X}` | `textarea` | comme le champ texte ; Entrée insère un retour, les flèches verticales déplacent le caret de ligne en ligne |
| `onkeydown={Msg::X}` | `input` texte | `Key` de chaque touche (flèches, Page préc./suiv., Entrée, Échap, Début, Fin, Tab, effacement, espace), avant le champ ; Haut, Bas, Page préc. et Page suiv. sont alors consommées (liste de suggestions, §11) |
| `oninput={Msg::X}` | `input type="range"` | `f64` pendant le glissement |
| `onchange={Msg::X}` | `input type="range"` | `f64` à la fin de l'interaction |
| `onchange={Msg::X}` | `checkbox` | `bool` (nouvelle valeur) |
| `onchange={msg}` | `radio` | aucune (message cloné) |
| `onchange={Msg::X}` | `select` | `u64` (valeur de l'option) |
| `onclose={msg}` | `dialog`, `k-popover` | aucune |
| `oncontextmenu={Msg::X}` | Tout élément | `ContextMenuRequest` |
| `ondrop={Msg::X}` | Cible de dépôt | `DropEvent` |

Un événement non applicable à la balise (`oninput` sur une case à cocher)
est une erreur de compilation qui indique les événements valides.

## 6. Animations

Les animations d'entrée, d'état et les transitions s'écrivent **en CSS**
(`animation`, `@keyframes`, `transition`, pseudo-classes) — déjà compilés.

Les attributs `data-*` réservés couvrent ce que le CSS ne peut pas exprimer :

| Attribut | Effet |
| --- | --- |
| `data-exit="fade-out"` | Animation de sortie quand l'élément disparaît (préréglages ou `@keyframes`) |
| `data-stagger="40ms"` | Décale l'animation d'entrée de chaque enfant |
| `data-replay={valeur}` | Rejoue l'animation quand la valeur change |

Les noms et durées sont vérifiés à la compilation (suggestion en cas de faute).
Les autres `data-*` sont acceptés et ignorés, comme en HTML.

## 7. Feuille de style par défaut

Comme un navigateur, le moteur applique une feuille par défaut avant les
styles de l'application : conteneurs en bloc vertical, `display:flex` en ligne
par défaut (comme sur le web), titres `h1`…`h6` en gras et dégressifs, marges
de `p`, listes avec retrait, `b/strong` en gras, `i/em` en italique,
`a` souligné avec couleur d'accent, `code/pre` en police à chasse fixe.
`overflow: auto | scroll` rend un conteneur défilant.

## 8. Migration

L'ancienne syntaxe (`<Column>`, `<Row>`, `<Text>"…"</Text>`, `<If>`, `<For>`,
`animate={…}`, `on_click`…) est retirée. Correspondances principales :

| Avant | Après |
| --- | --- |
| `<Column gap=12>` | `<div style="gap:12px">` (ou classe CSS) |
| `<Row>` | `<div style="display:flex">` |
| `<Text>"Bonjour"</Text>` | `<p>Bonjour</p>` ou `<span>Bonjour</span>` |
| `<Button on_click={Msg::X}>"Ok"</Button>` | `<button onclick={Msg::X}>Ok</button>` |
| `<TextInput value={v} on_change={Msg::N}/>` | `<input type="text" value={v} oninput={Msg::N}>` |
| `<If test={c}>…<Else>…</Else></If>` | `{#if c}…{:else}…{/if}` |
| `<For each={xs} item="x" key={x.id}>` | `{#each xs as x (x.id)}` |
| `<include src="a.ui" />` | `{@include "a.ui"}` |
| `animate={Animation::fade_in()}` | `animation: fade-in …` en CSS |
| `<Scroll>` | `style="overflow:auto"` |
| `<Modal>` | `<dialog>` |

## 9. Écarts et précisions de l'implémentation

Précisions (comportements que la spécification ne fixait pas) :

- **Paragraphes** : le texte et les balises en ligne voisins d'un conteneur
  forment un seul paragraphe (boîte anonyme). Un élément en ligne seul
  (`<span>`, `<small>`, `<code>`, `<label>`) devient le paragraphe et garde ses
  attributs. `code`, `small` et `label` sont des balises en ligne, promues en
  paragraphe quand elles sont seules.
- **Éléments en ligne d'un conteneur flex ou grille** : comme en HTML, chaque `<span>`
  (et chaque texte anonyme) devient un élément flex séparé, les blancs entre eux sont
  ignorés. Le `display` est lu à la compilation dans `style="…"`, sinon dans les classes
  littérales (`class="row tag"` : la dernière qui déclare `display` dans sa règle de
  base) ; les variantes conditionnelles (`@media`, `@container`, `:hover`, `class:x`) et
  `class={expr}` ne comptent pas (paragraphe). Dans un conteneur bloc, les `<span>`
  restent un paragraphe ; s'ils portent `id`, `title` ou `onclick` (refusés dans un
  paragraphe) sous un conteneur dont les classes peuvent être flex, ils restent séparés.
- **Débordement** : un conteneur de template sans `overflow` est `overflow: visible`
  (défaut CSS) : il ne rogne ni ses enfants ni un `::after` placé hors de lui, et ses
  coins arrondis ne découpent rien. `overflow: hidden | clip | auto | scroll` rogne (en
  suivant les coins arrondis). Le texte, les contrôles, dialogues et popovers gardent
  leur clip ; un `Element` construit en Rust garde le défaut natif (rognage).
- **Portions** : seules les propriétés de texte (taille, famille, graisse,
  couleur, décoration, italique par défaut de `i/em`) s'appliquent à une portion ;
  `id`, `title`, `href` et `onclick` hors `<a>` y sont refusés.
- **`{#each}`** : un chemin (`items`, `self.tracks`, `pair.0`, `data::LISTE`,
  `crate::X`) — identifiants reliés par `.` ou `::`, `static` et `const` compris —
  est parcouru par référence ; toute autre expression (appel, indexation, `0..n`,
  `xs.iter().rev()`, `*x`) est consommée.
- **Libellés** : le texte qui suit une case/un radio devient son libellé (comme
  le rendu HTML), sauf si un `<label for>` vise la case : ce libellé reste alors un
  élément à part. `<label for>` donne un nom accessible au champ visé ; placé à côté
  d'une case ou d'un interrupteur (même parent), un clic dessus la bascule (message
  `onchange` avec la valeur inverse de `checked`). Pour un champ texte, un clic sur
  le libellé ne déplace pas le focus.
- **Rôles** : `div` n'a pas de rôle (sauf avec `aria-label`) ; `main`, `nav`,
  `header`, `footer`, `aside` → rôles Main, Navigation, Banner, ContentInfo,
  Complementary ; `section`, `article`, `form` → Group ; `ul/ol/li` → List/ListItem ;
  `h1`…`h6` → Heading ; `a` → Link ; `img` → Image.
- **Dialogues** : `<dialog>` et `<k-popover>` déclarés n'importe où sont
  remontés dans la pile de couches de la racine ; le nom d'un `dialog` vient de
  `aria-label` ou de son premier titre ; `k-popover` exige `aria-label` et
  `onclose`.
- **Glisser-déposer** : `draggable={(groupe, clé)}` (ou `DragSource`) ; la cible
  de `ondrop` reprend ce couple ou `dropzone={(groupe, clé)}`.
- **`k-virtual-list`** : `items={collection possédée}` (ex. `Rc<Vec<T>>`),
  `item="nom"`, `row-height` ; le gabarit de ligne est construit à la demande,
  sans accès à `self` ni aux ressources du contexte.
- **`data-exit`** : `nom [durée] [easing] [délai]`, 220 ms `ease-in` par défaut ;
  le fantôme ne réserve pas sa place (les voisins se replacent aussitôt).
  `data-stagger` retarde selon l'index, y compris pour un élément ajouté plus tard.
- **`ui!`** : le balisage doit aussi être une suite de jetons Rust valide
  (guillemets équilibrés, pas de `//`) ; les fichiers `.ui` n'ont pas cette limite.
- **Composants** : `title` et `onclick` sont des props comme les autres ; un
  littéral `attr="…"` passe un `&str` (les nombres s'écrivent `size={16.}`).

Extensions ajoutées (utiles aux exemples portés) : `class:nom={cond}`,
`style:propriété={expr}` (valeur Rust typée, ex. `style:width={FILL}`),
`style:--nom={expr}` (propriété personnalisée héritée, voir [css.md](css.md#propriétés-personnalisées-héritées)),
`style={ViewStyle}`, `{#each …}{:else}…{/each}`, `{@include "f.ui" slot="x"}`,
`<form onsubmit>` avec `button type="submit"` et Entrée dans les champs,
`indeterminate` sur une case, `readonly`, `maxlength`, `option selected`,
`placeholder` sur `select`, `k-popover menu`.

Écarts :

- **`input type="password"`** : ajouté au moteur pour l'occasion
  (`TextInput::password()`) : puces à l'écran, valeur masquée pour
  l'accessibilité (pas de motif texte), copier/couper refusés.
- **Autres `input`** (`number`, `email`, `date`, `file`…) : erreur explicite.
- **Listes** : retrait sans puces ni numéros (pas de `list-style` dans le
  sous-ensemble CSS).
- **`overflow: auto | scroll`** : un seul axe défile (vertical si `overflow-y`
  défile) ; hors template, ces valeurs rognent comme `hidden`.
- **`<style>` dans un `.ui`** : toujours prévu pour une seconde étape (erreur
  explicite).
- **`font-style`** : l'italique existe pour les portions (`i`, `em`,
  `blockquote`) mais pas comme propriété CSS de classe.


## 10. SVG en ligne

Les icônes s’écrivent en SVG, collées telles quelles depuis Iconify (lucide, tabler,
heroicons, mynaui…). Le dessin est **compilé** : chemins, formes et transformations
deviennent des données statiques (`SvgImage`) ; rien n’est analysé à l’exécution.

```html
<button class="icon-button" onclick={Msg::Play} aria-label="Lecture">
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
    <g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2">
      <path d="M6 3l14 9-14 9z"/>
      <circle cx="12" cy="12" r="10"/>
    </g>
  </svg>
</button>
<svg src="icons/heart.svg" class="size-5" aria-label="Favori" />
```

| Élément | Attributs |
| --- | --- |
| `svg` | `viewBox`, `width`, `height` (nombre = px, ou `em`/`rem`/`%`), `preserveAspectRatio` (`xMidYMid meet` par défaut ou `none`), attributs de présentation hérités, attributs communs (`id`, `class`, `style`, `title`, `aria-label`, `onclick`…) ; `xmlns`, `version`, `role`, `focusable` acceptés ; `src="fichier.svg"` |
| `g` | `transform`, attributs de présentation hérités par ses enfants |
| `path` | `d` complet : M L H V C S Q T A Z, absolus et relatifs, syntaxe compacte (`M.5.5-1-1`, drapeaux d’arc collés) |
| `circle`, `ellipse`, `rect` (`rx`/`ry`), `line`, `polyline`, `polygon` | Géométrie en unités du viewBox (ou `px`) |
| `title` | Nom accessible du dessin (si pas d’`aria-label`) ; `desc` et `metadata` sont ignorés |

Présentation (héritée par `g`, surchargée par l’enfant) : `fill`, `stroke` (couleur
CSS constante, `none` ou `currentColor`), `stroke-width`, `stroke-linecap`,
`stroke-linejoin`, `stroke-miterlimit`, `fill-rule`, `fill-opacity`,
`stroke-opacity`, `opacity`, `transform` (`matrix`, `translate`, `scale`, `rotate`
avec ou sans centre, `skewX`, `skewY`) ; `clip-rule` est accepté (sans effet hors
d’un `clipPath`). Valeurs par défaut du SVG : `fill` noir, pas de trait, épaisseur 1.

- **`currentColor`** vaut la couleur CSS calculée de l’élément `<svg>` (`color`,
  héritée du bouton parent) : `:hover`, `:active` et les transitions de `color`
  recolorent l’icône. Sans couleur déclarée, la couleur de texte du thème.
- **Taille** : `width`/`height` donnent la taille par défaut (en px, multipliée par
  l’échelle du contexte) ; les classes et `style` la remplacent (`.size-5 { width:
  20px; height: 20px }`). Sans `width`/`height`, la taille du viewBox. Le dessin est
  centré et mis à l’échelle uniformément dans la boîte de contenu.
- **`<svg src="…">`** lit un fichier `.svg` relatif au template (prologue XML, doctype
  et commentaires tolérés, dépendance suivie par Cargo) ; les attributs de la balise
  remplacent ceux de la racine du fichier. La balise n’a pas d’enfants.
- **Accessibilité** : avec `aria-label` ou `<title>`, rôle Image et nom ; sinon
  l’icône est décorative (le nom vient du bouton qui la contient).
- **Limites** (erreur de compilation explicite) : pas de `text`, `use`/`defs`/`symbol`,
  dégradés et motifs (`url(#…)`), `clipPath`, `mask`, `filter`, tirets
  (`stroke-dasharray`), `class`/`id`/`style` sur les sous-éléments, expressions
  `{…}` ou blocs dans le dessin, `transform` sur la racine (utiliser la propriété CSS
  `transform` ou un `<g>`). `opacity` d’un groupe multiplie l’opacité de chaque forme
  (les recouvrements internes ne sont pas composés en groupe). Les arcs et quadratiques
  sont convertis en cubiques (erreur < 0,03 % du rayon). 1 024 formes au plus.
- **Rendu** : Direct2D garde une géométrie par forme (unités du viewBox, mise à
  l’échelle par la transformation) et un style de trait par combinaison bouts /
  jonctions / limite d’onglet ; le headless aplatit les courbes et remplit/trace au
  centre des pixels (jonctions en onglet et biseau arrondies).

## 11. Menus, listes de choix et infobulles

Un menu déroulant s’écrit comme en HTML/ARIA : un déclencheur `aria-haspopup` +
`aria-expanded`, et un `<k-popover>` ancré dont les entrées portent `role`. Le moteur
fournit le placement, l’affichage au-dessus de tout (non rogné par les conteneurs
défilants), la fermeture (clic extérieur, Échap, choix d’une entrée, perte du focus,
ancre disparue), le focus (entrée dans le menu, retour au déclencheur), la navigation
au clavier et l’accessibilité.

```html
<button id="profil" class="profil" onclick={Msg::ToggleProfil}
        aria-haspopup="menu" aria-expanded={self.profil_ouvert}>
  <span class="avatar">D</span> David
  <svg viewBox="0 0 24 24" width="14" height="14"><path d="m6 9 6 6 6-6" fill="none" stroke="currentColor" stroke-width="2"/></svg>
</button>
{#if self.profil_ouvert}
  <k-popover anchor="profil" menu placement="bottom-end" offset="8"
             aria-label="Menu du profil" onclose={Msg::FermerProfil}
             class="menu" data-exit="fade-out 120ms">
    <button class="menu-item" role="menuitem" onclick={Msg::Parametres}>
      <svg …lucide:settings…></svg> Paramètres
    </button>
    <div class="menu-sep" role="separator"></div>
    <button class="menu-item danger" role="menuitem" onclick={Msg::Quitter}>
      <svg …lucide:log-out…></svg> Quitter l’application
    </button>
  </k-popover>
{/if}

<k-popover anchor="recherche" listbox match-width aria-label="Suggestions"
           onclose={Msg::FermerSuggestions} class="suggestions">
  {#each self.suggestions as s (s.id)}
    <div class="suggestion" role="option" onclick={Msg::Ouvrir(s.id)}>…</div>
  {/each}
</k-popover>
```

| Attribut | Effet |
| --- | --- |
| `menu` / `listbox` (`k-popover`) | Menu (rôle Menu, entrées `role="menuitem"`) ou liste de choix (rôle List, options `role="option"`) : seules les entrées sont focalisables ; Haut/Bas (en boucle), Début/Fin, Entrée/Espace ; choisir une entrée envoie son message puis `onclose` |
| `combobox` (`k-popover`) | Liste de suggestions non modale sous un champ texte (`anchor` = le champ), voir plus bas |
| `placement` | `top`, `right`, `bottom` (défaut), `left`, suivi de `-start` (défaut), `-center` ou `-end` (`bottom-end` : sous l’ancre, bords droits alignés). Si le côté choisi manque de place et que l’opposé en a plus, le panneau bascule ; il est ensuite décalé pour rester dans la fenêtre |
| `offset="8"` | Distance à l’ancre en px (défaut 4, échelle du contexte appliquée) |
| `cross-offset="16"` | Décalage le long du côté, après l’alignement, en px (−256 à 256) : vers la droite pour `top`/`bottom`, vers le bas pour `left`/`right`. `placement="bottom-end" cross-offset="16"` : menu qui déborde de 16 px à droite de son bouton (menus « … » de RustMusic) |
| `match-width` | Au moins la largeur de l’ancre (sa hauteur pour `left`/`right`) : liste sous un champ de recherche |
| `role` (tout élément) | `menuitem`, `menu`, `option`, `listbox`, `separator`, `group`, `button`, `img`, `heading`, `navigation`, `none`/`presentation` ; autre valeur : erreur |
| `aria-expanded="true\|false"` / `{bool}` | État exposé par le motif UI Automation ExpandCollapse ; Expand/Collapse d’un lecteur d’écran activent le déclencheur si l’état diffère |
| `aria-haspopup` | `true`, `menu`, `listbox`, `dialog` ; exige `aria-expanded` (UI Automation n’a pas d’équivalent propre) |

- **Style** : le panneau et ses entrées sont stylés par leurs classes (fond, bordure,
  rayon, ombre, largeur, padding) ; sans fond déclaré, le thème fournit surface et
  bordure. `:hover` et `:focus-visible` signalent l’entrée active (le survol d’une
  entrée lui donne aussi le focus, comme un menu natif).
- **Nom accessible** d’un bouton riche (icône + texte) : son texte littéral, icônes
  exclues ; `aria-label` le remplace. Une icône `<svg>` sans `aria-label` ni `<title>`
  est décorative (absente de l’arbre d’accessibilité).
- **Animations** : entrée par la propriété CSS `animation` du panneau, sortie par
  `data-exit` (fantôme non interactif).
- **Limites** : un seul popover ouvert à la fois (pas de sous-menus en cascade) ; un
  popover `menu`, `listbox` ou panneau est modal pour le clavier : à l’ouverture, le
  focus quitte le déclencheur. Une entrée `menu`/`listbox` envoie `onclose` après son
  message : des interrupteurs qui laissent le panneau ouvert s’écrivent dans un panneau
  simple (sans `menu`).

**Suggestions pendant la saisie** (`combobox`) : la liste est ancrée au champ, peinte
au-dessus de tout, mais **non modale** — le focus reste dans le champ (la frappe
continue), la page reste cliquable. Le champ envoie ses touches par `onkeydown` ;
l’application tient la suggestion en surbrillance (`class:active`) et la choisit sur
Entrée.

```html
<input id="recherche" type="search" value={self.q.as_str()} oninput={Msg::Saisie}
       onkeydown={Msg::Touche} aria-label="Rechercher">
{#if self.suggestions_ouvertes}
  <k-popover anchor="recherche" combobox match-width offset="8" aria-label="Suggestions"
             onclose={Msg::FermerSuggestions} class="suggestions">
    {#each self.resultats as (i, r) (r.id)}
      <div class="suggestion" class:active={self.active == Some(i)} role="option"
           onclick={Msg::Ouvrir(i)}>{r.titre}</div>
    {/each}
  </k-popover>
{/if}
```

| Action | Effet |
| --- | --- |
| Frappe, flèches gauche/droite, effacement | Dans le champ (le focus ne bouge pas) |
| Haut / Bas / Page préc. / Page suiv. | Envoyées par `onkeydown` seulement (consommées par le champ) |
| Entrée | `onkeydown(Key::Enter)` puis `onchange` du champ |
| Clic sur une option | Son message, puis `onclose` |
| Échap, Tab, clic hors de la liste et du champ, molette hors de la liste, fenêtre inactive | `onclose` ; le clic hors de la liste agit quand même sur la page |

Les options gardent le rôle `option` (liste exposée à l’accessibilité) ; le survol ne
leur donne pas le focus. Pas encore de `aria-activedescendant` : l’option en
surbrillance n’est pas annoncée.

Infobulles : `title="…"` (ou `title={expr}`) affiche une infobulle après 500 ms au
survol ou au focus clavier, dessinée aux couleurs du thème (l’application Tauri
utilise l’infobulle native de WebView2, de même rôle). Elle est acceptée sous un
élément transformé (`:hover { transform: scale(1.05) }`) et se place alors sur la
géométrie non transformée.
