> Archive : cette page décrit les anciennes démonstrations retirées du projet. Les commandes et chemins de leur code sont historiques ; la bibliothèque actuelle conserve ses API et ses tests.

# File de lecture de la démo musicale

```powershell
cargo run --offline --release --bin rustmusic-demo --target-dir target/queue-verified -- --queue --profile --artwork-dir artifacts/artwork-fixtures-v1
```

Le bouton « File » ouvre ou ferme le panneau. À partir de 1 200 unités de
large, il est placé à droite ; sur une fenêtre plus étroite, la même composition
est présentée dans une modale fermable par Échap. Le lecteur reste fixe.

La file démarre avec douze titres fictifs. Cliquer une ligne la sélectionne ;
« Écouter » démarre sa lecture simulée. « Ajouter » ajoute une occurrence du
titre du lecteur, même s’il figure déjà dans la file. Les occurrences ont des
identifiants distincts. Les boutons du lecteur suivent l’ordre de la file et
ne bouclent pas à ses extrémités. L’activation d’un titre de la bibliothèque
le retrouve dans la file ou l’ajoute si une place est disponible.

Glisser le libellé d’une ligne la déplace avant ou après une autre. Les boutons
↑ et ↓ proposent la même réorganisation. Sous la liste, « Monter », « Descendre »
et « Retirer » agissent sur la sélection et sont accessibles par Tab ; les
boutons de chaque ligne restent utilisables à la souris et via UI Automation.
× retire cette occurrence ; vider la
file arrête la lecture. Retirer l’occurrence en cours choisit sa suivante, ou
la précédente si elle était la dernière. Réordonner une ligne ne change pas
le titre en cours. Aucune opération ne supprime de fichier sur disque.

La file utilise `VirtualList`, une hauteur fixe de 64 unités et une marge de
deux lignes. Elle accepte au maximum 10 000 occurrences. Haut/Bas et Home/End
parcourent la liste ; Entrée sélectionne le titre actif. Les boutons de
réorganisation et de retrait exposent leurs actions accessibles. Les données
de la file sont partagées avec les fabriques par Arc et copiées lors des
modifications si une vue les utilise encore.

Les contrôles étroits utilisent une recette de bouton avec un padding réduit.
La recette visuelle a révélé que le padding ordinaire laissait une largeur
nulle aux symboles : un test vérifie maintenant la place réservée à leur texte.

Les tests couvrent les clés stables, les doublons, les snapshots, les bornes,
la suppression du titre actif, la file vide, l’ordre suivant/précédent, les
formats compacts et larges et la navigation vers la ligne 10 000 avec moins de
24 lignes matérialisées dans le viewport testé. Le glisser-déposer générique
est décrit dans le [guide](../drag-drop.md).

`src/music/queue.rs` contient le modèle, les commandes et la composition ;
`src/music/view.rs` place le panneau et la modale. Il s’agit d’une démonstration
applicative : aucun moteur audio, persistance ou métadonnées réelles. Les
mesures de cette version sont consignées dans le [relevé de session](queue-validation.md).
