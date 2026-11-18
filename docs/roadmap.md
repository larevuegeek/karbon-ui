# Feuille de route

Manques relevés en portant RustMusic sur karbon-ui, pas encore traités. Chaque entrée dit
ce qui manque et, entre parenthèses, l'effet visible dans RustMusic. Une entrée corrigée
sort de la liste (son test et sa documentation font foi).

## Clavier et souris

- **Raccourcis globaux** : l'application ne reçoit pas le clavier hors d'un champ, et
  `Key` ne connaît ni les touches caractères ni Ctrl/Alt/Cmd (Espace, flèches, Ctrl+K,
  Ctrl+Maj+M ; la page Raccourcis ne peut pas lire une nouvelle combinaison).
- **Modificateurs du clic** : un `onclick` ne dit pas si Maj ou Ctrl étaient enfoncés
  (sélection par plage ou par ajout dans l'atelier de tags).
- **Champ texte** : pas d'`onblur`, `onchange` ne part que sur Entrée, et un champ qui ne
  réagit qu'à `onchange` est refusé (il exige `oninput`).
- **Molette** : ni modificateurs (Ctrl, Maj) ni `deltaMode` dans `WheelEvent`.

## CSS

- **`visibility`** (`invisible`, `group-hover:invisible`).
- **`text-shadow`**, **`decoration-dotted`**, **`decoration-current/45`**.
- **`word-break` / `overflow-wrap: anywhere`** (chemin de fichier sans espace).
- **Unité `ch`** (`max-w-[52ch]`), **`row-span-*`**.
- **`border-dashed`**, **`cursor: zoom-in / zoom-out`**, **`inert`**.
- **Dégradés** : `bg-radial` (Tailwind v4), `repeating-linear-gradient`,
  `conic-gradient` ; **`clip-path`**.
- **Sélecteurs de voisins** : `[.a+&]`, `[&:hover+.a]`.
- **Variable d'élément dans un dégradé ou une ombre** (`color-mix(in oklab, var(--c0)
  85%, white)` avec `--c0` posé par l'élément).
- Écartés pour l'instant (décoratifs, coûteux) : `first-letter:`, fond
  `url("data:image/svg+xml…")`, `mix-blend-mode`, `<filter>` / `feTurbulence`.

## Gabarits et SVG

- **Racine `{#if}`** d'un composant.
- **Classes calculées** (`class="{x ? 'a' : 'b'}"`, classe passée en prop) et classes
  avec `( ) [ ] /` dans `class:nom={…}`.
- **`{#each … as (a, b)}`** sur un itérateur de paires (toujours lu « indice, élément »).
- **Lien ou bouton au milieu d'un paragraphe** (`<button>` dans `<h2>` ou `<p>`) ;
  bulle d'un terme de glossaire.
- **`<input type="color">`**.
- **SVG** : `<defs>`, dégradés, `<mask>` (drapeaux ronds), `stroke-dasharray`, `<text>`,
  `{#if}` et expressions à l'intérieur.

## Système et performances

- **Glisser-déposer de fichiers** depuis l'explorateur.
- **Liste des écrans** ; **observateur de taille** (`ResizeObserver`).
- **`k-virtual-list`** : les lignes n'ont accès ni à `self` ni aux ressources.
- **Glisser-déposer** : pas d'état « survolé » ni « source » pour styler la cible.
- **Passes par image** proportionnelles à ce qui est visible (translation, faits du
  pointeur, animations, survol parcourent tout l'arbre).
