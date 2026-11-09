# Accessibilité Windows : première intégration UI Automation

Le moteur expose maintenant ses contrôles à Windows UI Automation, sans crate
externe. Un client peut découvrir le champ de saisie, lire ou remplacer sa valeur,
placer le focus et activer les boutons. Cette étape constitue une première base ;
elle ne couvre pas encore toute l’accessibilité d’une application.
Les champs de saisie exposent également `TextPattern` : plages de texte, sélection,
recherche, déplacements et rectangles visibles, sur une ligne ou plusieurs.

## Ce qui est exposé

UI Automation sépare les propriétés d’un contrôle de ses capacités, appelées
« patterns ». Un champ peut ainsi exposer une valeur sans encore exposer toutes
les opérations sur les plages de texte. Voir la
[présentation des fournisseurs Microsoft](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-providersoverview).

| Élément | Informations et opérations disponibles |
| --- | --- |
| Fenêtre | Nom, rectangle écran, enfants, contrôle sous un point et élément focalisé. |
| Bouton | Nom, état actif, focus et `InvokePattern` pour déclencher son action. |
| Checkbox et Switch | `TogglePattern`, état Off/On et Mixed pour une case, désactivation et notifications. |
| Groupe radio et choix | `SelectionPattern` sur le groupe, `SelectionItemPattern` sur chaque radio, sélection unique et conteneur explicite. |
| Slider | `RangeValuePattern`, valeur, bornes, pas, orientation, lecture seule et modification numérique complète. |
| Select | `ExpandCollapsePattern` et `SelectionPattern` ; options avec `SelectionItemPattern`, sélection conservée à la fermeture. |
| Modale | Panneau avec `IsDialog = true`, nom, enfants actifs et focus ; arrière-plan exclu du snapshot et des commandes. |
| Popover et menu | Conteneur ou `Menu`, actions `MenuItem`/`InvokePattern`, séparateur, focus et événements d’ouverture/fermeture ; couche active uniquement. |
| Tooltip | HelpText de l’ancre disponible en permanence ; nœud ToolTip passif pendant l’affichage et notifications d’ouverture/fermeture. |
| Champ de saisie (une ou plusieurs lignes) | Nom, aide, valeur, lecture seule, `ValuePattern` et `TextPattern`. |
| Texte | Contenu utilisé comme nom accessible. |
| Scroll et liste | Conteneurs et enfants actuellement construits. |
| Séparateur redimensionnable | Rôle, position et focus ; pas encore de `RangeValuePattern`. |
| Boutons de fenêtre | Réduire, agrandir/restaurer et fermer par `InvokePattern`. |
| Bouton bascule (`aria-pressed`) | Button + `TogglePattern` (pas d’`InvokePattern`) ; Toggle active l’élément comme un clic. |
| Onglets (`role="tab"`, `aria-selected`) | TabItem + `SelectionItemPattern` ; la liste (`role="tablist"`) est un Tab avec `SelectionPattern` ; le panneau (`role="tabpanel"`) un Pane. |
| Case, interrupteur, radio dessinés (`aria-checked`) | Mêmes patterns que les contrôles natifs ; Toggle ou Select activent l’élément. |
| Élément courant (`aria-current`) | `AriaProperties` = `current=page` (ou `step`, `location`, `date`, `time`, `true`). |

Les propriétés comprennent aussi l’identifiant stable, la visibilité, les limites
en pixels écran et la disponibilité du focus. Les positions tiennent compte du DPI
et des écrans dont les coordonnées sont négatives.

`ValuePattern.SetValue` remplace tout le contenu ; il ne représente pas une frappe
clavier. Le moteur applique les mêmes contraintes que l’éditeur : champ actif,
lecture seule, retours à la ligne remplacés par des espaces dans un champ
monoligne (gardés dans un champ multiligne) et longueur maximale. Il envoie ensuite le
message produit par le `on_change` du champ à `Application::update`. Cette opération ne déplace pas elle-même le focus.
Voir le [contrat Microsoft de SetValue](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-ivalueprovider-setvalue).

