# Karbon UI pour VS Code

Support local des templates `.ui` du projet, écrits dans la syntaxe HTML
décrite par [`docs/html-syntax.md`](../../docs/html-syntax.md) :

```html
<ul data-stagger="40ms">
  {#each self.tracks as track (track.id)}
    <li class:active={track.id == self.current} onclick={Msg::Play(track.id)}>{track.title}</li>
  {:else}
    <p class="muted">Aucun morceau &mdash; ajoute un dossier.</p>
  {/each}
</ul>
```

## Coloration

- balises HTML (`div`, `p`, `button`, `input`, `select`, `dialog`…), balises
  du moteur à tiret (`k-split`, `k-popover`, `k-virtual-list`), composants Rust
  avec majuscule, éventuellement qualifiés (`<Card>`, `<widgets::Card />`) et
  `<slot />`, chacun avec une portée distincte ;
- attributs : valeurs entre guillemets doubles ou simples, sans guillemets,
  attributs booléens (`disabled`, `switch`) et expressions Rust `attr={expr}` ;
- portées distinctes pour les événements (`onclick`, `oninput`, `onchange`,
  `onclose`…), les directives `class:nom={cond}` / `style:propriété={expr}`,
  les attributs `data-*` et `aria-*` ;
- déclarations CSS des attributs `style="…"` (propriétés, nombres, unités,
  couleurs) ;
- texte HTML brut (sans guillemets), entités (`&amp;`, `&#233;`) et
  commentaires `<!-- … -->` ;
- blocs façon Svelte : `{#if}`, `{:else if}`, `{:else}`, `{/if}`, `{#each … as … (clé)}`,
  `{/each}`, `{@include "fichier.ui" slot="nom"}`, `{@element expr}` ;
  les mots-clés sont colorés comme tels et la condition, l'itérable, le motif
  et la clé utilisent la grammaire Rust ;
- Rust entre accolades `{…}`, en texte comme en attribut, y compris les
  accolades imbriquées, chaînes (brutes), commentaires et closures
  (`{|v| Msg::Set(3, v)}`), sur plusieurs lignes si besoin.

## Édition

- commentaires `<!-- … -->` (`Maj+Alt+A`, ou `Ctrl+:` qui utilise la même
  forme faute de commentaire de ligne en HTML) ;
- fermeture automatique de `{}`, `[]`, `()`, guillemets et `<!--` ;
- indentation après une balise ouvrante non vide (hors balises vides comme
  `input`, `img`, `hr`, `br`), après `{#if …}`, `{#each …}`, `{:else}`, et
  désindentation sur `</…>`, `{:else}`, `{/if}`, `{/each}` ;
  `Entrée` entre `<div>` et `</div>` (ou entre `{#if}` et `{/if}`) ouvre une
  ligne indentée ;
- régions repliables `<!-- #region -->` / `<!-- #endregion -->`.

## Snippets

Préfixes (en anglais) : `div`, `flex`, `section`, `p`, `h1`, `span`, `expr`,
`button`, `label`, `input`, `checkbox`, `switch`, `radio`, `range`, `select`,
`option`, `img`, `dialog`, `ul`, `if`, `ifelse`, `elseif`, `each`, `eachi`,
`include`, `include-slot`, `element`, `slot`, `slot-default`, `component`,
`k-split`, `k-popover`, `k-virtual-list`, `scroll`, `data-exit`, `class:`,
`comment`. Chaque snippet existe aussi avec le préfixe `ui-…`.

Pour insérer un snippet, taper son préfixe dans un fichier `.ui`, appuyer sur
`Ctrl+Espace`, sélectionner la proposition et utiliser `Tab` pour parcourir
les champs.

L'extension est déclarative : aucun code d'extension n'est exécuté. Les
expressions utilisent la grammaire Rust disponible dans VS Code. Le thème
choisit les couleurs ; la coloration ne fournit pas l'analyse de rust-analyzer.

## Installation locale

Depuis la racine du projet, avec PowerShell :

```powershell
./tools/vscode-karbon-ui/package.ps1
code --install-extension ./artifacts/karbon-ui-language-0.1.0.vsix --force
```

Puis lancer **Développeur : Recharger la fenêtre** depuis `Ctrl+Shift+P`.
Les fichiers `.ui` du projet sont associés à **Karbon UI** par
`.vscode/settings.json`. Le mode est aussi sélectionnable en bas à droite.
L'extension ne capture pas globalement l'extension `.ui` des autres projets,
qui peut notamment appartenir à Qt.

## Périmètre

Les erreurs des templates (balise ou événement non pris en charge, clé de
`{#each}` manquante…) restent signalées par `cargo check --examples`.
Pas encore de complétion des attributs et props, navigation vers les
composants, renommage Rust dans les templates, analyse sémantique ou
formateur `.ui`. Le formatage automatique à l'enregistrement est désactivé
pour ce langage dans ce workspace afin de ne pas appliquer un formateur
HTML/Rust inadapté. Les fichiers `.rs` avec `ui!` gardent pour l'instant la
coloration Rust habituelle.

## Vérification de la grammaire

`tests/highlighting.cjs` utilise vscode-textmate et vscode-oniguruma déjà
fournis par une installation de VS Code, sans npm install. Il accepte le
chemin du dossier `resources/app` en premier argument. Les modules étant dans
`node_modules.asar`, l'exécuter avec le Node intégré à Electron :

```powershell
$env:ELECTRON_RUN_AS_NODE = '1'
& "$env:LOCALAPPDATA/Programs/Microsoft VS Code/Code.exe" tools/vscode-karbon-ui/tests/highlighting.cjs `
  "$env:LOCALAPPDATA/Programs/Microsoft VS Code/<commit>/resources/app"
```

Un second argument permet de tester une autre grammaire `source.rust`, par
exemple celle de Rust Syntax.

Le test vérifie les portées attendues (balises, attributs, événements,
directives, entités, blocs, Rust imbriqué), que le texte brut ne devient ni
Rust ni commentaire, les motifs d'indentation, que chaque snippet se colore
sans laisser d'état ouvert, et que tous les `.ui` de `examples/` et
`tests/` se terminent dans l'état initial de la grammaire.

Références : [grammaires et langues imbriquées](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide),
[configuration du langage](https://code.visualstudio.com/api/language-extensions/language-configuration-guide).
