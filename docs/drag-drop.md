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
