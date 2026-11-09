# Le kit de composants

Des composants stylés, prêts à l'emploi, qui s'écrivent comme du HTML. Ils sont bâtis sur
les contrôles natifs du moteur : clavier, focus et accessibilité viennent avec eux.
La galerie les montre tous :

```sh
cargo run --release --example kit_gallery
```

```rust,ignore
use karbon_ui::ui::kit::prelude::*;

ui! {
    <Page padding="xl" gap="lg">
        <Title>Bibliothèque</Title>
        <Card title="Nouvel album" subtitle="Quelques secondes suffisent">
            <Input label="Titre" value={&self.title} oninput={Msg::Title} icon="music" />
            <Button slot="footer" ghost>Annuler</Button>
            <Button slot="footer" primary icon="plus" onclick={Msg::Add}>Ajouter</Button>
        </Card>
    </Page>
}
```

`kit::prelude` contient tout le prélude habituel, avec les composants du kit à la place
des widgets de même nom (`Button`, `Select`, `Text`…). Un attribut inconnu est une erreur
de compilation. Un texte contenant une apostrophe s'écrit dans un fichier `.ui`, pas dans
`ui!`, dont le contenu doit rester une suite de jetons Rust.

## Attributs communs

| Attribut | Valeurs |
| --- | --- |
| Teinte | `primary`, `success`, `warning`, `danger`, `info`, `variant={Variant::Danger}`, `color="#e11d48"` |
| Apparence (boutons) | `soft`, `outline`, `faded`, `ghost`, `shadow`, `link`, `glass` ; forme `pill` |
| Taille | `sm`, `lg` |
| Distances | `gap="md"`, `padding="xl"` : `none`, `xs`, `sm`, `md`, `lg`, `xl`, `2xl`, ou un nombre |
| Événements | voir la section Événements |
| Toujours permis | `id`, `class`, `style="…"`, `style:width={…}`, `hidden`, `slot` |

## Composants

Ils sont rangés par type, dans autant de modules (`kit::typography`, `kit::forms`,
`kit::buttons`, `kit::animation`, `kit::pagination`…), tous repris par `kit::prelude`.

| Type | Composants |
| --- | --- |
| Mise en page | `Page`, `Container`, `Stack`, `Row`, `Grid`, `Spacer`, `Divider` |
| Texte | `Title` (`display`, `h2`, `h3`, `h4`, `gradient`, `center`), `Text` (`muted`, `sm`, `lg`, `bold`, `center`, `mono`), `Code`, `Kbd`, `Link` |
| Boutons | `Button` (`icon`, `icon_right`, `loading`, `disabled`, `full`), `IconButton` (`icon`, `label`) |
| Formulaires | `Field`, `Input` (`label`, `hint`, `error`, `icon`, `outline`, `password`, `search`, `required`), `Textarea`, `Select`, `Checkbox`, `Switch`, `RadioGroup`, `Slider` |
| Affichage | `Card`, `Badge` (`solid`, `outline`, `dot`, `onclose`), `Avatar`, `Stat`, `Table`, `TableRow`, `TableCell`, `List`, `ListItem`, `EmptyState`, `Accordion`, `Icon` |
| Retours d'état | `Alert`, `Progress`, `Spinner`, `Skeleton`, `Toasts`, `Toast` |
| Navigation | `Tabs` (`underlined`), `Breadcrumb`, `Steps`, `Navbar` (`glass`), `Sidebar`, `NavItem` |
| Pagination | `Pagination` |
| Carrousel | `Carousel` (`per_view`, `loop_around`, `no_arrows`, `no_dots`) |
| Animation | `Animate` : entrées `fade`, `up`, `down`, `left`, `right`, `zoom`, `bounce`, `shake` ; boucles `pulse`, `float`, `spin` ; `delay`, `duration`, `replay` |
| Effets | `Glass` (`strong`, `pill`, `tint`), `Aurora` (`full`, `flat`, `animated`), `Title gradient`, `Card glass`, `Dialog glass` |
| Couches | `Dialog`, `Dropdown`, `MenuItem`, `Tooltip` |

