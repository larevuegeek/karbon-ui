# Templates HTML compilés

Deux exemples complets sont écrits en templates :

```powershell
cargo run --offline --release --example rustmusic_html
cargo run --offline --release --example bootstrap_demo
```

`rustmusic_html` découpe une vraie page en composants (`examples/rustmusic_html/ui/`,
un dossier par zone : barre latérale, accueil, lecteur, menus) avec ses feuilles CSS et
ses composants Rust ; `bootstrap_demo` tient en une page
([page.ui](../examples/bootstrap_demo/page.ui)). Les scénarios minimaux de chaque
construction (blocs, boucles à clé, composants et slots, includes, balisage HTML, SVG,
diagnostics de compilation) sont dans `tests/templates/`. La
[spécification](html-syntax.md) fixe le contrat.

Un fichier `.ui` s’écrit comme du HTML : il est traduit en appels natifs **à la
compilation**. Il n’y a ni navigateur, ni DOM, ni JavaScript, ni parseur au
runtime. Tout ce qui n’est pas pris en charge est une erreur de compilation
explicite (balise, attribut, événement, entité, durée…), avec une suggestion
quand un nom ressemble à un nom connu.

```html
<main class="page">
  <section class="banner" data-exit="fade-out">
    <h1>Une page native, aucun navigateur.</h1>
    <p class="muted">Modifie ce fichier puis relance <b>cargo run</b>.</p>
  </section>

  <label for="name">Ton nom</label>
  <input id="name" type="text" value={self.name.as_str()} oninput={Msg::Name} placeholder="Ex. David">
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

## Appeler un template

```rust,ignore
use karbon_ui::ui::prelude::*;
css_file!(mod css, "views/styles.css");

fn view(&self, _: &ViewContext) -> Element {
    let cx = TemplateContext::default();
    ui_file!("views/home.ui", context = cx, styles = css)
}
```

`ui_file!` lit le fichier relativement au fichier Rust appelant ; Cargo
recompile quand le fichier ou ses includes changent. Options : `context`,
`scope`, `slots`, `styles`. `ui!` accepte le même balisage dans le code Rust,
précédé des directives facultatives `@context(cx);`, `@scope(id);`,
`@slots(slots);` et `@styles(css);` :

```rust
use karbon_ui::ui::prelude::*;
#[derive(Clone)]
enum Msg { Play }
let title = "Bonjour";
let page: Element = ui! {
    <section style="padding: 24px; gap: 16px">
        <h1>{title}</h1>
        <button onclick={Msg::Play}>Lecture</button>
    </section>
};
# let _ = page;
```

Dans `ui!`, le balisage doit aussi être une suite de jetons Rust valide : pas de
guillemet isolé, pas de `//` (commentaire Rust) ni de préfixe réservé (`a#b`).
Les fichiers `.ui` n’ont pas ces contraintes ; les préférer pour du texte rédigé.
Les noms Rust utilisés (`Msg`, variables, `self`) doivent être en portée à l’appel.
Un template a un seul élément racine.

## Texte et expressions

- Le texte s’écrit sans guillemets. Les blancs sont regroupés comme en HTML :
  une suite d’espaces ou de retours devient un espace, les bords des blocs sont
  ignorés. `<pre>` conserve les blancs.
- `{expr}` dans le texte affiche `expr` avec `Display`.
- `attr={expr}` passe une valeur Rust typée ; `attr="texte"` est un littéral,
  converti au type attendu par les balises natives (`min="0"` → nombre, avec
  erreur de compilation si la conversion échoue). Les accolades ne sont pas
  interpolées dans les guillemets.
- Entités : `&amp; &lt; &gt; &quot; &apos; &nbsp; &#233; &#xE9;`, ainsi que
  `&mdash; &ndash; &hellip; &laquo; &raquo; &rarr; &middot; &lbrace; &rbrace;`…
  Une entité inconnue est une erreur ; un `&` isolé reste un `&`.
- `<!-- commentaire -->` est ignoré.
- `{@element expr}` insère un `Element` existant (il garde son identité) ou un
  `Widget` Rust (porte de sortie vers l’API Rust : menus, widgets maison…).

## Blocs

```html
{#if cond} … {:else if autre} … {:else} … {/if}
{#each iterable as item (clé)} … {:else} liste vide {/each}
{#each iterable as (index, item) (clé)} … {/each}
{@include "form.ui"}
{@include "track.ui" slot="track"}
```