## Plages de texte des champs de saisie

`ITextProvider` fournit le document, la sélection courante, les portions visibles
et une plage vide correspondant à un point écran. Chaque plage implémente
`ITextRangeProvider` : lecture, clone, comparaison, recherche avant/arrière,
déplacement des extrémités et sélection. Voir le
[contrat Microsoft des plages de texte](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nn-uiautomationcore-itextrangeprovider).

Les positions internes restent en octets UTF-8, comme celles de l’éditeur. Les
déplacements par caractère utilisent les positions mesurées par DirectWrite ;
ils ne passent pas au milieu d’un cluster. `GetText` renvoie un BSTR UTF-16 et sa
limite de longueur ne coupe pas une paire de substitution d’emoji. Les mots
suivent pour l’instant les séparations par espaces du champ. Le format est
uniforme ; l’unité ligne suit les lignes visuelles exposées par le moteur
(`AccessibleText::lines`, une seule pour un champ monoligne), l’unité paragraphe les
retours à la ligne, et page et document couvrent tout le champ. Un point écran est
rapporté à la ligne la plus proche, puis au caret le plus proche sur cette ligne. Les invalidations et limites de déplacement suivent les règles de
[Move](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-itextrangeprovider-move)
et [MoveEndpointByUnit](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-itextrangeprovider-moveendpointbyunit).

Les rectangles utilisent la géométrie du rendu, le scroll interne, le clipping
et le DPI. Une sélection vide n’a pas de rectangle de surbrillance. Le placeholder
est une aide, pas une partie du document. Pendant une composition IME, le texte
affiché est exposé ; une sélection externe est refusée jusqu’à la fin de la
composition pour ne pas l’annuler à partir d’indices provisoires.

`Select` modifie la sélection sans remplacer le texte, sans ajouter une entrée
d’historique et sans prendre le focus. `ScrollIntoView` révèle le début ou la fin
de la plage, y compris dans un parent défilant, sans changer la sélection. Un
champ en lecture seule reste sélectionnable ; un champ désactivé refuse ces
mutations. L’éditeur vérifie aussi la valeur attendue pour rejeter une commande
de sélection devenue obsolète avant son traitement sur le thread UI.

Les plages conservées après édition sont recalées de façon conservatrice à partir
du préfixe et du suffixe communs. Elles deviennent indisponibles après retrait
du contrôle ou fermeture. Le recalage n’est pas un journal de toutes les éditions
intermédiaires. La sélection multiple est explicitement refusée. Le seul attribut
de texte actuellement exposé est la lecture seule ; les autres attributs renvoient
la valeur réservée « non pris en charge ».

## Du widget au fournisseur Windows

Le point de départ reste `WidgetSemantics` : rôle, nom et état actif. Les widgets
existants fournissent ces informations. Un widget personnalisé doit décrire son
sens, en plus de dessiner ses pixels. Un bouton uniquement représenté par une
icône doit notamment conserver un nom compréhensible.

Les états ARIA déclarés par l’application (`Element::aria_pressed`, `aria_checked`,
`aria_selected`, `aria_current`, ou les attributs du même nom dans un gabarit)
complètent ce contrat sans widget dédié : l’état de choix exposé (`choice`) est celui
du widget natif, sinon `aria-checked`, sinon `aria-pressed` (`Toggle`), sinon
`aria-selected` (`Radio`). macOS et Linux les reprennent : VoiceOver (case AXToggle,
AXTabGroup, AXTabButton, `isAccessibilitySelected`) et AT-SPI (bouton bascule, page tab,
états `PRESSED`, `SELECTABLE`, `SELECTED`, `ACTIVE`, attribut `current`).

Le chemin des données est le suivant :