Icônes : `ICON_NAMES` donne la liste ; `<Icon name="plus" size={20} />`, à la couleur du
texte. L'état reste dans l'application, comme pour tout contrôle : `<Tabs value={self.tab}
onchange={Msg::Tab} />`, `<Dialog>` dans un `{#if}` ou avec `open={self.visible}`.

Slots : `Card` (`actions`, `footer`), `Dialog` (`footer`), `Navbar` (`actions`), `ListItem`
(`leading`, `trailing`).

## Effets

```html
<Aurora full flat>
    <Stack padding="2xl" gap="lg">
        <Title display gradient>Du verre discret</Title>
        <Glass padding="lg">Contenu</Glass>
        <Button primary shadow pill>Commencer</Button>
    </Stack>
</Aurora>
```

Le verre floute et sature ce qui est peint dessous : il a besoin d'un fond riche (`Aurora`,
image, dégradé). `Aurora` est une lumière très douce, à poser sur toute la fenêtre (`full
flat`) pour qu'aucun bord ne se voie ; `animated` la fait dériver, au prix d'une image
redessinée en continu.

## Événements

Tout élément et tout composant accepte, sur sa racine :

| Attribut | Reçoit |
| --- | --- |
| `onhover={Msg::Hover}` | `bool` : le pointeur entre, puis sort |
| `onpress={Msg::Press}` | `bool` : l'appui commence, puis finit |
| `onfocus={Msg::Focus}` | `bool` : le focus entre, puis sort |
| `ondblclick={Msg::Open}` | rien |

Ces événements comptent aussi ce qui arrive aux descendants : survoler un bouton, c'est
survoler sa carte. Chaque contrôle garde les siens : `onclick`, `oninput`, `onchange`,
`onenter`, `onclose`, `ontoggle`. Les valeurs s'écrivent `Msg::Volume` ou
`|v: f64| Msg::Set(3, v)`. En Rust : `Element::on_hover`, `on_press`, `on_focus`,
`on_double_click`.

## Personnaliser

Du plus simple au plus fin :

1. **Le thème de toute l'application**, une seule fois au lancement (les fenêtres ouvertes
   suivent à l'image suivante) :

   ```rust,ignore
   set_theme(KitTheme::default().primary(Color::hex(0xe11d48)).radius(12.));
   ```

   `KitTheme` expose aussi les palettes complètes (`light`, `dark`).
2. **Un composant** : `<Button color="#e11d48">`, ou `style:width={120.}`.
3. **Vos classes**, posées sur la racine du composant et appliquées après les siennes :
   `<Button class="promo">`.
4. **Les variables d'une zone** : `.promo { --k-primary: #e11d48; }` sur un conteneur. Elle
   vaut pour les couleurs (fonds unis, textes, bordures) lues à l'exécution. Les teintes
   passées par attribut, les rayons, les ombres et les dégradés viennent du thème.
5. **Composer** : un `Component` à vous qui assemble ceux du kit.

Variables : `--k-bg`, `--k-surface`, `--k-subtle`, `--k-border`, `--k-text`, `--k-muted`,
`--k-primary`, `--k-success`, `--k-warning`, `--k-danger`, `--k-info`, `--k-radius`,
`--k-radius-sm`, `--k-radius-lg`.

Le thème clair ou sombre vient de `Application::color_preferences`. Une vue qui utilise le
kit donne aussi ses couleurs au thème du moteur : cases, pistes et listes ouvertes suivent.

## Organisation

`src/ui/kit/` : un fichier par famille, `css/` pour les styles, `theme.rs` pour les
palettes, `events.rs` pour les attributs d'événement. Pour ajouter un composant : une
structure à `#[derive(Default)]`, `impl<R> Component<R>`, un gabarit `kit_ui!` qui réutilise
une balise native, ses classes dans `css/`, l'export dans `mod.rs` et son `prelude`.

## Limites

- `Carousel` et les boucles d'`Animate` ne peuvent pas contenir de champ, de liste ni de
  zone de défilement : le moteur refuse ces contrôles sous un déplacement durable.
- Le défilement automatique d'un carrousel reste à la charge de l'application (`value`).
- Pas encore de `Drawer` : les dialogues du moteur se placent au centre ou en haut.
- `justify-content: space-around` et `space-evenly` ne sont pas pris en charge par le moteur.
- `Select` ne gère que des valeurs `u64`, comme la balise native.