Seule la branche active est construite. La clé de `{#each}` est obligatoire :
elle donne une identité stable à chaque élément quand la liste est réordonnée.
Un chemin (`self.tracks`, `items`, `data::LISTE`, `crate::X.0` : identifiants
reliés par `.` ou `::`, `static` et `const` compris) est parcouru **par
référence**, comme en Svelte ; une autre expression (appel, indexation, `0..n`,
`xs.iter().rev()`) est consommée.
`{#each}` construit tous ses éléments : pour des milliers de lignes, utiliser
`<k-virtual-list>`.

`{@include}` insère un fragment pendant la compilation (chemin relatif au fichier
qui inclut, dans le package Cargo ; cycles et fichiers absents signalés à la
compilation). Un fragment peut contenir plusieurs nœuds et viser un slot.
Chaque élément qui a des descendants est construit dans sa propre fonction
(`template::subtree`, `#[inline(never)]`) : une page entière faite d'includes ne
tient pas dans un seul cadre de pile et se construit en debug sur la pile de
1 Mio du thread principal. Le code des expressions s'exécute donc dans une
fermeture : `return`, `?` et `.await` n'y visent pas la fonction englobante.

## Balises

Les balises en minuscules sont du HTML et correspondent aux composants natifs.

| Balises | Composant natif |
| --- | --- |
| `div`, `section`, `main`, `header`, `footer`, `nav`, `article`, `aside`, `form` | Conteneur vertical ; rôle d’accessibilité (`main` → Main, `nav` → Navigation, `header` → Banner, `footer` → ContentInfo, `aside` → Complementary, autres → Group ; `div` sans rôle sauf `aria-label`) |
| `ul`, `ol`, `li` | Liste et éléments de liste (rôles List / ListItem), retrait à gauche |
| `p`, `h1`…`h6`, `blockquote`, `pre` | Paragraphe : un texte enrichi (titres : rôle Heading) |
| `b`, `strong`, `i`, `em`, `u`, `s`, `a`, `code`, `small`, `span`, `label`, `br` | Portions de texte en ligne |
| `button` | Bouton ; `type` (`button`, `submit`, `reset`), `disabled` |
| `input type="text"` / `"search"` / `"password"` | Champ de saisie ; `value`, `placeholder`, `maxlength`, `readonly` ; `password` affiche des puces, masque la valeur à l’accessibilité et refuse copier/couper |
| `textarea` | Champ multiligne (`TextInput::multiline`) ; `value={…}` (pas de contenu entre les balises), `oninput`, `onkeydown`, `placeholder`, `maxlength`, `readonly`, `rows` (hauteur naturelle en lignes, 2 par défaut) ; Entrée insère un retour à la ligne, Tab garde la navigation au focus |
| `input type="checkbox"` | Case ; `checked`, `indeterminate` ; `switch` → interrupteur |
| `input type="radio"` | Bouton radio ; `name` regroupe, `checked` |
| `input type="range"` | Curseur ; `min`, `max`, `step`, `orient="vertical"`, `readonly` |
| `select` + `option value="…"` | Liste déroulante ; valeurs d’options `u64`, `value={…}` (`u64` ou `Option<u64>`), `option selected` |
| `img` | Image ; `src={Arc<Image>}`, `alt` (nom accessible) ; `object-fit` en CSS |
| `dialog` | Modale (focus piégé, Échap) ; nom accessible = `aria-label` ou premier titre ; `backdrop="rgba(…)"` (voile constant), `placement="top"` (contre le haut de la fenêtre, centrée par défaut), `autofocus` (le dialogue lui-même reçoit le focus au lieu de son premier contrôle) |
| `hr` | Séparateur |
| `k-split` | Deux panneaux redimensionnables : `axis`, `fraction`, `min-first`, `min-second`, `resizable` |
| `k-popover` | Popover ancré : `anchor="id"`, `aria-label`, `onclose`, `menu`, `listbox`, `placement="bottom-end"`, `offset="8"`, `match-width` ([menus](html-syntax.md#11-menus-listes-de-choix-et-infobulles)) |
| `k-virtual-list` | Liste virtualisée : `items={collection possédée}`, `item="nom"`, `row-height`, un gabarit de ligne enfant |
| `svg` (`path`, `circle`, `ellipse`, `rect`, `line`, `polyline`, `polygon`, `g`) | Dessin vectoriel compilé, `currentColor` = couleur CSS ; `src="icone.svg"` ([détails](html-syntax.md#10-svg-en-ligne)) |
| `Card`, `widgets::Card` | Composant Rust (`impl Component`) |

Non pris en charge (erreur explicite) : `table` et dérivés, `video`,
`audio`, `iframe`, `canvas`, `script`, `details/summary`, `style`,
`fieldset`, `progress`… ainsi que les autres types d’`input` (`number`,
`email`, `date`, `file`…).

### Contenu en ligne et paragraphes

Le texte et les balises en ligne voisins forment **un seul paragraphe** (un nœud
de texte pour l’accessibilité), comme une boîte anonyme HTML :

```html
<p>Bonjour <b>toi</b>, lis le <a onclick={Msg::Guide}>guide</a> &rarr;</p>
<div>Texte <code>libre</code> dans un conteneur</div>
```

Un élément en ligne seul dans un conteneur (`<span class="tag">…</span>`,
`<small>`, `<label for>`) devient lui-même le paragraphe et garde son `id`, ses
classes, `style`, `title` et `onclick`. Dans un conteneur bloc, deux `<span>` voisins
forment **un** paragraphe ; dans un conteneur flex ou grille (`display` de `style` ou
des classes littérales), chacun devient un élément flex séparé, comme en HTML
([détails](html-syntax.md#9-écarts-et-précisions-de-limplémentation)).

Dans un paragraphe, les portions acceptent `class`, `style`, `class:x`,
`style:x` (propriétés de texte : taille, famille, graisse, couleur, décoration)
et `hidden`. Un lien est `<a onclick={msg}>texte</a>` : souligné, couleur
d’accent du thème, atteint avec Tab, activé par clic, Entrée/Espace ou
l’action *Invoke* de l’accessibilité (rôle Link). Un lien qui passe à la ligne
est cliquable sur chacun de ses fragments. `href` est refusé : un lien émet un
message. Un `<p>` ne peut pas contenir de bloc (`<div>` → erreur).

Le texte qui suit une case ou un radio (jusqu’au bloc suivant) devient son
libellé, comme visuellement en HTML ; `<label><input type="checkbox"> Texte</label>`
fonctionne aussi. `<label for="id">Texte</label>` reste visible et donne son texte
comme nom accessible au champ, curseur ou `select` d’`id="id"`.

### Contrôles et messages

Les attributs d’événement reçoivent une expression Rust qui produit le type
`Message` de l’application ; tout arrive dans `Application::update`.

| Événement | Où | Valeur passée |
| --- | --- | --- |
| `onclick={msg}` | Tout élément, `button`, `li`, lien `a`… | aucune (message cloné) |
| `oninput={Msg::X}` | `input` texte | `String` à chaque frappe |
| `onchange={msg}` | `input` texte | aucune, à la validation (Entrée) |
| `oninput={Msg::X}` / `onchange={Msg::X}` | `input type="range"` | `f64` pendant / à la fin du glissement |
| `onchange={Msg::X}` | `checkbox` | `bool` (nouvelle valeur) |
| `onchange={msg}` | `radio` | aucune (message cloné) |
| `onchange={Msg::X}` | `select` | `u64` (valeur de l’option) |
| `onclose={msg}` | `dialog`, `k-popover` | aucune |
| `onsubmit={msg}` | `form` | aucune : `button type="submit"` et Entrée dans ses champs |
| `oncontextmenu={Msg::X}` | Tout élément | `ContextMenuRequest` |
| `ondrop={Msg::X}` | Cible de dépôt | `DropEvent` |

Une variante tuple suffit pour les valeurs (`oninput={Msg::Name}` avec
`Name(String)`), une fermeture aussi (`onchange={|v| Msg::Set(3, v)}`). Un
événement non applicable (`oninput` sur une case) est une erreur de compilation
qui liste les événements valides. Un champ sans `oninput` doit être `readonly`
ou `disabled`, un bouton sans `onclick` doit être `disabled` ou `type="submit"`
dans un `<form onsubmit>`. Les valeurs restent détenues par l’application : un
`update` qui renvoie `true` reconstruit la vue.

### Attributs communs

| Attribut | Effet |
| --- | --- |
| `id="nom"` / `id={expr}` | Identité explicite (unique dans l’arbre), cible de `label for` et `anchor` |
| `class="a b"` | Classes de la feuille passée au template (`styles = css`) |
| `class={RECETTE}` / `class={[A, B]}` | Recettes Rust (`styles!`) ou classes `css::X` |
| `class:actif={cond}` | Applique la classe `.actif` si `cond` |
| `style="…"` | Déclarations CSS compilées, même couverture que `css_file!` (sans `@keyframes` ni variables) |
| `style:propriété={expr}` | Valeur Rust typée pour une propriété du nœud (`style:width={FILL}`, `style:background={gradient}`, `style:opacity={o}`…) |
| `title` | Infobulle native |
| `hidden` / `hidden={cond}` | Non construit |
| `disabled` | Contrôle inactif (boutons, champs, cases, curseurs, `select`) |
| `aria-label` | Nom accessible (conteneurs, contrôles, `dialog`, `k-popover`) |
| `tabindex="-1"` | Exclu de la navigation au clavier |
| `draggable={(groupe, clé)}`, `ondrop`, `dropzone={(groupe, clé)}` | Glisser-déposer interne ; la cible reprend le `draggable` du même élément si `dropzone` est absent |
| `data-exit`, `data-stagger`, `data-replay` | Animations (voir plus bas) ; les autres `data-*` sont ignorés |

## Feuille de style par défaut

Comme un navigateur, le moteur applique une feuille par défaut **avant** les
classes et `style="…"` de l’application :

| Balise | Défaut |
| --- | --- |
| Conteneurs | Bloc vertical, largeur disponible |
| `display: flex` | En ligne (direction `row`), comme sur le web |
| `h1` … `h6` | Gras ; 30, 24, 20, 17, 15 et 13 px |
| `p` | Marge basse de 12 px |
| `ul`, `ol` | Retrait gauche de 24 px (sans puces) |
| `blockquote` | Retrait gauche de 20 px, italique |
| `b`, `strong` | Gras |
| `i`, `em` | Italique |
| `u` / `s` | Souligné / barré |
| `a` | Souligné, couleur d’accent du thème |
| `code`, `pre` | Police à chasse fixe (`Consolas` sous Windows) |
| `small` | 13 px |
| `hr` | Ligne d’1 px, marges de 8 px |
| `button` | Largeur du contenu |

`overflow: auto | scroll` rend un conteneur défilant ([détails](css.md)).
Toutes les dimensions du template (feuille par défaut comprise) sont
multipliées par `TemplateContext::with_scale(s)`.

## Animations

L’entrée, les états et les transitions s’écrivent en CSS (`animation`,
`@keyframes`, `transition`, `:hover`, `:active`). Les attributs réservés
complètent ce que le CSS ne peut pas exprimer :

| Attribut | Effet |
| --- | --- |
| `data-exit="fade-out 200ms"` | Animation de sortie quand l’élément disparaît : préréglages `fade-out`, `slide-out-up/down/left/right`, `scale-out`, ou un `@keyframes` de la feuille |
| `data-stagger="40ms"` | Décale l’animation d’entrée de chaque enfant de son index × 40 ms |
| `data-replay={valeur}` | Rejoue l’animation d’entrée quand la valeur change |

Voir [animations](animation.md) pour le fonctionnement (fantôme non interactif).

## Composants et slots

Un composant est une structure Rust implémentant `Component` ; sa balise commence
par une majuscule :

```rust
use karbon_ui::ui::prelude::*;
#[derive(Default)]
struct Card { title: String }
impl Component for Card {
    fn render(self, cx: &TemplateContext<'_>, mut slots: Slots) -> Element {
        ui! { @context(cx); @slots(slots);
            <article style="padding: 20px; gap: 12px">
                <h2>{self.title}</h2>
                <div><slot /></div>
            </article>
        }
    }
}
let card = ui! {
    <Card title="Lecture">
        <p>Contenu réutilisable</p>
    </Card>
};
# let _ = card;
```

Les attributs deviennent les champs du composant (`-` → `_`) ; les champs
omis utilisent `Default`. `attr="texte"` passe un `&str` (vers `String`,
`Arc<str>`…), `attr={expr}` une valeur typée, un attribut seul `true`.
Les nombres s’écrivent `size={16.}`. Pour des props sans `Default` :
`props={Card { title: value }}`. `id`, `class`, `class:x`, `style="…"`,
`style:x`, `slot`, `hidden`, `tabindex` et `data-*` s’appliquent à l’élément
rendu ; `title` et `onclick` sont des props comme les autres.

`<slot />` reçoit les enfants ordinaires, `<slot name="header" />` ceux qui
portent `slot="header"`, et son contenu sert de repli. Un include peut viser un
slot : `{@include "track.ui" slot="track"}`. Les éléments sont déplacés, pas
clonés. Un slot est consommé une fois ; `Slots::unused_names()` aide à
diagnostiquer les noms non consommés.

Les ressources restent empruntées : `TemplateContext::new(&assets, "page")`, puis
`Component<Assets>`. Les composants reçoivent `cx.resources`, `cx.font`,
`cx.scale` et leur identité `cx.id`.

## Dialogues et popovers

`<dialog>` et `<k-popover>` peuvent être déclarés n’importe où dans la page
(typiquement dans un `{#if}`) : le moteur les remonte au-dessus de la page,
dans la pile de couches de la racine. Le dialogue piège le focus et se ferme
avec Échap (`onclose`).

## Sous-arbres mémorisés

La vue est reconstruite entière à chaque message qui le demande (une horloge de
lecture qui avance la position, une saisie…). Pour une longue liste, refaire chaque
ligne coûte cher : construction, styles, mesures et placement (≈ 50 ms pour 150
lignes riches). `memo={clé}` sur un conteneur dit que son sous-arbre ne dépend que de
`clé` (toute valeur `Hash`) : si la vue précédente avait cet élément, à la même
identité, avec la même clé, le sous-arbre n'est pas construit ; le runtime remet en
place l'ancien, avec ses styles résolus, ses mesures et son placement (translaté s'il
a seulement bougé, par exemple au défilement). C'est `Html.Lazy` d'Elm ou `React.memo`.

```html
{#each table.rows as row (row.id.as_str())}
    <div memo={(row, self.selected(row.n), cover_ptr(row))} class="ligne">…</div>
{/each}
```

La clé doit couvrir **tout** ce que le sous-arbre lit : données, état de l'application
(`self.…`) et ressources qui arrivent plus tard (une image chargée de façon
asynchrone : son identité, par exemple `Arc::as_ptr`). Ce que la clé oublie reste figé
à la valeur de la vue qui l'a construit. Les états d'interaction (survol, focus,
pressé), les variables CSS héritées et les requêtes média ou de conteneur suivent
toujours : les styles du sous-arbre repris sont résolus de nouveau, et un changement
de métrique y annule la reprise des mesures. En Rust : `view::lazy(id, &clé, || …)`.
`UiRuntime::set_incremental(false)` désactive la reprise (tests d'équivalence).

## Identités

Les identités automatiques dépendent de la position dans le template et du
contexte du composant ; déplacer du balisage peut donc les changer. `id="nom"`
est une identité explicite globale. Dans une boucle, les identités suivent la clé
de `{#each}`. Utiliser un contexte ou `@scope("nom");` distinct pour répéter un
template.

## Coloration dans VS Code

Le support local [Karbon UI](../tools/vscode-karbon-ui/README.md) colore le
HTML, les blocs `{#if}`/`{#each}` et le Rust entre accolades, et fournit des
snippets (`button`, `input`, `each`, `dialog`…). Le workspace associe les
fichiers `.ui` à ce langage.

## Coût et limites

La crate locale `crates/ui-macros` n’utilise que `std` et `proc_macro` ; elle
s’exécute à la compilation. Le template produit les mêmes constructeurs Rust
qu’une composition manuelle ; aucun balisage n’est analysé pendant l’affichage
et les modifications de `.ui` demandent une recompilation (pas de rechargement
à chaud). Les erreurs d’un fichier `.ui` citent `fichier:ligne:colonne`, mais
rustc rattache les erreurs de type des expressions Rust d’un fichier externe à
l’appel `ui_file!`.

Ce n’est pas un navigateur : la mise en page reste celle du moteur (flex, grille,
positions, voir [CSS](css.md)), sans fusion des marges ni sélecteurs de balises
dans les feuilles (les classes restent le seul sélecteur). Les portions de texte
n’acceptent que des propriétés de texte. Windows reste le seul backend
interactif ; le backend [headless](headless.md) permet de tester les vues ailleurs.
