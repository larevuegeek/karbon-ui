# Select : un choix dans un menu déroulant

`Select` est un contrôle non éditable dessiné par notre moteur. L’application
fournit ses options et la clé sélectionnée ; le runtime gère l’ouverture,
le parcours provisoire, le défilement et la fermeture.

## Construire le contrôle

Dans l’état de l’application, conserver par exemple :

```rust
selected_source: Option<u64>,
sources: Arc<[SelectOption]>,
```

Initialiser `sources` une fois avec les imports de `ui::widgets` :

```rust
let sources: Arc<[SelectOption]> = vec![
    SelectOption::new(1, "Cet ordinateur"),
    SelectOption::new(2, "Serveur NAS"),
    SelectOption::new(3, "Source indisponible").disabled(),
].into();
```

Dans `Application::view` :

```rust
Element::custom(
    162,
    Select::new(self.selected_source, Arc::clone(&self.sources), Msg::Source)
        .label("Bibliothèque active")
        .placeholder("Choisir une source…")
        .visible_items(6),
)
```

`Msg::Source` est une variante `Source(u64)` : le gestionnaire est une
fonction `Fn(u64) -> Msg` qui reçoit la clé choisie. Le nombre `162` identifie le widget ; les
clés `1`, `2`, `3` identifient ses options. Ces deux espaces sont indépendants.
Garder les clés stables après tri ou reconstruction, sans utiliser l’indice
d’une ligne comme identité si son contenu peut changer.

`label` est le nom accessible. Pour afficher un titre au-dessus du contrôle,
ajouter un widget `Text`.

## Conserver la sélection

```rust
fn update(&mut self, message: Msg) -> bool {
    match message {
        Msg::Source(option) => self.selected_source = Some(option),
        // ...
    }
    true
}
```

Le retour `true` reconstruit la vue avec la nouvelle sélection. Le survol et
les flèches changent seulement l’option active dans le menu. La valeur de
l’application change à la confirmation, si la clé diffère de l’ancienne.
Échap et le clic extérieur ne déclenchent aucun changement.

`None` affiche le placeholder. Si une clé est fournie, elle doit exister dans
les options. Une option désactivée peut rester la sélection affichée, mais
l’utilisateur ne peut pas la choisir. Une liste vide avec `None` est valide.
Les clés sont uniques dans chaque Select ; la validation refuse les doublons,
les clés sélectionnées absentes, plus de 512 options et `visible_items` hors
de l’intervalle 1–20. La valeur par défaut est 8 lignes visibles.

## Souris et clavier

| Interaction | Effet |
| --- | --- |
| Cliquer le contrôle, Entrée ou Espace | Ouvrir le menu. |
| Haut/Bas sur un contrôle fermé | Ouvrir sans modifier la sélection applicative. |
| Haut/Bas dans le menu | Parcourir les options actives, sans boucler aux extrémités. |
| Home / End | Atteindre la première / dernière option active. |
| Page Up/Down | Parcourir une page. |
| Lettres | Chercher un préfixe sans tenir compte de la casse ; une lettre répétée parcourt les correspondances. |
| Entrée / Espace dans le menu | Confirmer l’option active et fermer. |
| Clic puis relâchement sur la même option | Confirmer cette option. |
| Molette, piste et poignée de scroll | Parcourir les lignes du menu. |
| Échap | Fermer et conserver la sélection précédente. |
| Tab / Shift+Tab | Fermer, puis poursuivre le parcours des contrôles. |
| Clic extérieur | Fermer en absorbant ce clic pour ne pas activer un contrôle derrière. |

La recherche repart après une seconde sans saisie. Les options désactivées
sont ignorées par le clavier. Un événement souris à position inchangée ne
remplace pas le choix actif obtenu au clavier.

## Comment le menu passe devant la page

`src/ui/overlays/layers.rs` calcule un rectangle ancré au contrôle : en dessous si
l’espace le permet, au-dessus sinon, avec réduction à l’espace disponible.
Le runtime peint d’abord l’arbre, puis le menu. Il peut donc dépasser le clip
du panneau parent ; il reste limité à la zone disponible dans la fenêtre.
Ce menu ne crée aucune seconde fenêtre Windows.

Les événements du menu sont traités avant ceux des contrôles en dessous.
Un seul Select est ouvert à la fois. Un déplacement de son ancre, un scroll
extérieur, un resize, une perte de focus ou un changement de ses options ou
de sa valeur ferme le menu. Une reconstruction conservant les mêmes données
et la même géométrie peut le laisser ouvert.

Cette géométrie sert de première base aux couches ancrées. Les [modales](modal.md)
et les [popovers](popover.md) partagent maintenant une pile de focus ; les menus
en cascade restent à développer.

## Apparence, coût et limites

`.style(SelectStyle { ... })` permet de choisir `background`, `foreground`,
`border`, `focus` et `highlight`. Chaque champ vaut `Option<Color>` : `None`
reprend le thème. `.disabled()` désactive tout le contrôle. Dimensions et
espacements extérieurs se règlent sur l’`Element`.

Les options sont partagées avec `Arc`, sans reconstruire l’arbre à chaque
survol. Seules les lignes visibles du menu sont peintes ; la liste d’options
reste en mémoire. Cette première version utilise des lignes de 32 unités et
du texte de taille 14. Elle ne propose ni édition, ni multisélection, ni
contenu de ligne personnalisé. Pour des milliers d’éléments, utiliser une
vue de recherche avec `VirtualList` plutôt que ce Select limité à 512 choix.

Sous Windows, le contrôle expose `ExpandCollapsePattern` et `SelectionPattern`,
et ses options `SelectionItemPattern`. L’option sélectionnée reste lisible
quand le menu est fermé. Les identifiants accessibles combinent la clé du
contrôle et celle de l’option. Les essais avec un lecteur d’écran restent à
faire ; le client de recette conserve parfois une indication de focus sur
le premier champ malgré le déplacement réel du focus dans l’application.

## Essayer et retrouver le code

```sh
cargo test --offline --lib
```

Le panneau « Bibliothèque active » propose deux sources, une option désactivée
et vingt options de démonstration pour essayer le défilement.

| Fichier | Rôle |
| --- | --- |
| `src/ui/components/select/mod.rs` | API, dessin, état transitoire et interactions du Select. |
| `src/ui/overlays/layers.rs` | Placement du rectangle ancré. |
| `src/ui/runtime/mod.rs` | Intégration des événements, du focus, des messages et de la peinture finale. |
| `src/application/mod.rs` | Contrat `Application::update` et adaptateur des messages. |
| `src/platform/windows/accessibility/expand/` | Expansion et nœuds accessibles du menu. |

Les onze tests communs couvrent les couches, les clics, le clavier, les
annulations, le défilement, les reconstructions et la validation. Trois tests
natifs couvrent les interfaces COM, les identités, la sélection et la durée
de vie. La suite complète compte 144 tests, dont les essais GPU et WARP.

La démo a été vérifiée à l’écran : ouverture par clic et clavier, choix de
Serveur NAS depuis le client accessible, End puis Entrée sur la dernière
option, puis conservation du choix après passage à WARP et au thème clair.



