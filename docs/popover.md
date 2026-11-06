# Popovers et menus contextuels

Un `Popover` est une couche temporaire ancrée à un élément ou à un point de la
fenêtre. Il réutilise les composants et le rendu existants, sans seconde fenêtre
native et sans voile assombri. Le mode menu propose des actions `MenuItem`.

## Ouvrir un panneau depuis un bouton

Importer `ui::popover::{Anchor, Popover}`, puis conserver un booléen applicatif
`settings_open`. Dans `Application::view` :

```rust
let page = Element::column(1, vec![
    widgets::button(2, "Réglages", Msg::OpenSettings),
]);
let mut overlays = Vec::new();
if self.settings_open {
    let content = Element::column(11, vec![
        widgets::text(12, "Réglages rapides"),
        Element::custom(13, Slider::new(self.volume, Msg::Volume).label("Volume")),
        widgets::button(14, "Terminer", Msg::CloseSettings),
    ]).gap(10.);
    overlays.push(Element::popover(
        10,
        Popover::new("Réglages rapides", Anchor::Element(ElementId(2)), Msg::CloseSettings)
            .initial_focus(13),
        content,
    ).width(Length::Px(300.)));
}
Element::layers(100, page, overlays)
```

`Msg` est le type `Message` de l’application. Le dernier argument de
`Popover::new` est le message envoyé quand le panneau doit se fermer. Dans
`update`, l’application ouvre ou ferme le panneau en changeant son booléen
et en renvoyant `true`. Le slider conserve son fonctionnement habituel : sa
fonction `Msg::Volume` reçoit chaque valeur. Les champs, cases, switches et Select restent utilisables.

Le titre de `Popover::new` est le nom accessible ; ajouter un `Text` pour un
titre visible. Le panneau utilise par défaut une largeur de 280 unités, un
padding de 8 et un rayon de 10. `.width`, `.height`, `.padding` et `.surface`
personnalisent l’élément. Les composants intérieurs gardent leurs propres styles.

## Construire un menu d’actions

```rust
let content = Element::column(21, vec![
    Element::custom(22, MenuItem::new("Lire", Msg::Play).shortcut("Entrée")),
    Element::custom(23, MenuItem::new("Option indisponible", Msg::Play).disabled()),
    Element::custom(24, MenuSeparator),
    Element::custom(25, MenuItem::new("Ajouter à la playlist", Msg::AddToPlaylist)),
]);
let menu = Element::popover(
    20,
    Popover::new("Actions du titre", anchor, Msg::CloseMenu).menu(),
    content,
);
```

`MenuItem` et `MenuSeparator` sont exportés par `ui::widgets`. Une action de
menu envoie son message à `Application::update`, comme un bouton. Après son
activation, le moteur envoie aussi `Msg::CloseMenu`, une seule fois. Ce second message ferme
uniquement le menu : l’action principale peut ouvrir une modale ou modifier
les données. L’application doit retirer le menu de sa prochaine vue.

`.shortcut(...)` affiche une indication ; il n’enregistre pas un raccourci.
`.color(Color)` personnalise le texte d’une action. Les lignes font 36 unités,
avec les couleurs de survol, de pression et de focus du thème.
Les menus de cette première étape contiennent des actions et du contenu non
focalisable. Pour un formulaire, utiliser un Popover sans `.menu()`.

## Raccorder le clic droit

Ajouter une liaison au contrôle ou au conteneur concerné :

```rust
widgets::button(2, "Un titre", Msg::Play).context_menu(Msg::TrackMenu)
```

`context_menu` reçoit une fonction `Fn(ContextMenuRequest) -> Msg` ; une
variante `TrackMenu(ContextMenuRequest)` suffit. Puis traiter la demande dans
`update` :

```rust
Msg::TrackMenu(request) => {
    self.menu_anchor = Some(Anchor::Point(request.position));
    self.menu_target = Some(request.target);
}
```

La vue construit ensuite le menu avec cette ancre. `request.target` identifie
l’élément portant la liaison, et `position` est un point en unités logiques de
fenêtre. Le moteur choisit la liaison la plus proche, puis remonte vers les
conteneurs. Une liaison sur un widget désactivé n’est pas déclenchée.

`request.keyboard` distingue le clavier du clic droit. Shift+F10 ou la touche
Menu demandent le menu près du contrôle focalisé. Dans une `VirtualList`, une
liaison sur la ligne active utilise cette ligne et sa géométrie visible, sans
construire toutes les lignes.