```text
WidgetSemantics + état du runtime
    → AccessibleElement
    → snapshot Windows avec coordonnées écran
    → interfaces COM UI Automation
    → client d’accessibilité
```

`Runtime::accessibility()` produit l’arbre sémantique. Un simple conteneur de
layout n’ajoute pas de niveau inutile. Les enfants sont rattachés au plus proche
parent sémantique ; un séparateur reste une feuille. L’état hors champ dépend du
rectangle réellement visible après découpage.

Dans l’autre sens, les opérations passent par `AccessibilityAction` : `Focus`,
`Invoke`, `SetValue`, `SelectText` ou `RevealText`. Elles empruntent les mécanismes habituels du runtime.
Focaliser un contrôle peut donc aussi faire défiler son conteneur pour le révéler.

## Threads, mémoire et notifications

Les appels d’un client peuvent arriver sur un autre thread. Ils lisent un
snapshot immuable partagé par `Arc`. Ils ne manipulent directement ni les widgets
ni les ressources graphiques. Les règles de threads constituent un point central
de [l’intégration UI Automation](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-threading).

Une mutation entre dans une file possédée, puis un message Windows transmet son
identifiant au thread UI. Aucun pointeur vers une variable de pile n’est envoyé.
La file accepte au plus 64 requêtes ; un appel attend au plus deux secondes.
Une requête non commencée est annulée après expiration. Une opération déjà en
cours peut néanmoins se terminer après ce délai.

Les fournisseurs COM utilisent des références comptées et un cache de références
faibles. Un client peut garder une référence après la disparition d’un élément :
les opérations signalent alors que l’élément n’est plus disponible. Les interfaces
annoncées par cet objet restent stables jusqu’à sa destruction. La fermeture de
la fenêtre invalide aussi les requêtes restantes et libère son snapshot.

Le snapshot est renouvelé lorsque changent la vue, le layout, le scroll, le texte,
la sélection, le focus ou la géométrie native. Le survol et le seul clignotement du caret ne
déclenchent pas sa reconstruction. Les notifications signalent notamment les
changements de structure, de focus, de valeur, de nom et d’état ; la position
(`BoundingRectangle`) n'est signalée que pour l'élément focalisé. Comme dans Chromium,
un défilement ne signale pas le déplacement de chaque élément : sur une liste de
morceaux, ces milliers d'événements par publication occupaient plus du tiers du thread
de l'interface dès qu'un client UI Automation écoutait. Les nœuds de l'ancien snapshot
sont indexés par clé, et le fournisseur n'est demandé que pour un nœud qui change ; le
snapshot indexe aussi ses nœuds pour les requêtes des clients. Pendant un défilement
(glissement de la molette, pouce tiré), l'arbre n'est republié qu'une fois le contenu
arrêté, sauf changement de focus : le reconstruire dix fois par seconde (~6 ms sur une
liste de morceaux) coûtait des images ; `IsOffscreen` n'est pas signalé non plus, les
clients le lisent à la demande.
TextPattern reçoit également les événements de changement de texte et de sélection.

La liste virtualisée expose seulement ses lignes matérialisées : le fournisseur
ne construit pas les 10 000 lignes pour les outils d’accessibilité.

## Où modifier le code

| Fichier | Responsabilité |
| --- | --- |
| `src/ui/core/element/mod.rs` | Contrat sémantique des widgets. |
| `src/ui/runtime/mod.rs` | Arbre accessible et actions communes. |
| `src/ui/components/text_input/mod.rs` | Remplacement de valeur et état de l’éditeur. |
| `src/platform/windows/accessibility/mod.rs` | Snapshots, commandes, géométrie et notifications. |
| `src/platform/windows/accessibility/provider/` | Objets COM et implémentation des interfaces. |
| `src/platform/windows/accessibility/text/` | TextPattern, plages, géométrie, recherche et navigation. |
| `src/platform/windows/accessibility/choices/` | Toggle, Selection, SelectionItem et notifications des choix. |
| `src/platform/windows/accessibility/expand/` | Expansion du Select, popup et identités des options par clé. |
| `src/platform/windows/accessibility/range_value/` | RangeValue et notifications du slider numérique. |
| `src/platform/windows/accessibility/text/tests.rs` | Tests des plages COM, Unicode et durées de vie. |
| `src/platform/windows/accessibility/ffi.rs` | Déclarations natives, GUID et types ABI. |
| `src/platform/windows/accessibility/tests.rs` | Tests des objets COM et des types natifs. |

