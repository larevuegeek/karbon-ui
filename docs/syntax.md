# Écrire une interface avec une syntaxe simple

Importer l’API de composition :

```rust
use karbon_ui::ui::prelude::*;
```

Une vue reste une fonction Rust qui retourne un `Element`. Son arbre indique
la structure ; les méthodes indiquent les dimensions et la présentation.

```rust
column("profile", [
    Text::new("Mon profil").size(24.).bold().element("profile.title"),
    row("profile.actions", [
        button("profile.save", "Enregistrer", Msg::Save),
        button("profile.cancel", "Annuler", Msg::Cancel),
    ]).gap(12.),
]).padding(24).height(FILL)
```

Les fonctions `row`, `column`, `grid`, `layers`, `scroll`, `split`, `modal`
et `popover` construisent les éléments du moteur. Elles acceptent un tableau,
un `Vec` ou un itérateur d’enfants. Il y a une seule structure `Element` à la
sortie ; le layout, les événements et le rendu restent ceux du socle commun.

L’exemple complet et compilé est [`examples/simple_ui.rs`](../examples/simple_ui.rs) :

```powershell
cargo run --offline --example simple_ui --target-dir target/syntax-check
```

## Dimensions et espacements

```rust
element.width(240)                 // 240 unités logiques
element.width(percent(50.))        // 50 % du parent
element.width(FILL)                // une part de l'espace restant
element.width(fraction(2.))        // deux parts
element.width(AUTO)                // taille naturelle
element.height(80)
element.padding(16)                // quatre côtés
element.padding([12, 24])          // vertical, horizontal
element.padding([8, 16, 12, 20])    // haut, droite, bas, gauche, comme CSS
element.gap(12.)
element.align(Align::Center)
```

`Length::Px`, `Length::Fill` et `Insets` restent acceptés. Les dimensions
invalides sont refusées lors de la validation du layout.

## Styles nommés

Définir les valeurs partagées dans le `styles.rs` de l’application :

```rust
const PANEL: ViewStyle = ViewStyle::new()
    .width(280.)
    .padding(20.)
    .gap(12.)
    .background(Color::hex(0x131715))
    .radius(16.);

const COMPACT: ViewStyle = ViewStyle::new().padding(12.).gap(6.);
```

Puis les appliquer à la vue :

```rust
column("panel", [text("panel.title", "Bibliothèque")])
    .style(PANEL)
    .style(COMPACT)
    .width(320)
```

Seules les propriétés renseignées sont appliquées, de gauche à droite.
Ici, la largeur finale est 320, le padding 12 et le gap 6 ; la couleur et
l’arrondi de `PANEL` sont conservés. Une propriété omise conserve sa valeur.
`ViewStyle` est une valeur `Copy` appliquée à la construction de l’élément.

Dans une recette constante, `.width(280.)` et `.height(80.)` définissent des
dimensions fixes ; `.fill_width()` et `.fill_height()` occupent l’espace
restant. `Insets::edges(haut, droite, bas, gauche)` permet un padding asymétrique
constant avec `.insets(...)`.

Le style du conteneur concerne sa mise en page et son fond. Les recettes
`ButtonStyle`, `TextStyle`, `InputStyle`, `ChoiceStyle`, etc. concernent le
dessin du widget et ses états. Elles se configurent avant `.element(...)` :

```rust
Button::new("Enregistrer", Msg::Save)
    .variant(ButtonVariant::Outline)
    .style(button_style.clone())
    .element("save")
    .width(160)
```

`SurfaceStyle` et `.styled(...)` restent disponibles pour partager des surfaces
avec bordure et couleurs sémantiques. Voir [les recettes de composants](styles.md).
Cette API ne prévoit pas encore d’héritage de styles ni de sélecteurs CSS.

## Texte et polices

```rust
Text::new("Bonjour David")
    .font("Inter")
    .weight(700)
    .size(24.)
    .color(Color::hex(0x22c55e))
    .element("welcome")
```

Le widget mesure et dessine avec la même famille et la même graisse, avec
retour à la ligne dans sa largeur disponible. `.bold()` fixe la graisse à
700. L’ordre des appels s’applique également à la typographie.

Une police personnalisée doit être déclarée par `Application::fonts()` avec
une famille et des octets embarqués dans un `FontAsset`. La chaîne passée à
`.font()` est le nom interne de la famille. Les tests conservent leurs polices
dans `tests/fixtures/fonts/`, avec leurs licences. Les polices système peuvent
également être demandées. Les autres widgets gardent leurs recettes de
typographie actuelles ; le choix de famille ajouté ici concerne `Text`.

## Messages et mise à jour

Chaque application déclare le type des messages émis par sa vue :

```rust
#[derive(Clone)]
enum Msg {
    Save,
    Cancel,
    NameChanged(String),
    Volume(f64),
    VolumeCommitted(f64),
    Notifications(bool),
}

impl Application for App {
    type Message = Msg;
    fn view(&self, _: &ViewContext) -> Element { /* ... */ }
    fn update(&mut self, message: Msg) -> bool {
        match message {
            Msg::Save => self.save(),
            Msg::Cancel => self.cancel(),
            Msg::NameChanged(name) => self.name = name,
            Msg::Volume(value) => self.volume = value,
            Msg::VolumeCommitted(value) => self.persist_volume(value),
            Msg::Notifications(enabled) => self.notifications = enabled,
        }
        true // reconstruire la vue
    }
}
```

