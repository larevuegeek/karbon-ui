# Modales : contenu, couches et focus

Une modale est un dialogue dessiné au-dessus de la page, dans la même fenêtre
native. Ses enfants sont les composants habituels : `Text`, `Button`,
`TextInput`, `Select`, lignes, grilles et zones de défilement.

L’application conserve les données et décide quels dialogues sont ouverts.
Le moteur centre leur contenu, peint les fonds assombris et réserve les
interactions à celui qui se trouve au sommet.

## Composer la page et ses dialogues

Importer `ui::modal::Modal`, puis dans `Application::view` :

```rust
let page = Element::column(1, vec![
    widgets::button(2, "Ouvrir", Msg::OpenDialog),
]);
let mut dialogs = Vec::new();
if self.dialog_open {
    let content = Element::column(11, vec![
        Element::custom(12, Text::new("Créer une playlist").size(24.).bold()),
        Element::custom(13, TextInput::new(self.name.as_str(), Msg::Name)
            .label("Nom de la playlist")),
        widgets::button(14, "Annuler", Msg::CloseDialog),
        widgets::button(15, "Créer", Msg::CreatePlaylist),
    ]).gap(16.);
    dialogs.push(Element::modal(
        10,
        Modal::new("Créer une playlist")
            .on_dismiss(Msg::CloseDialog)
            .initial_focus(13),
        content,
    ));
}
Element::layers(100, page, dialogs)
```

`Msg` est le type `Message` de l’application ; les boutons et `on_dismiss`
reçoivent un message cloné à chaque activation. Tous les
identifiants d’éléments restent uniques dans l’arbre, y compris les dialogues
masqués par une autre modale. Garder les mêmes identifiants lors d’une mise
à jour de texte ou de thème permet de conserver le focus.

`Element::layers` doit être la racine. Son premier enfant est la page et ses
enfants suivants sont les modales, dans l’ordre du fond vers le premier plan.
La validation accepte jusqu’à huit dialogues. Les modales se déclarent
directement dans cette liste, même si une action d’un dialogue en ouvre un autre.
Un [popover](popover.md) facultatif peut compléter la liste en dernière position.

## Ouvrir et fermer depuis l’application

```rust
fn update(&mut self, message: Msg) -> bool {
    match message {
        Msg::OpenDialog => self.dialog_open = true,
        Msg::CloseDialog => self.dialog_open = false,
        Msg::Name(name) => self.name = name,
        Msg::CreatePlaylist => {
            // Valider les données et appliquer l’action métier ici.
            self.dialog_open = false;
        }
    }
    true
}
```

La fermeture consiste à ne plus retourner cette modale dans la prochaine vue.
`on_dismiss` envoie son message à `Application::update` ; il ne retire pas le
dialogue à la place de l’application. Pour refuser une fermeture ou afficher
une erreur, conserver le dialogue et mettre à jour ses enfants. La saisie se
conserve normalement en traitant le message `on_change` du champ dans `update`.

Le titre fourni à `Modal::new` est le nom accessible, obligatoire et non vide.
Le titre visible est un `Text` à inclure dans le contenu.

## Options et aspect

| Option | Valeur par défaut / effet |
| --- | --- |
| `on_dismiss(message)` | Aucun message par défaut ; notification de fermeture facultative. |
| `close_on_escape(bool)` | `true` ; Échap peut demander la fermeture. |
| `close_on_backdrop(bool)` | `false` ; autoriser la fermeture au clic sur le fond. |
| `initial_focus(id)` | Premier contrôle actif ; l’ID demandé doit appartenir au dialogue. |
| `backdrop(Color)` | Noir avec alpha 150/255 ; personnalisable indépendamment du panneau. |

Le panneau utilise le thème, une largeur de 480 unités, des coins de 14 unités
et un padding de 24 unités. Les méthodes habituelles de l’élément permettent
de régler `.width(...)`, `.height(...)`, `.padding(...)` et `.surface(...)`.
La hauteur automatique dépend du contenu. Le dialogue est centré dans la zone
de contenu de la fenêtre, avec une marge de 16 unités, et réduit à l’espace
disponible. Le texte est remesuré quand sa largeur diminue.