## Validation et suite

Les 144 tests du projet passent, y compris les essais explicites GPU et logiciel.
Les nouveaux tests couvrent les chaînes Unicode, les valeurs, les erreurs, le
focus sans déplacement involontaire, l’activation, les accès concurrents, la durée
de vie des fournisseurs et les rectangles natifs. Clippy passe sans avertissement.
Neuf tests supplémentaires couvrent TextPattern et les nouvelles opérations de
l’éditeur : Unicode, déplacements, recherche, clipping, références COM, recalage,
sélection sans changement de valeur/focus, révélation et composition simulée.

Un client externe a découvert les contrôles de la démonstration, lu et modifié
le champ avec accents et emoji, activé Valider et relu le résultat. Ce contrôle
ne remplace pas une recette avec Narrateur ou NVDA. Cette vérification externe
précédait TextPattern : la nouvelle étape a été vérifiée par les tests des
interfaces COM et du runtime, sans nouvelle recette externe ni lecteur d’écran.
Les choix ont ensuite été ajoutés : découverte depuis un client UI Automation,
activation des cases, du switch et d’un radio, puis vérification visuelle du
clavier et des thèmes. Le champ du client indiquant le focus n’a pas suivi les
anneaux visibles pendant cette recette ; les annonces de focus restent donc à
vérifier avec un lecteur d’écran. Voir le [guide des choix](choices.md).
Le slider a ensuite été vérifié depuis UI Automation avec SetValue et à l’écran
par glissement/clavier, en GPU puis WARP. Voir le [guide numérique](slider.md).
Les [modales](modal.md) ont été découvertes depuis le même client : seuls le
dialogue actif et ses contrôles sont exposés. La confirmation remplace le
formulaire dans cet arbre ; le formulaire réapparaît à sa fermeture. IsDialog
et l’expiration d’un fournisseur retiré sont vérifiés par les tests COM. Les
annonces de dialogue avec un lecteur d’écran restent à tester.
Les [menus et popovers](popover.md) ajoutent leurs rôles et notifications. Le menu
a été découvert depuis le client accessible avec ses actions désactivées et son
séparateur ; les annonces lecteur d’écran restent à vérifier.
Les [info-bulles](tooltip.md) ont été vérifiées par Tab, délai natif et Échap :
le client découvre l’aide du bouton et le nœud ToolTip affiché. Les tests COM
contrôlent ses propriétés passives et la conservation du focus. Les annonces
avec un lecteur d’écran restent à vérifier.

```sh
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline -- --include-ignored
```

Pour vérifier les interfaces accessibles du socle :

```sh
cargo test --offline --lib
```

Restent à développer : `TextPattern2`, les attributs de texte supplémentaires,
la sélection des listes virtualisées, `ScrollPattern`, RangeValue des séparateurs
et `VirtualizedItemPattern`.
Les lignes non matérialisées ne sont pas encore accessibles individuellement.
Les essais réels avec lecteur d’écran, IME et texte bidirectionnel restent à faire.
Sous macOS, le pont NSAccessibility (`src/platform/macos/accessibility/`) expose
l’arbre à VoiceOver ; un champ multiligne y est annoncé comme `AXTextArea`, sans encore
les attributs de texte par plage (lignes, rectangles de caractères). L’intégration
native Linux n’est pas implémentée.



