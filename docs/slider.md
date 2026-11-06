# Slider : volume, position et validation

`Slider` règle une valeur numérique bornée. Il est dessiné par le moteur et
réutilise le layout, le focus, les thèmes et les rendus GPU/logiciel existants.
Il peut être horizontal ou vertical. Le libellé accessible se règle avec `label` ;
ajouter un widget `Text` séparé pour un titre ou une valeur visible.

## Construction

Dans `Application::view` :

```rust
Element::custom(
    143,
    Slider::new(self.volume, Msg::Volume)
        .range(0., 100.)
        .step(1.)
        .label("Volume")
        .on_commit(Msg::VolumeCommitted),
)
```

`Msg::Volume` et `Msg::VolumeCommitted` sont des variantes `(f64)` du type
`Message` de l’application : les deux gestionnaires sont des fonctions
`Fn(f64) -> Msg`, et une fermeture convient aussi.
Le constructeur utilise par défaut la plage 0–100 et le pas 1. `on_commit` est
facultatif. Les valeurs sont des `f64`, ce qui permet aussi `.range(0., 1.).step(0.01)`.
Les dimensions et espacements extérieurs se règlent sur `Element`.

La piste peut être personnalisée sans remplacer son comportement :

```rust
SliderStyle { track_thickness: Some(8.), ..SliderStyle::default() }
```

L’épaisseur par défaut est 4 unités logiques. La même option s’applique aux
orientations horizontale et verticale ; elle doit être finie, positive et
inférieure ou égale à 128. Les couleurs restent réglables avec `track`, `fill`,
`thumb` et `focus`.

## Deux moments distincts

La valeur fournie à `Slider::new` appartient à l’application. Pendant une
interaction, le runtime conserve une valeur provisoire, utilisée pour le dessin
et l’accessibilité même si la vue applicative n’a pas encore changé.

| Gestionnaire | Moment et rôle |
| --- | --- |
| `Slider::new(value, on_change)` | Nouvelle valeur provisoire différente de la précédente, après application du pas. |
| `.on_commit(on_commit)` | Valeur finale différente de la valeur de départ, à la fin d’une interaction valide. |

Les deux messages arrivent dans `Application::update` ; renvoyer `true`
demande de reconstruire la vue.

Pour un volume qui change immédiatement :

```rust
fn update(&mut self, message: Msg) -> bool {
    match message {
        Msg::Volume(value) => self.volume = value,
        Msg::VolumeCommitted(value) => self.save_volume(value),
        // ...
    }
    true
}
```

Pour une position de lecture, on peut ne changer la valeur applicative qu’après
validation :

```rust
Slider::new(self.position, Msg::PreviewPosition)
    .range(0., 231.)
    .on_commit(Msg::Seek)

// Dans update :
Msg::PreviewPosition(_) => return false, // le slider dessine son aperçu
Msg::Seek(value) => self.position = value,
```

Une démo peut aussi utiliser la valeur provisoire pour afficher un texte
d’aperçu, sans modifier la position validée. Elle ne contient aucun moteur audio.

Après la fin de l’interaction, le dessin reflète à nouveau la valeur de la vue.
L’application doit donc accepter la modification dans l’un de ces messages et
demander une reconstruction lorsqu’elle veut la conserver.

## Souris et clavier

| Interaction | Comportement |
| --- | --- |
| Cliquer la piste | Commencer une modification à cette position. |
| Attraper la poignée | Conserver le point de prise, sans saut initial. |
| Glisser | Mettre à jour l’aperçu ; la capture permet de continuer hors du rectangle. |
| Relâcher | Appliquer la dernière position bornée et valider une seule fois. |
| Flèche droite/haut | Graduation suivante. |
| Flèche gauche/bas | Graduation précédente. |
| Page Up/Down | Avancer/reculer de dix graduations, dans les limites de la plage. |
| Home / End | Minimum / maximum. |
| Relâcher les touches de réglage | Valider quand toutes les touches de réglage maintenues sont relâchées. |
| Échap, perte de focus natif, annulation ou Tab pendant une modification | Restaurer la valeur de départ, sans validation ; Tab poursuit ensuite la navigation. |

L’annulation peut envoyer le message `on_change` avec la valeur de départ pour
restaurer un modèle qui avait appliqué les changements en direct. Elle n’envoie
pas de message `on_commit`. Retirer, désactiver ou changer la configuration du
composant (plage, pas, axe) pendant l’interaction l’invalide et libère la
capture, sans écraser un nouvel état applicatif ni envoyer une validation
devenue obsolète. Les gestionnaires ne font pas partie de cette configuration :
la vue reconstruite après chaque valeur provisoire fournit de nouvelles
fonctions, et les messages suivants utilisent celles de l’arbre courant.

`.axis(Axis::Vertical)` place le minimum en bas et le maximum en haut. Un slider
en `.read_only()` reste focalisable et lisible, mais refuse les modifications.
`.disabled()` le retire des interactions et du parcours Tab.

## Plage, pas et style

La validation exige des bornes finies et strictement croissantes, une valeur
initiale finie comprise entre les bornes, et un pas positif, fini, représentable
et au plus égal à la taille de la plage. Une configuration invalide est rejetée
avant le rendu. Le pas est ancré sur le minimum ; les deux bornes restent
atteignables même si le pas ne divise pas exactement la plage.

`SliderStyle` propose les couleurs facultatives `track`, `fill`, `thumb` et
`focus`. `None` utilise le thème. La piste et la poignée ont des dimensions fixes
pour cette première version. Les graduations dessinées, l’échelle logarithmique,
le toucher et les sliders multiples ne sont pas encore proposés. Les contrôles
dans des lignes virtualisées demandent également un contrat de navigation dédié.

## Accessibilité et validation

Le backend Windows expose `RangeValuePattern` : valeur, minimum, maximum, petit
pas, grand pas et lecture seule. `SetValue` refuse NaN, les valeurs infinies et
les valeurs hors plage, puis arrondit au pas accepté. Cette commande représente
une modification complète : changement et validation, sans déplacer elle-même
le focus. Elle termine une éventuelle modification locale sur ce même slider.
Les propriétés et notifications proviennent du même état que le dessin. Voir
les contrats Microsoft de [RangeValue](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-implementingrangevalue)
et du [contrôle Slider](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportslidercontroltype).

Onze tests supplémentaires couvrent plage, pas fractionnaire, dernière graduation,
capture, annulation, aperçu différé, validation unique, clavier, lecture seule,
désactivation, transition clavier/souris et ABI COM. La suite complète compte
144 tests, avec GPU et WARP ; formatage et Clippy passent sans avertissement.

Vérifié à l’écran : découverte des deux réglages depuis UI Automation, réglage
du volume à 80 par `SetValue`, glissement à 40, position validée à 165 secondes,
flèche gauche vers 164 secondes, puis passage à WARP et au thème clair avec les
valeurs conservées. L’aperçu pendant un appui et les cas d’annulation sont aussi
vérifiés par les tests du runtime. La recette avec lecteur d’écran et les
annonces de focus restent ouvertes.

| Fichier | Rôle |
| --- | --- |
| `src/ui/components/slider/mod.rs` | Configuration, dessin et état provisoire des interactions. |
| `src/ui/runtime/mod.rs` | Focus, événements, envoi des messages et snapshot accessible. |
| `src/application/mod.rs` | Contrat `Application` et adaptateur des messages. |
| `src/platform/windows/accessibility/range_value/` | Interface COM et notifications Windows. |

```sh
cargo test --offline --lib
```



