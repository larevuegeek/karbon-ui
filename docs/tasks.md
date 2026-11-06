# Tâches en arrière-plan

Le service permet de lancer un travail sur un thread de fond puis de recevoir
ses données sur le thread UI, sans crate externe ni runtime async imposé.
Son code est commun aux plateformes ; le réveil de la fenêtre est actuellement
raccordé au seul backend Windows disponible. Sans fenêtre, les aperçus hors écran
et le harnais [headless](headless.md) (`HeadlessApp::run_tasks`) exécutent les
mêmes tâches jusqu’à leur fin, sur toutes les cibles.

## Essayer

```powershell
cargo test --offline --lib
```

Le panneau « Travail en arrière-plan » permet de charger, relancer et annuler
la génération de 20 000 titres factices. Un délai volontaire d’environ une
seconde rend l’interaction observable : ce n’est pas un scan disque ni une
mesure de performance. Le compteur, les champs et les autres fenêtres restent
utilisables. Relancer remplace la demande précédente.

L’exemple [`examples/background_task.rs`](../examples/background_task.rs) fait
une véritable lecture du dossier courant, sans le modifier :

```powershell
cargo run --offline --example background_task --target-dir target/tasks-check
```

## Trois opérations dans l’application

L’application conserve une file `Vec<TaskCommand>` puis la rend au moteur.
Une tâche possède un `TaskId` logique, local à sa fenêtre. Le travail reçoit
un jeton d’annulation et rend une valeur Rust possédée :

```rust,ignore
let directory = self.directory.clone();
self.tasks.push(TaskCommand::Spawn(Task::new(TaskId(1), move |cancel| {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        if cancel.is_cancelled() { break; }
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok::<_, std::io::Error>(names)
})));
```

La closure et son résultat doivent être `Send + 'static`. Elle capture ses
données par valeur ; elle ne reçoit ni référence à l’application, ni arbre UI,
ni contexte graphique natif. L’application elle-même n’a pas besoin d’être
`Send` et continue de vivre sur le thread UI.

Le callback suivant vide la file après les interactions :

```rust,ignore
fn take_task_commands(&mut self) -> Vec<TaskCommand> {
    std::mem::take(&mut self.tasks)
}
```

Le résultat revient dans `task_completed`, sur le thread UI :

```rust,ignore
fn task_completed(&mut self, result: TaskResult) -> bool {
    if result.id != TaskId(1) { return false; }
    self.status = match result.into_value::<std::io::Result<Vec<String>>>() {
        Ok(Ok(names)) => format!("{} entrées", names.len()),
        Ok(Err(error)) => format!("Lecture impossible : {error}"),
        Err(error) => format!("Travail impossible : {error}"),
    };
    true
}
```

`true` invalide la vue ; sa reconstruction attend le prochain dessin, ou une
préparation nécessaire à un nouvel événement. Plusieurs résultats peuvent
ainsi être regroupés sans remesurer la page à chaque callback. `false` conserve
la vue ; cela n’empêche pas de demander une autre tâche ou une commande de
fenêtre. Ce callback doit rester court : le calcul long appartient au travail
de fond, pas au traitement de son résultat.

Le type demandé avec `into_value::<T>()` doit être celui rendu par la tâche.
Un type incompatible produit `TaskError::TypeMismatch`. Un `Result<T, E>`
métier reste une valeur de tâche : il est distinct des erreurs du service.
`TaskError` se convertit en `karbon_ui::Error::Task`, si bien que `?` fonctionne
dans une fonction qui renvoie `karbon_ui::Result` :

```rust,ignore
fn names(result: TaskResult) -> karbon_ui::Result<Vec<String>> {
    // TaskError -> Error::Task, puis std::io::Error -> Error::Io.
    Ok(result.into_value::<std::io::Result<Vec<String>>>()??)
}
```
`TaskResult::success` et `TaskResult::failure` permettent aussi de tester un
callback ou de raccorder une boucle UI personnalisée.

## Annuler, relancer et fermer

| Commande | Comportement |
| --- | --- |
| `Spawn(Task::new(id, work))` | Annuler la version précédente du même ID et soumettre une nouvelle version. |
| `Cancel(id)` | Annuler cet ID et retirer tout résultat encore en attente. |
| `CancelAll` | Annuler les tâches de cette fenêtre, pas celles des autres. |

L’annulation est coopérative : `cancel.is_cancelled()` permet au travail de
s’arrêter. Elle ne termine pas de force un thread ni un appel disque/réseau
bloquant. Pour des opérations longues, vérifier le jeton entre les lots et
utiliser les possibilités de délai/annulation de la bibliothèque métier.

