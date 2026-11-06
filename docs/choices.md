# Cases à cocher, switches et boutons radio

Ces composants sont dessinés par le moteur et partagent les événements, le focus,
le thème et le rendu CPU/GPU existants. Leurs valeurs appartiennent à l’application.
Ils ne créent pas de contrôles Windows enfants.

## Construire un choix

Dans `Application::view`, en important `Element` et les widgets :

```rust
Element::custom(
    100,
    Checkbox::new("Lecture aléatoire", self.shuffle, Msg::Shuffle),
)
```

`self.shuffle` décrit l’état à afficher. Le troisième argument est une fonction
`Fn(bool) -> Msg` : après une activation valide, elle reçoit la **nouvelle**
valeur et le message produit est envoyé à `Application::update`. Une variante
`Shuffle(bool)` suffit :

```rust
Msg::Shuffle(enabled) => self.shuffle = enabled,
```

Renvoyer `true` depuis `update` reconstruit la vue avec la nouvelle valeur. Le
composant ne modifie pas tout seul un état interne concurrent de celui de
l’application. Cela permet aussi de remettre une préférence à zéro depuis une
autre action. Conserver l’identifiant de l’élément garde son focus.

Le switch suit le même contrat :

```rust
Element::custom(
    101,
    Switch::new("Notifications", self.notifications, Msg::Notifications),
)
```

Dans un exemple, « Notifications » peut représenter un booléen applicatif ; il
n’active aucun service ni abonnement système.

## État intermédiaire et désactivation

Une case possède `CheckState::Off`, `On` ou `Mixed`. Le dernier représente par
exemple une sélection partielle :

```rust
Checkbox::new("Tout sélectionner", false, Msg::SelectAll)
    .state(CheckState::Mixed)
```

`CheckState::next()` passe de Off à On, de On à Off et de Mixed à On : une case
indéterminée transmet donc `true`. L’application peut choisir une autre
politique en calculant elle-même son état. Le switch
conserve deux états booléens.

`.disabled()` conserve le dessin et l’état accessible, tout en retirant le
contrôle des activations souris/clavier et du parcours Tab. Désactiver un contrôle
pendant un appui annule aussi son activation en attente.

## Choix exclusifs

Utiliser un groupe explicite et une valeur unique dans l’application :

```rust
widgets::radio_group(120, "Qualité", vec![
    Element::custom(121, RadioButton::new(
        "Originale", self.quality == Quality::Original, Msg::Quality(Quality::Original),
    )),
    Element::custom(122, RadioButton::new(
        "Économique", self.quality == Quality::Compressed, Msg::Quality(Quality::Compressed),
    )),
])
```

Un bouton radio reçoit un message (cloné à chaque choix), pas une fonction :
l’application affecte la qualité reçue dans `update`.

Un radio isolé et plusieurs radios sélectionnés dans un même groupe sont rejetés
par la validation. Un groupe initialement sans sélection est autorisé. Les
groupes imbriqués restent indépendants. Activer un radio déjà sélectionné
n’émet pas de nouveau message. Le moteur n’ajoute pas d’action permettant
de décocher le radio actif ; l’application peut réinitialiser le groupe.

`radio_group` construit une colonne avec un nom accessible. Ajouter un widget
`Text` séparé pour un titre visible. Padding, largeur, gap et surfaces se règlent
avec les méthodes habituelles d’`Element`.

## Souris et clavier

| Interaction | Comportement |
| --- | --- |
| Clic sur l’indicateur ou le libellé | Même activation. |
| Appui puis relâchement hors du contrôle | Activation annulée. |
| Espace | Appui visuel, activation au relâchement ; répétitions ignorées. |
| Entrée | Activation unique à l’appui. |
| Tab / Maj+Tab | Chaque case et switch est un arrêt ; un groupe radio compte pour un arrêt. |
| Entrée dans un groupe | Radio actif disponible, sinon premier disponible. |
| Flèches sur un radio | Choix précédent/suivant avec bouclage, en sautant les options désactivées. |
| Home / End sur un radio | Premier/dernier choix disponible. |

Si le focus a été placé directement sur un autre radio, Tab quitte ce groupe à
partir de ce contrôle. Les changements de focus révèlent les contrôles dans leurs
conteneurs défilants. Ces premiers groupes sont destinés aux formulaires de
l’arbre ordinaire ; les contrôles interactifs dans les lignes virtualisées
demandent un contrat de navigation supplémentaire.

## Personnaliser

```rust
Switch::new("Notifications", self.notifications, Msg::Notifications)
    .style(ChoiceStyle {
        accent: Some(Color::rgb(56, 132, 230)),
        ..ChoiceStyle::default()
    })
```

`ChoiceStyle` propose `accent`, `foreground`, `border`, `track` et `focus`.
`None` utilise le thème. Les dimensions et la taille de texte internes sont fixes
pour cette première version ; le layout extérieur reste personnalisable.

## Accessibilité et vérifications

Les cases et switches exposent `TogglePattern`, leur état et leurs changements.
Les radios exposent `SelectionItemPattern` et leur conteneur ; le groupe expose
`SelectionPattern`. La sélection est unique. Le groupe annonce une sélection
requise lorsqu’il en possède une ; l’ajout d’un deuxième choix ou le retrait
du choix actif sont refusés par les opérations accessibles correspondantes.
Les commandes accessibles produisent les mêmes messages applicatifs et ne
déplacent pas le focus. Voir le [guide d’accessibilité](accessibility.md).

Huit tests couvrent capture, annulation, clavier, désactivation, navigation radio,
validation, styles, commandes accessibles, ABI COM et libération des références.
La suite complète compte 144 tests. La démo a été vérifiée à l’écran : cases,
switch, changement exclusif, Tab/flèche gauche, thèmes sombre/clair et conservation
des valeurs après passage du GPU à WARP. Les essais avec Narrateur/NVDA restent
ouverts, notamment la vérification des annonces et du focus côté client.

Les widgets sont séparés dans `src/ui/components/choices/`, les interactions
sont intégrées à `src/ui/runtime/` et les interfaces Windows sont dans
`src/platform/windows/accessibility/choices/`. Les tests restent exécutables :

```sh
cargo test --offline --lib choice_tests
```