Un formulaire plus haut que la fenêtre doit placer son contenu dans un
`Element::scroll` avec une hauteur appropriée. La réduction du panneau ne crée
pas de scroll implicite ; le contenu dépassant son clip est découpé.
La barre de titre et les boutons de la fenêtre restent utilisables.

Dans un overlay Windows transparent, conserver de l’alpha sur les zones qui
doivent intercepter les clics. Un pixel d’alpha nul d’une fenêtre layered peut
laisser passer le clic au bureau avant que notre moteur le reçoive. Le mode
global `--click-through` reste destiné aux overlays passifs.

## Focus et ordre des interactions

À l’ouverture, le moteur mémorise le contrôle focalisé puis choisit le focus
initial du dialogue. Tab et Shift+Tab bouclent dans celui-ci. Les radios gardent
leur navigation habituelle. Sans contrôle actif, le dialogue lui-même sert
de point de focus. Un clic sur le fond ne déplace pas le focus derrière.

À la fermeture, le contrôle précédent est restauré s’il existe encore et
reste actif dans la couche révélée. Sinon, le moteur choisit un contrôle vivant
de cette couche. Une confirmation peut ainsi rendre le focus au bouton de son
formulaire parent, puis ce formulaire à son déclencheur dans la page.

Échap ferme d’abord un Select ouvert ou annule une interaction en cours.
Une composition IME reçoit Échap avant la modale. Sans interaction prioritaire,
la touche demande la fermeture du dialogue. Maintenir Échap ne traverse pas
plusieurs couches et ne ferme pas la fenêtre hôte par répétition.

Les clics sur le fond sont capturés jusqu’au relâchement et ne déclenchent pas
un bouton derrière. Avec `close_on_backdrop(true)`, un appui et un relâchement
à l’extérieur demandent la fermeture. Le scroll ne se propage pas à la page.
Une ouverture ou fermeture programmatique termine les interactions de la couche
précédente. Une perte de focus natif laisse le dialogue ouvert.

## Du Rust aux pixels et à Windows

```text
Application::view → Element::layers(page, dialogues)
    → validation et pile de focus dans layers.rs
    → mesure et centrage dans layout.rs
    → événements limités au dialogue actif dans runtime.rs
    → page, fond assombri, dialogue, puis Select éventuel
    → scène commune → Direct2D / DirectWrite
```

La page et les dialogues inférieurs restent en mémoire et sont peints derrière
le voile. Ce premier socle ne suspend pas un travail métier en arrière-plan et
n’utilise pas de texture de cache pour toute la page. Un survol ne reconstruit
pas la vue ni le layout ; l’ouverture change la composition et les remesure.

Le snapshot accessible expose seulement le dialogue du dessus et ses enfants,
en plus des contrôles de fenêtre. Les commandes visant un élément derrière sont
refusées. Le backend Windows expose le panneau avec `IsDialog = true` ; cette
propriété indique aux clients qu’il s’agit d’un dialogue. Voir la
[définition Microsoft de IsDialog](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-automation-element-propids).
Il ne fournit pas de `WindowPattern` pour ce panneau dessiné dans la fenêtre.
Les annonces de titre et de focus avec Narrateur/NVDA restent à vérifier.

## Vérifications

```sh
cargo test --offline --lib modal_tests
```

Treize tests communs couvrent placement, réduction et texte replié, focus, pile,
clics, annulations, maintien d’Échap, scroll, accessibilité et validation.
Un test natif couvre `IsDialog` et l’expiration du fournisseur. La suite complète
compte 144 tests, y compris GPU et WARP ; formatage et Clippy passent.

Le code est dans `src/ui/overlays/modal.rs`, avec l’intégration dans `ui/core/element/`,
`ui/layout/` et `ui/runtime/`.
Les [popovers et menus contextuels](popover.md) réutilisent désormais la pile
de focus commune dans `ui/overlays/layers.rs`. Les animations, les dialogues déplaçables,
les drawers et les menus en cascade restent à développer.