Le backend Windows transforme `WM_CONTEXTMENU` en cet événement commun. Les
coordonnées écran sont converties en coordonnées locales ; la valeur native
`(-1, -1)` désigne la demande au clavier. Voir le
[contrat Microsoft de WM_CONTEXTMENU](https://learn.microsoft.com/en-us/windows/win32/menurc/wm-contextmenu).

## Placement, listes de choix et templates

`Popover::placement(PopoverPlacement { side, align, offset, match_anchor, cross_offset })`
choisit le côté de l’ancre (`Side::Top/Right/Bottom/Left`), l’alignement le long de ce
côté (`SideAlign::Start/Center/End`), la distance, avec `match_anchor` une largeur au
moins égale à celle de l’ancre, et avec `cross_offset` un décalage le long du côté
(vers la droite ou le bas ; `cross-offset` dans un template). Le panneau bascule du côté opposé si le côté choisi
manque de place et que l’autre en a plus, puis reste dans la fenêtre. Défaut : dessous,
aligné à gauche, 4 unités (comportement d’origine).

`Popover::listbox()` (`PopoverKind::Listbox`) se comporte comme un menu dont les
entrées ont le rôle `ListItem` (options d’une liste, rôle List exposé).
`Popover::combobox()` (`PopoverKind::Combobox`) est une liste de suggestions **non
modale** ancrée à un champ texte : elle ne prend pas le focus, n’entre pas dans la pile
de focus des couches, laisse la page interactive (le test de hit la consulte d’abord,
puis la couche du dessous) ; Échap, Tab, un clic hors d’elle et de son champ, la
molette hors d’elle ou la perte du focus de la fenêtre la ferment ; choisir une option
envoie son message puis la fermeture. Le champ pilote la surbrillance par
`TextInput::on_key` (`onkeydown`). Dans un
template, `<k-popover menu|listbox placement=… offset=… match-width>` et l’attribut
`role="menuitem|option"` sur des boutons ou des `div onclick` construisent ces menus
avec un contenu libre (icônes SVG, textes, séparateurs) : voir
[html-syntax.md §11](html-syntax.md#11-menus-listes-de-choix-et-infobulles).

## Placement, focus et fermeture

Placer le popover en dernier dans `Element::layers`, après les modales
éventuelles. Un seul popover général peut être déclaré à la fois. Son ancre
élément doit appartenir à la page ou à la modale actuellement accessible.
Une ancre située derrière une autre modale ne permet pas de la contourner.

`Anchor::Element` suit le rectangle visible du contrôle après layout.
`Anchor::Point` accepte un point fini, puis limite le panneau à la fenêtre.
Le placement préfère le dessous ; il passe au-dessus si l’espace disponible
est meilleur. La marge au bord est de 8 unités. Le panneau dépasse le clip de
son ancien parent et reste limité à la fenêtre.

Le moteur conserve le focus précédent et choisit `initial_focus`, le premier
contrôle actif ou, à défaut, le panneau lui-même. Pendant l’ouverture, les
interactions et le snapshot accessible sont limités à cette couche temporaire.
Cela évite les activations derrière un menu ouvert. Un Select à l’intérieur
peut encore se superposer au panneau.

| Interaction | Comportement |
| --- | --- |
| Échap | Fermer d’abord un Select ou annuler l’interaction en cours, puis demander la fermeture du popover. |
| Clic extérieur | Absorber l’appui et le relâchement ; demander la fermeture au relâchement extérieur. |
| Tab / Shift+Tab | Demander la fermeture, restaurer le contrôle précédent, puis avancer/reculer dans le parcours de la couche révélée. |
| Perte de focus de la fenêtre | Demander la fermeture. |
| Molette à l’extérieur | Fermer sans faire défiler la page pendant cet événement. |
| Haut/Bas dans un menu | Parcourir les actions actives, en bouclant aux extrémités. |
| Home / End dans un menu | Première / dernière action active. |
| Entrée / Espace dans un menu | Activer l’action puis demander la fermeture. |
| Mouvement dans un menu | Focaliser l’action survolée ; un événement à position inchangée ne remplace pas le choix clavier. |

Une ancre retirée, désactivée, entièrement cachée ou rendue inaccessible par
une modale provoque le retrait du panneau et l’envoi de son message de fermeture.
`update` doit mettre à jour l’état applicatif pour conserver cette fermeture.
Une reconstruction avec les mêmes identifiants et la même ancre peut maintenir
le panneau ouvert. Le redimensionnement recalcule son placement.

Pour un menu long, fournir un `Element::scroll` vertical avec des identifiants
explicites, et une hauteur au panneau. Les flèches et End révèlent l’action
focalisée dans ce viewport. Le contenu trop grand sans scroll sera découpé.
Les menus en cascade et la recherche par lettres restent à développer.

Dans une fenêtre layered transparente, les pixels d’alpha nul peuvent laisser
passer les clics au bureau avant leur réception par le moteur. L’absorption du
clic extérieur s’applique aux événements effectivement reçus par la fenêtre.

## Windows UI Automation

Le panneau simple est exposé comme un conteneur nommé. Le menu expose le rôle
`Menu`, ses actions le rôle `MenuItem` et `InvokePattern`, y compris la
disponibilité du pattern sur une action désactivée dont l’invocation est refusée.
Le séparateur est distinct. Les notifications d’ouverture et de fermeture
suivent l’apparition du menu, sans être répétées au survol. Voir les
[règles Microsoft pour les menus](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportmenucontroltype)
et les [actions de menu](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportmenuitemcontroltype).
Les annonces effectives avec Narrateur/NVDA restent à vérifier.

## Code et vérifications

```sh
cargo test --offline --lib popover_tests
```

| Fichier | Rôle |
| --- | --- |
| `src/ui/overlays/popover/mod.rs` | API Popover/Anchor, demandes contextuelles et dessin des actions. |
| `src/ui/overlays/layers.rs` | Placement, validation des couches et pile commune de focus. |
| `src/ui/runtime/mod.rs` | Routage, fermeture, navigation et appels à l’application. |
| `src/ui/layout/mod.rs` | Positionner le panneau après la page et les modales. |
| `src/platform/windows/events/pointer.rs` | Convertir WM_CONTEXTMENU. |
| `src/platform/windows/accessibility/` | Rôles, Invoke et événements de menus. |

Quatorze tests communs couvrent contrôles réutilisés, focus, clics, clavier,
menus longs, restauration, virtualisation, ancres invalidées et validation.
Deux tests natifs couvrent le cycle des notifications et les coordonnées du menu, dont le signal clavier en 32/64 bits. La suite complète
compte 144 tests, y compris GPU et WARP ; formatage et Clippy passent.
Le menu a été observé à l’écran et découvert par le client accessible. La démo
est restée ouverte car elle était en cours d’utilisation ; la recette visuelle
complète des popovers, du clic droit et de Shift+F10 reste à terminer.