Une tâche annulée ne déclenche pas `task_completed`. L’application met donc
elle-même son état à jour quand elle demande l’annulation, comme dans la démo.
Même si le travail ignore le jeton et termine ensuite, son résultat est rejeté.
Une relance invalide aussi un résultat déjà calculé mais pas encore livré.
Un résultat livré avant la nouvelle demande ne peut pas être retiré rétroactivement.

La fermeture annule le scope de tâches de la fenêtre et libère ses résultats.
Une fenêtre rouverte avec le même `WindowId` reçoit un nouveau scope : elle
ne peut pas récupérer la réponse de l’ancienne. Des fenêtres différentes
peuvent utiliser le même `TaskId` sans partager leurs demandes.

## Fonctionnement et limites de ressources

Le premier travail crée un pool commun à la session. Il utilise entre un et
quatre threads selon le parallélisme disponible ; aucun worker n’est créé
tant qu’il n’y a pas de tâche. Les workers attendent sur une condition au
repos. La file globale contient au plus 64 travaux en attente, en plus des
travaux en cours. Une fenêtre conserve au plus 64 tâches actives, résultats
non livrés compris. Ces limites sont fixes dans cette première version.

Une saturation produit `TaskError::QueueFull` dans `task_completed`, sans
attendre qu’une place se libère sur le thread UI. Un démarrage de pool refusé
produit `StartFailed`. Une panique Rust déroulable dans le travail devient
`WorkerPanicked` ; le worker peut ensuite traiter une autre tâche. Le hook de
panique Rust peut toutefois écrire son message sur la sortie d’erreur.

Les travaux annulés encore en file sont retirés et leurs captures libérées.
Les ressources utilisateur sont libérées hors du verrou interne de la file.
Les données rendues par une tâche restent sous la responsabilité de
l’application : limiter le nombre de travaux ne limite pas la taille d’un
fichier ou d’un `Vec` rendu. Les caches d’images bornés restent à construire.

À la fermeture de la session, le moteur n’attend pas sur le thread UI la fin
d’un travail non coopératif. Les workers encore occupés restent détachés
jusqu’à leur fin. Relancer sans arrêt une tâche qui ignore l’annulation peut
donc occuper les workers et retarder les demandes suivantes ; le pool ne crée
pas de threads supplémentaires pour contourner ce problème.

## Adaptateur Windows

Les workers déposent les résultats dans une boîte de réception synchronisée,
avec l’ID et la version de la tâche. Ils ne touchent jamais l’application.
Un message `PostMessageW` adressé à la fenêtre réveille la boucle, y compris
lorsqu’elle attend un événement. Le message ne contient aucune adresse de
résultat : les données restent possédées par la boîte. Un ancien message de
réveil ne peut donc pas transporter la réponse d’une fenêtre détruite.

Le résultat est validé avant le callback. Les demandes d’annulation et de
relance invalident la version précédente dès la fin du callback applicatif,
avant un éventuel ancien réveil déjà en file. Entre deux résultats, le moteur
extrait les nouvelles commandes pour permettre au premier callback d’annuler
une autre réponse encore en attente. La boucle principale vérifie aussi les
boîtes : un message de réveil perdu ne supprime pas les données.

Voir le contrat natif de [PostMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew).

| Fichier | Rôle |
| --- | --- |
| `src/task/mod.rs` | Task, TaskId, commandes, jeton et résultat typé. |
| `src/task/executor/mod.rs` | Pool, file bornée, versions, annulation et boîtes. |
| `src/application/mod.rs` | Callbacks applicatifs facultatifs. |
| `src/ui/runtime/mod.rs` | Retour au modèle UI et invalidation. |
| `src/platform/windows/window/tasks.rs` | Scopes natifs, réveil et livraison. |

## Vérification et suite

La suite compte 168 tests : 166 pour la bibliothèque et deux pour la démo.
Cette étape ajoute neuf tests du service commun, trois tests natifs sur
fenêtres cachées, un test d’invalidation du runtime et un test de la démo.
Ils couvrent résultats typés, erreurs, saturation, annulation, relance,
libération des captures, retour sur le thread UI, fermeture/réouverture et
rejet d’un résultat déjà en file avant son dispatch Windows.

Formatage, Clippy et compilation de toutes les cibles sont vérifiés. La
recette visuelle du panneau, de la saisie pendant un chargement et du lecteur
d’écran reste à faire ; aucune fenêtre visible de recette n’a été ouverte.
Les données simulées ne permettent pas d’annoncer un gain face à Tauri.

Le [premier écran RustMusic](archive/rustmusic-demo.md) utilise maintenant ce service
pour le [chargement PNG/JPEG](image-loading.md), avec cache borné et premiers
[relevés hors écran](archive/performance.md). Le service n’intègre pas encore de
progression intermédiaire, de priorités configurables ni d’exécuteur
`Future`/async. Les backends macOS et Linux restent à implémenter.
