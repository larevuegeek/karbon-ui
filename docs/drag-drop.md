# Glisser-déposer interne

Le socle expose des sources et des cibles indépendantes de Windows et de
RustMusic. L’application possède ses données et décide de leur modification
dans `Application::update`, à partir du message produit par la cible. Le moteur
transmet des clés stables, sans déplacer
lui-même les objets métier.

```rust,ignore
use karbon_ui::ui::drag_drop::{DragSource, DropTarget};

let row = widgets::button(42, "Un titre", Msg::SelectTrack(track_key))
    .draggable(DragSource::new(7, track_key))
    .drop_target(DropTarget::new(7, track_key, Msg::MoveTrack));
```

La source doit être un bouton ou un widget interactif ayant une action. La
cible peut être un conteneur de ligne. Le groupe distingue les opérations
compatibles ; les clés identifient les objets, indépendamment de leur position.
Une cible utilise l’axe vertical par défaut ; `.axis(Axis::Horizontal)` change
le partage avant/après.

Le dernier argument de `DropTarget::new` est une fonction
`Fn(DropEvent) -> Msg` ; une variante `MoveTrack(DropEvent)` suffit.

```rust,ignore
fn update(&mut self, message: Msg) -> bool {
    match message {
        Msg::MoveTrack(event) => {
            self.move_track(event.source, event.target, event.position)
        }
        // ...
    }
}
```

Un déplacement de six unités logiques déclenche le drag. En dessous de ce
seuil, le bouton garde son clic normal. Pendant le drag, le clic est supprimé,
la capture est conservée et une ligne indique la position d’insertion. Le
relâchement envoie le message de la cible une seule fois, pour une cible compatible de
clé différente de la source. La cible effective tient compte du clipping et
de la couche active, dont les modales.

Échap, perte de focus, perte de capture ou reconstruction de la vue annulent
l’opération. Un resize qui reconstruit la vue annule donc aussi le drag.
Le scroll près d’un bord avance sur les mouvements du pointeur ; il n’utilise
pas de timer et n’avance pas lorsque le pointeur reste immobile. Les mouvements
demandent une peinture, sans reconstruire la vue ni refaire le layout. Une
liste virtualisée peut matérialiser de nouvelles lignes lors de l’auto-scroll.

Pour l’accessibilité et le clavier, proposer aussi des commandes explicites de
déplacement, par exemple des boutons « Monter » et « Descendre » sur la
sélection, hors de la liste virtualisée pour rester accessibles par Tab.
Cette première API ne fournit ni drag clavier générique ni pattern UI Automation
de drag/drop. Elle ne prend pas en charge les fichiers déposés depuis l’OS,
les transferts entre fenêtres ou un aperçu flottant personnalisé.

Code : `src/ui/runtime/drag_drop/model.rs` (API), `src/ui/runtime/drag_drop/interaction.rs` (interaction),
`src/ui/tests/drag_drop.rs` (seuil, dépôt, annulation, groupes, désactivation,
reconstruction et auto-scroll). Les sources et cibles utilisent deux options
sur le tas dans `Element`, sans payload métier conservé par le runtime.

## Déplacement libre (`onpan`)

Pour un élément que l'utilisateur place où il veut (bouton flottant, poignée), un
bouton porte `onpan` (`Element::on_pan`) : au-delà de 4 unités logiques de mouvement,
l'appui devient un déplacement, le clic est supprimé et l'application reçoit un
`PanEvent` au début (`PanPhase::Start`), à chaque mouvement (`Move`) puis au
relâchement (`End`, aussi sur Échap, perte de focus ou de capture). L'événement donne la
position du pointeur dans la fenêtre (`x`, `y`) et le déplacement depuis l'appui (`dx`,
`dy`), en unités logiques, ainsi que la taille du parent du bouton à l'appui
(`parent_width`, `parent_height`) : une poignée placée au bord d'une colonne ou d'un
panneau part ainsi de sa largeur à l'écran, même quand elle est automatique.

```html
<button onclick={Msg::Toggle} onpan={|e: PanEvent| Msg::Moved(e)}>…</button>
```

Une reconstruction de la vue pendant le geste ne l'interrompt pas : le bouton garde la
capture tant qu'il existe. Sous le seuil, le bouton garde son clic normal.

**Bloc déplacé par le moteur (`data-pan-frame`, `Element::pan_frame`).** Sans lui,
l'application tient la position et la donne à l'élément à chaque mouvement (une
reconstruction par mouvement). Avec un ancêtre `data-pan-frame`, le moteur déplace
lui-même ce bloc pendant le geste en translatant son placement (comme un défilement) :
une peinture par mouvement, sans reconstruire ni redisposer, le bloc restant dans la
fenêtre (`dx`, `dy` sont bornés d'autant). L'application n'a rien à faire pendant le
geste (`Move` peut rendre `false`) ; au relâchement (`End`), elle donne la place finale,
par exemple collée au bord le plus proche, et le bloc y glisse depuis le point de lâcher
(0,22 s, départ rapide, arrivée douce).

```html
<div class="absolute" data-pan-frame style:top={Length::Px(top)} style:right={Length::Px(inset)}>
    <button onclick={Msg::Toggle} onpan={|e: PanEvent| Msg::Moved(e)}>…</button>
    …
</div>
```

Code : `src/ui/runtime/drag_drop/pan.rs` (geste, bloc, glissement final), tests
`src/ui/tests/pan.rs`.