`update` renvoie `true` pour reconstruire la vue. Il n’y a plus de méthode par
type d’événement : texte, valeurs numériques, sélection, dépôt et menu
contextuel arrivent tous dans `update`.

Deux formes de gestionnaires existent :

- **une valeur** pour les actions simples ; elle est clonée à chaque activation
  (`M: Clone + 'static`) ;
- **une fonction** `Fn(T) -> M` pour les widgets qui transmettent une valeur.
  Une variante tuple est déjà une fonction : `Msg::NameChanged` convient pour
  `NameChanged(String)`, et une fermeture reste possible (`|v| Msg::Set(3, v)`).

Rust ne permet pas qu’un même paramètre accepte à la fois une valeur et une
fermeture (cohérence des traits) : chaque widget choisit donc la forme naturelle.

| Constructeur | Gestionnaire |
| --- | --- |
| `button(id, label, Msg::Save)`, `Button::new`, `widgets::button` | message |
| `menu_item(id, label, msg)`, `MenuItem::new(label, msg)` | message |
| `RadioButton::new(label, selected, Msg::Size(2))` | message, seulement si non sélectionné |
| `Modal::new(..).on_dismiss(msg)`, `Popover::new(label, anchor, msg)` | message |
| `TextInput::new(value, Msg::NameChanged)`, `input(id, value, f)`, `textarea(id, value, f)` | `Fn(String) -> M` |
| `TextInput::on_submit(msg)` | message (Entrée, champ monoligne) |
| `Slider::new(value, Msg::Volume)` | `Fn(f64) -> M`, valeur provisoire |
| `Slider::on_commit(Msg::VolumeCommitted)` | `Fn(f64) -> M`, valeur finale |
| `Select::new(selected, options, Msg::Source)` | `Fn(u64) -> M`, clé choisie |
| `Checkbox::new(label, checked, Msg::Notifications)`, `Switch::new` | `Fn(bool) -> M`, **nouvelle** valeur |
| `DropTarget::new(group, key, Msg::Reorder)` | `Fn(DropEvent) -> M` |
| `element.context_menu(Msg::ContextMenu)` | `Fn(ContextMenuRequest) -> M` |

Une case à cocher indéterminée (`CheckState::Mixed`) devient cochée : elle
transmet `true`.

```rust
button("save", "Enregistrer", Msg::Save)
TextInput::new(self.name.as_str(), Msg::NameChanged).element("name")
Checkbox::new("Notifications", self.notifications, Msg::Notifications).element("notify")
```

Un widget personnalisé activable retourne une `Action` :

```rust
struct Tile {
    open: Action,
}
impl Widget for Tile {
    // measure, paint...
    fn action(&self) -> Option<Action> {
        Some(self.open.clone())
    }
}
let tile = Tile { open: Action::new(Msg::Save) };
```

Tous les gestionnaires d’une vue doivent produire `Application::Message`. Un
autre type est une erreur de programmation : le runtime s’arrête avec un message
indiquant le type émis et le type attendu. Deux fenêtres peuvent toutefois
utiliser des types de message différents.

## Identité et composants réutilisables

Une clé stable permet au runtime de conserver le focus, l’édition et le scroll
lorsque la vue est reconstruite ou réordonnée. Les noms doivent être uniques
dans l’arbre courant ; les doublons sont refusés à la validation.

Pour un petit écran, un nom comme `"search"` suffit. Pour un composant utilisé
plusieurs fois, transmettre une portée :

```rust
fn profile(scope: ElementId, name: &str) -> Element {
    column(scope, [
        text(scope.child("label"), name),
        button(scope.child("save"), "Enregistrer", Msg::Save),
    ])
}

row("profiles", [
    profile(ElementId::named("left-profile"), "David"),
    profile(ElementId::named("right-profile"), "Invité"),
])
```

Les noms sont convertis en clés numériques stables sans stocker leur chaîne.
`scope.item(track.id)` utilise directement une clé métier numérique. Choisir
une clé indépendante de la position courante dans la liste.

Tous les widgets qui implémentent `Widget` disposent de `.element(clé)` grâce
au trait `WidgetExt` du prelude. Leur configuration reste accessible en Rust.
`container.painter(widget)` permet de peindre un conteneur avant ses enfants.

## Contenu conditionnel

```rust
column("page", [header()])
    .child_if(self.sidebar_open, || sidebar())
```

La fermeture n’est appelée que si la condition est vraie. Un `if`, un `match`
ou une boucle Rust conviennent aussi pour préparer les enfants d’une vue.

## Exemple dans le projet

Lire [`examples/simple_ui.rs`](../examples/simple_ui.rs), puis le
[plan du moteur UI](../src/ui/README.md). Les tests vérifient les champs
réordonnés, les styles composés, l’acheminement des messages et les widgets personnalisés.
